use bqip_autonomy::{
    AutonomousLearner, AutonomousLearningConfig, InferenceObservation, ValueFitnessWeights,
};
use bqip_core::{PhaseEnvelope, REGISTER_BYTES};
use bqip_training::{
    evaluate, ConceptTokenCompiler, ConceptTokenizerConfig, MetricsLedger, MetricsLedgerEvent,
    SplitRatios, SplitTrainedModel, TokenizedDocument, TrainConfig, TrainableScope, TrainingCorpus,
    TrainingError,
};
use bqip_transformer::{HybridConfig, HybridTransformer, MixerKind, TransformerError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationRole {
    Operator,
    Model,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
    pub timestamp_ms: u64,
}

impl ConversationMessage {
    pub fn new(
        role: ConversationRole,
        content: impl Into<String>,
        timestamp_ms: u64,
    ) -> Result<Self, ControlError> {
        let message = Self {
            role,
            content: content.into(),
            timestamp_ms,
        };
        message.validate()?;
        Ok(message)
    }

    pub fn validate(&self) -> Result<(), ControlError> {
        if self.content.trim().is_empty() {
            return Err(ControlError::EmptyMessage);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingControlConfig {
    pub model: HybridConfig,
    pub tokenizer: ConceptTokenizerConfig,
    pub train: TrainConfig,
    pub split_ratios: SplitRatios,
    pub node_public_key: [u8; REGISTER_BYTES],
    pub allow_model_self_training: bool,
}

impl TrainingControlConfig {
    pub fn validate(&self) -> Result<(), ControlError> {
        self.model.validate()?;
        self.tokenizer.validate()?;
        self.train.validate()?;
        self.split_ratios.validate()?;
        if self.model.vocab_size != self.tokenizer.vocab_size {
            return Err(ControlError::InvalidConfig(
                "model vocab_size must equal tokenizer vocab_size",
            ));
        }
        if self.model.max_context < 2 {
            return Err(ControlError::InvalidConfig(
                "model max_context must be at least 2",
            ));
        }
        Ok(())
    }
}

impl Default for TrainingControlConfig {
    fn default() -> Self {
        Self {
            model: HybridConfig {
                vocab_size: 512,
                d_model: 64,
                max_context: 64,
                mixer: MixerKind::GatedDeltaNet,
                num_heads: 4,
                phase_bucket_size: 32,
                structural_feedback: 0.04,
                ..HybridConfig::default()
            },
            tokenizer: ConceptTokenizerConfig {
                vocab_size: 512,
                max_tokens: 96,
                ngram_min: 1,
                ngram_max: 3,
                include_byte_tokens: true,
            },
            train: TrainConfig {
                epochs: 4,
                batch_size: 4,
                learning_rate: 0.08,
                max_grad_norm: 1.5,
                trainable_scope: TrainableScope::LmHead,
                twin_loss_weight: 0.05,
                phase_delta_increment: 0,
                twin_perturbation_scale: 0.02,
                graph_contrastive_weight: 0.1,
                ..TrainConfig::default()
            },
            split_ratios: SplitRatios {
                train: 0.8,
                validation: 0.2,
                test: 0.0,
            },
            node_public_key: [91u8; REGISTER_BYTES],
            allow_model_self_training: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConversationCommand {
    IngestText {
        concept_label: String,
        body: String,
    },
    AddInference {
        provider: String,
        model: String,
        concept_label: String,
        prompt: String,
        response: String,
        confidence: f32,
    },
    SetEpochs(usize),
    SetBatchSize(usize),
    SetLearningRate(f32),
    SetMaxGradNorm(f32),
    SetScope(TrainableScope),
    TrainNow,
    Evaluate,
    Status,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlAction {
    TextIngested {
        concept_label: String,
        token_count: usize,
    },
    InferenceIngested {
        concept_label: String,
        provider: String,
        model: String,
    },
    TrainConfigUpdated {
        field: &'static str,
        value: String,
    },
    TrainingCompleted {
        initial_loss: f32,
        final_loss: f32,
        ledger_records: usize,
    },
    EvaluationCompleted {
        mean_loss: f32,
        accuracy: f32,
    },
    StatusReported {
        documents: usize,
        inferences: usize,
        ledger_records: usize,
        epochs: usize,
        learning_rate: f32,
        trainable_scope: TrainableScope,
    },
}

#[derive(Clone, Debug)]
pub struct ControlTurn {
    pub message: ConversationMessage,
    pub commands: Vec<ConversationCommand>,
    pub actions: Vec<ControlAction>,
    pub reply: String,
    pub training_report: Option<SplitTrainedModel>,
}

pub struct NaturalLanguageTrainingController {
    config: TrainingControlConfig,
    compiler: ConceptTokenCompiler,
    learner: AutonomousLearner,
    model: HybridTransformer,
    documents: Vec<TokenizedDocument>,
    inferences: Vec<InferenceObservation>,
    ledger: MetricsLedger,
    turn_index: u64,
}

impl NaturalLanguageTrainingController {
    pub fn new(config: TrainingControlConfig) -> Result<Self, ControlError> {
        config.validate()?;
        let compiler = ConceptTokenCompiler::new(config.tokenizer.clone())?;
        let learner = AutonomousLearner::new(
            compiler.clone(),
            AutonomousLearningConfig {
                vocab_size: config.model.vocab_size,
                max_context: config.model.max_context,
                split_ratios: config.split_ratios,
                fitness_weights: ValueFitnessWeights::default(),
            },
        )?;
        let model = HybridTransformer::new(config.model.clone(), config.node_public_key)?;
        Ok(Self {
            config,
            compiler,
            learner,
            model,
            documents: Vec::new(),
            inferences: Vec::new(),
            ledger: MetricsLedger::new(),
            turn_index: 0,
        })
    }

    pub fn model(&self) -> &HybridTransformer {
        &self.model
    }

    pub fn ledger(&self) -> &MetricsLedger {
        &self.ledger
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn inference_count(&self) -> usize {
        self.inferences.len()
    }

    pub fn train_config(&self) -> &TrainConfig {
        &self.config.train
    }

    pub fn process(&mut self, message: ConversationMessage) -> Result<ControlTurn, ControlError> {
        message.validate()?;
        let commands = parse_commands(&message)?;
        if message.role == ConversationRole::Model
            && !self.config.allow_model_self_training
            && commands.iter().any(is_mutating_command)
        {
            return Err(ControlError::ModelSelfTrainingDisabled);
        }
        let mut actions = Vec::new();
        let mut training_report = None;
        for command in &commands {
            match command {
                ConversationCommand::IngestText {
                    concept_label,
                    body,
                } => {
                    let document = self
                        .compiler
                        .compile(concept_label.clone(), body.as_bytes())?;
                    let token_count = document.tokens.len();
                    self.documents.push(document);
                    actions.push(ControlAction::TextIngested {
                        concept_label: concept_label.clone(),
                        token_count,
                    });
                }
                ConversationCommand::AddInference {
                    provider,
                    model,
                    concept_label,
                    prompt,
                    response,
                    confidence,
                } => {
                    let observation = InferenceObservation::new(
                        provider.clone(),
                        model.clone(),
                        concept_label.clone(),
                        prompt.clone(),
                        response.clone(),
                        *confidence,
                        0,
                        0,
                        message.timestamp_ms,
                        self.envelope_for_message(&message),
                    )?;
                    self.inferences.push(observation);
                    actions.push(ControlAction::InferenceIngested {
                        concept_label: concept_label.clone(),
                        provider: provider.clone(),
                        model: model.clone(),
                    });
                }
                ConversationCommand::SetEpochs(epochs) => {
                    self.config.train.epochs = *epochs;
                    self.config.train.validate()?;
                    actions.push(ControlAction::TrainConfigUpdated {
                        field: "epochs",
                        value: epochs.to_string(),
                    });
                }
                ConversationCommand::SetBatchSize(batch_size) => {
                    self.config.train.batch_size = *batch_size;
                    self.config.train.validate()?;
                    actions.push(ControlAction::TrainConfigUpdated {
                        field: "batch_size",
                        value: batch_size.to_string(),
                    });
                }
                ConversationCommand::SetLearningRate(learning_rate) => {
                    self.config.train.learning_rate = *learning_rate;
                    self.config.train.validate()?;
                    actions.push(ControlAction::TrainConfigUpdated {
                        field: "learning_rate",
                        value: format!("{learning_rate:.8}"),
                    });
                }
                ConversationCommand::SetMaxGradNorm(max_grad_norm) => {
                    self.config.train.max_grad_norm = *max_grad_norm;
                    self.config.train.validate()?;
                    actions.push(ControlAction::TrainConfigUpdated {
                        field: "max_grad_norm",
                        value: format!("{max_grad_norm:.8}"),
                    });
                }
                ConversationCommand::SetScope(scope) => {
                    self.config.train.trainable_scope = *scope;
                    actions.push(ControlAction::TrainConfigUpdated {
                        field: "trainable_scope",
                        value: format!("{scope:?}"),
                    });
                }
                ConversationCommand::TrainNow => {
                    let trained = self.train_now()?;
                    actions.push(ControlAction::TrainingCompleted {
                        initial_loss: trained.report.train.initial_loss,
                        final_loss: trained.report.train.final_loss,
                        ledger_records: self.ledger.len(),
                    });
                    training_report = Some(trained);
                }
                ConversationCommand::Evaluate => {
                    let metrics = evaluate(&self.model, &self.current_corpus()?)?;
                    self.ledger.append(MetricsLedgerEvent::Evaluation {
                        split: bqip_training::EvalSplit::Train,
                        metrics: metrics.clone(),
                    })?;
                    actions.push(ControlAction::EvaluationCompleted {
                        mean_loss: metrics.mean_loss,
                        accuracy: metrics.accuracy,
                    });
                }
                ConversationCommand::Status => {
                    actions.push(ControlAction::StatusReported {
                        documents: self.documents.len(),
                        inferences: self.inferences.len(),
                        ledger_records: self.ledger.len(),
                        epochs: self.config.train.epochs,
                        learning_rate: self.config.train.learning_rate,
                        trainable_scope: self.config.train.trainable_scope,
                    });
                }
            }
        }
        if commands.is_empty() {
            actions.push(ControlAction::StatusReported {
                documents: self.documents.len(),
                inferences: self.inferences.len(),
                ledger_records: self.ledger.len(),
                epochs: self.config.train.epochs,
                learning_rate: self.config.train.learning_rate,
                trainable_scope: self.config.train.trainable_scope,
            });
        }
        let reply = build_reply(&actions);
        self.turn_index = self.turn_index.saturating_add(1);
        Ok(ControlTurn {
            message,
            commands,
            actions,
            reply,
            training_report,
        })
    }

    fn train_now(&mut self) -> Result<SplitTrainedModel, ControlError> {
        let corpus = self.current_corpus()?;
        let trained = self.learner.flash_learn(
            self.model.clone(),
            &corpus,
            self.config.train.clone(),
            &mut self.ledger,
            &self.turn_index.to_le_bytes(),
        )?;
        self.model = trained.model.clone();
        Ok(trained)
    }

    fn current_corpus(&self) -> Result<TrainingCorpus, ControlError> {
        let mut documents = self.documents.clone();
        documents.extend(
            self.learner
                .inference_documents(&self.inferences)
                .unwrap_or_default(),
        );
        if documents.is_empty() {
            return Err(ControlError::NoTrainingData);
        }
        Ok(TrainingCorpus::from_tokenized_documents(
            self.config.model.vocab_size,
            self.config.model.max_context,
            &documents,
        )?)
    }

    fn envelope_for_message(&self, message: &ConversationMessage) -> PhaseEnvelope {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bqip-control-message-envelope");
        hasher.update(&(self.turn_index).to_le_bytes());
        hasher.update(&message.timestamp_ms.to_le_bytes());
        hasher.update(message.content.as_bytes());
        let hash = hasher.finalize();
        let mut sig = [0u8; 8];
        sig.copy_from_slice(&hash.as_bytes()[..8]);
        PhaseEnvelope::balanced(u64::from_le_bytes(sig))
    }
}

fn parse_commands(message: &ConversationMessage) -> Result<Vec<ConversationCommand>, ControlError> {
    let content = message.content.trim();
    let lower = content.to_ascii_lowercase();
    let mut commands = Vec::new();

    if let Some((label, body)) = parse_learn_command(content) {
        commands.push(ConversationCommand::IngestText {
            concept_label: label,
            body,
        });
    }
    if let Some(inference) = parse_inference_command(content)? {
        commands.push(inference);
    }
    if let Some(epochs) = parse_usize_after(&lower, &["epochs", "epoch"]) {
        commands.push(ConversationCommand::SetEpochs(epochs));
    }
    if let Some(batch_size) = parse_usize_after(&lower, &["batch size", "batch"]) {
        commands.push(ConversationCommand::SetBatchSize(batch_size));
    }
    if let Some(learning_rate) = parse_f32_after(&lower, &["learning rate", "lr"]) {
        commands.push(ConversationCommand::SetLearningRate(learning_rate));
    }
    if let Some(max_grad_norm) = parse_f32_after(&lower, &["max grad norm", "grad norm"]) {
        commands.push(ConversationCommand::SetMaxGradNorm(max_grad_norm));
    }
    if lower.contains("dense finite") || lower.contains("scope dense") {
        commands.push(ConversationCommand::SetScope(
            TrainableScope::DenseFiniteDifference,
        ));
    } else if lower.contains("lm head") || lower.contains("scope lm") {
        commands.push(ConversationCommand::SetScope(TrainableScope::LmHead));
    }
    if lower.contains("train now")
        || lower.contains("start training")
        || lower.contains("flash learn")
        || lower.contains("self train")
        || lower == "train"
    {
        commands.push(ConversationCommand::TrainNow);
    }
    if lower.contains("evaluate") || lower.contains("eval") {
        commands.push(ConversationCommand::Evaluate);
    }
    if lower.contains("status") || lower.contains("where are we") {
        commands.push(ConversationCommand::Status);
    }

    Ok(commands)
}

fn parse_learn_command(content: &str) -> Option<(String, String)> {
    for marker in ["learn concept ", "learn:", "remember:", "ingest:"] {
        if let Some(index) = content.to_ascii_lowercase().find(marker) {
            let payload = content[index + marker.len()..].trim();
            if marker == "learn concept " {
                let (label, body) = payload.split_once(':')?;
                let label = label.trim();
                let body = body.trim();
                if !label.is_empty() && !body.is_empty() {
                    return Some((label.to_string(), body.to_string()));
                }
            } else {
                let body = payload.trim();
                if !body.is_empty() {
                    return Some(("conversation memory".to_string(), body.to_string()));
                }
            }
        }
    }
    None
}

fn parse_inference_command(content: &str) -> Result<Option<ConversationCommand>, ControlError> {
    let lower = content.to_ascii_lowercase();
    if !lower.contains("inference") {
        return Ok(None);
    }
    let provider = parse_field(content, "provider").unwrap_or_else(|| "conversation".to_string());
    let model = parse_field(content, "model").unwrap_or_else(|| "external-model".to_string());
    let concept_label = parse_field(content, "concept").unwrap_or_else(|| "inference".to_string());
    let prompt =
        parse_field(content, "prompt").unwrap_or_else(|| "conversation prompt".to_string());
    let response = parse_field(content, "response")
        .or_else(|| {
            content
                .split_once(':')
                .map(|(_, body)| body.trim().to_string())
        })
        .ok_or(ControlError::InvalidInferenceCommand)?;
    let confidence = parse_field(content, "confidence")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.7);
    Ok(Some(ConversationCommand::AddInference {
        provider,
        model,
        concept_label,
        prompt,
        response,
        confidence,
    }))
}

fn parse_field(content: &str, key: &str) -> Option<String> {
    let lower = content.to_ascii_lowercase();
    let marker = format!("{key}=");
    let start = lower.find(&marker)? + marker.len();
    let rest = &content[start..];
    let end = rest
        .find(';')
        .or_else(|| rest.find('\n'))
        .unwrap_or(rest.len());
    let value = rest[..end].trim().trim_matches('"');
    (!value.is_empty()).then(|| value.to_string())
}

fn parse_usize_after(content: &str, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|key| parse_number_after(content, key).and_then(|value| value.parse().ok()))
}

fn parse_f32_after(content: &str, keys: &[&str]) -> Option<f32> {
    keys.iter()
        .find_map(|key| parse_number_after(content, key).and_then(|value| value.parse().ok()))
}

fn parse_number_after<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let start = content.find(key)? + key.len();
    let tail = content[start..].trim_start_matches(|character: char| {
        character.is_whitespace() || character == '=' || character == ':' || character == ','
    });
    let end = tail
        .find(|character: char| {
            !(character.is_ascii_digit()
                || character == '.'
                || character == 'e'
                || character == '-'
                || character == '+')
        })
        .unwrap_or(tail.len());
    (end > 0).then_some(&tail[..end])
}

fn is_mutating_command(command: &ConversationCommand) -> bool {
    matches!(
        command,
        ConversationCommand::IngestText { .. }
            | ConversationCommand::AddInference { .. }
            | ConversationCommand::SetEpochs(_)
            | ConversationCommand::SetBatchSize(_)
            | ConversationCommand::SetLearningRate(_)
            | ConversationCommand::SetMaxGradNorm(_)
            | ConversationCommand::SetScope(_)
            | ConversationCommand::TrainNow
    )
}

fn build_reply(actions: &[ControlAction]) -> String {
    if actions.is_empty() {
        return "No training command executed.".to_string();
    }
    let mut parts = Vec::new();
    for action in actions {
        match action {
            ControlAction::TextIngested {
                concept_label,
                token_count,
            } => parts.push(format!("ingested `{concept_label}` as {token_count} tokens")),
            ControlAction::InferenceIngested {
                concept_label,
                provider,
                model,
            } => parts.push(format!(
                "ingested inference `{concept_label}` from {provider}/{model}"
            )),
            ControlAction::TrainConfigUpdated { field, value } => {
                parts.push(format!("set {field}={value}"))
            }
            ControlAction::TrainingCompleted {
                initial_loss,
                final_loss,
                ledger_records,
            } => parts.push(format!(
                "trained loss {initial_loss:.6}->{final_loss:.6}, ledger_records={ledger_records}"
            )),
            ControlAction::EvaluationCompleted {
                mean_loss,
                accuracy,
            } => parts.push(format!("evaluated loss={mean_loss:.6}, accuracy={accuracy:.4}")),
            ControlAction::StatusReported {
                documents,
                inferences,
                ledger_records,
                epochs,
                learning_rate,
                trainable_scope,
            } => parts.push(format!(
                "status documents={documents}, inferences={inferences}, ledger_records={ledger_records}, epochs={epochs}, lr={learning_rate:.6}, scope={trainable_scope:?}"
            )),
        }
    }
    parts.join("; ")
}

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("conversation message must be nonempty")]
    EmptyMessage,
    #[error("invalid control config: {0}")]
    InvalidConfig(&'static str),
    #[error("model-authored self-training is disabled")]
    ModelSelfTrainingDisabled,
    #[error("no training data is available")]
    NoTrainingData,
    #[error("invalid inference command")]
    InvalidInferenceCommand,
    #[error(transparent)]
    Training(#[from] TrainingError),
    #[error(transparent)]
    Transformer(#[from] TransformerError),
    #[error(transparent)]
    Autonomy(#[from] bqip_autonomy::AutonomyError),
}
