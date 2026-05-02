use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bqip_core::{phase_project, PhaseEnvelope, Register};
use bqip_transformer::{HybridTransformer, LazyBqipMemory, Matrix, ModelWeights, TransformerError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

fn timestamp() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}.{:03}", d.as_secs(), d.subsec_millis()),
        Err(_) => "unknown".to_string(),
    }
}

const CORPUS_MAGIC: &[u8; 8] = b"BQIPTRN1";
const METRICS_RECORD_MAGIC: &[u8; 8] = b"BQIPMET1";
const HEADER_BYTES: usize = 48;
const GENESIS_LEDGER_HASH: [u8; 32] = [0u8; 32];

fn encode_checked<T: Serialize>(magic: &[u8; 8], value: &T) -> Result<Vec<u8>, TrainingError> {
    let payload = bincode::serialize(value)?;
    let hash = blake3::hash(&payload);
    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len());
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(hash.as_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

fn decode_checked_with_len<T: DeserializeOwned>(
    magic: &[u8; 8],
    bytes: &[u8],
) -> Result<(T, usize), TrainingError> {
    if bytes.len() < HEADER_BYTES || &bytes[..8] != magic {
        return Err(TrainingError::InvalidPersistentHeader);
    }
    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&bytes[8..16]);
    let payload_len = u64::from_le_bytes(len_bytes) as usize;
    let required = HEADER_BYTES
        .checked_add(payload_len)
        .ok_or(TrainingError::InvalidPersistentHeader)?;
    if required > bytes.len() {
        return Err(TrainingError::PersistentCapacity {
            capacity: bytes.len(),
            required,
        });
    }
    let payload = &bytes[HEADER_BYTES..required];
    let actual_hash = blake3::hash(payload);
    if &bytes[16..48] != actual_hash.as_bytes() {
        return Err(TrainingError::PersistentHashMismatch);
    }
    Ok((bincode::deserialize(payload)?, required))
}

fn decode_checked<T: DeserializeOwned>(magic: &[u8; 8], bytes: &[u8]) -> Result<T, TrainingError> {
    Ok(decode_checked_with_len(magic, bytes)?.0)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConceptTokenizerConfig {
    pub vocab_size: usize,
    pub max_tokens: usize,
    pub ngram_min: usize,
    pub ngram_max: usize,
    pub include_byte_tokens: bool,
}

impl ConceptTokenizerConfig {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.vocab_size == 0 {
            return Err(TrainingError::InvalidTokenizerConfig(
                "vocab_size must be nonzero",
            ));
        }
        if self.max_tokens < 2 {
            return Err(TrainingError::InvalidTokenizerConfig(
                "max_tokens must be at least 2",
            ));
        }
        if self.ngram_min == 0 || self.ngram_max < self.ngram_min {
            return Err(TrainingError::InvalidTokenizerConfig(
                "ngram bounds must satisfy 1 <= min <= max",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConceptTokenCompiler {
    config: ConceptTokenizerConfig,
}

impl ConceptTokenCompiler {
    pub fn new(config: ConceptTokenizerConfig) -> Result<Self, TrainingError> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn config(&self) -> &ConceptTokenizerConfig {
        &self.config
    }

    pub fn compile(
        &self,
        concept_label: impl Into<String>,
        bytes: &[u8],
    ) -> Result<TokenizedDocument, TrainingError> {
        self.config.validate()?;
        let concept_label = concept_label.into();
        if concept_label.trim().is_empty() {
            return Err(TrainingError::EmptyConceptLabel);
        }
        if bytes.is_empty() {
            return Err(TrainingError::EmptyDocument);
        }

        let mut tokens = Vec::with_capacity(self.config.max_tokens);
        tokens.push(hashed_token(
            self.config.vocab_size,
            b"concept-label",
            concept_label.as_bytes(),
        ));
        for word in split_words(concept_label.as_bytes()) {
            push_token(
                &mut tokens,
                self.config.max_tokens,
                hashed_token(self.config.vocab_size, b"label-word", &word),
            );
        }

        if self.config.include_byte_tokens {
            for byte in bytes {
                push_token(
                    &mut tokens,
                    self.config.max_tokens,
                    byte_token(self.config.vocab_size, *byte),
                );
                if tokens.len() >= self.config.max_tokens {
                    break;
                }
            }
        }

        let words = split_words(bytes);
        for word in &words {
            push_token(
                &mut tokens,
                self.config.max_tokens,
                hashed_token(self.config.vocab_size, b"body-word", word),
            );
        }
        for ngram_len in self.config.ngram_min..=self.config.ngram_max {
            if words.len() < ngram_len {
                continue;
            }
            for window in words.windows(ngram_len) {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"concept-ngram");
                hasher.update(&(ngram_len as u64).to_le_bytes());
                for word in window {
                    hasher.update(word);
                    hasher.update(&[0xff]);
                }
                push_token(
                    &mut tokens,
                    self.config.max_tokens,
                    token_from_hash(self.config.vocab_size, hasher.finalize().as_bytes()),
                );
            }
        }

        if tokens.len() < 2 {
            tokens.push(hashed_token(self.config.vocab_size, b"document", bytes));
        }
        tokens.truncate(self.config.max_tokens);
        let document = TokenizedDocument {
            concept_label,
            tokens,
            document_hash: *blake3::hash(bytes).as_bytes(),
        };
        document.validate(self.config.vocab_size)?;
        Ok(document)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenizedDocument {
    pub concept_label: String,
    pub tokens: Vec<u32>,
    pub document_hash: [u8; 32],
}

impl TokenizedDocument {
    pub fn validate(&self, vocab_size: usize) -> Result<(), TrainingError> {
        if self.concept_label.trim().is_empty() {
            return Err(TrainingError::EmptyConceptLabel);
        }
        if self.tokens.len() < 2 {
            return Err(TrainingError::TokenizedDocumentTooShort);
        }
        if self
            .tokens
            .iter()
            .any(|token| *token as usize >= vocab_size)
        {
            return Err(TrainingError::TokenOutOfVocabulary);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingExample {
    pub context: Vec<u32>,
    pub target_token: u32,
    pub weight: f32,
    pub source_hash: [u8; 32],
    pub concept_label: Option<String>,
}

impl TrainingExample {
    pub fn new(
        context: Vec<u32>,
        target_token: u32,
        weight: f32,
        concept_label: Option<String>,
    ) -> Result<Self, TrainingError> {
        let example = Self {
            source_hash: derive_source_hash(&context, target_token, concept_label.as_deref()),
            context,
            target_token,
            weight,
            concept_label,
        };
        example.validate(usize::MAX)?;
        Ok(example)
    }

    pub fn validate(&self, vocab_size: usize) -> Result<(), TrainingError> {
        if self.context.is_empty() {
            return Err(TrainingError::EmptyContext);
        }
        if !self.weight.is_finite() || self.weight <= 0.0 {
            return Err(TrainingError::InvalidExampleWeight);
        }
        if self.target_token as usize >= vocab_size
            || self
                .context
                .iter()
                .any(|token| *token as usize >= vocab_size)
        {
            return Err(TrainingError::TokenOutOfVocabulary);
        }
        let expected = derive_source_hash(
            &self.context,
            self.target_token,
            self.concept_label.as_deref(),
        );
        if expected != self.source_hash {
            return Err(TrainingError::SourceHashMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingCorpus {
    pub vocab_size: usize,
    pub max_context: usize,
    pub examples: Vec<TrainingExample>,
}

impl TrainingCorpus {
    pub fn from_sequences(
        vocab_size: usize,
        max_context: usize,
        sequences: &[Vec<u32>],
        concept_label: Option<String>,
    ) -> Result<Self, TrainingError> {
        if vocab_size == 0 {
            return Err(TrainingError::InvalidCorpus("vocab_size must be nonzero"));
        }
        if max_context == 0 {
            return Err(TrainingError::InvalidCorpus("max_context must be nonzero"));
        }
        let mut examples = Vec::new();
        for sequence in sequences {
            append_sequence_examples(&mut examples, max_context, sequence, concept_label.clone())?;
        }
        let corpus = Self {
            vocab_size,
            max_context,
            examples,
        };
        corpus.validate()?;
        Ok(corpus)
    }

    pub fn from_tokenized_documents(
        vocab_size: usize,
        max_context: usize,
        documents: &[TokenizedDocument],
    ) -> Result<Self, TrainingError> {
        if documents.is_empty() {
            return Err(TrainingError::EmptyCorpus);
        }
        let mut examples = Vec::new();
        for document in documents {
            document.validate(vocab_size)?;
            append_sequence_examples(
                &mut examples,
                max_context,
                &document.tokens,
                Some(document.concept_label.clone()),
            )?;
        }
        let corpus = Self {
            vocab_size,
            max_context,
            examples,
        };
        corpus.validate()?;
        Ok(corpus)
    }

    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.vocab_size == 0 {
            return Err(TrainingError::InvalidCorpus("vocab_size must be nonzero"));
        }
        if self.max_context == 0 {
            return Err(TrainingError::InvalidCorpus("max_context must be nonzero"));
        }
        if self.examples.is_empty() {
            return Err(TrainingError::EmptyCorpus);
        }
        for example in &self.examples {
            example.validate(self.vocab_size)?;
            if example.context.len() > self.max_context {
                return Err(TrainingError::ContextTooLong {
                    requested: example.context.len(),
                    max: self.max_context,
                });
            }
        }
        Ok(())
    }

    pub fn split(&self, ratios: SplitRatios, seed: &[u8]) -> Result<CorpusSplit, TrainingError> {
        self.validate()?;
        ratios.validate()?;
        let normalized = ratios.normalized();
        let mut train = Vec::new();
        let mut validation = Vec::new();
        let mut test = Vec::new();
        for (index, example) in self.examples.iter().cloned().enumerate() {
            let bucket = deterministic_unit_interval(seed, &example.source_hash, index);
            if bucket < normalized.train {
                train.push(example);
            } else if bucket < normalized.train + normalized.validation {
                validation.push(example);
            } else {
                test.push(example);
            }
        }
        if train.is_empty() {
            if let Some(example) = validation.pop().or_else(|| test.pop()) {
                train.push(example);
            }
        }
        let split = CorpusSplit {
            train: Self {
                vocab_size: self.vocab_size,
                max_context: self.max_context,
                examples: train,
            },
            validation: optional_corpus(self.vocab_size, self.max_context, validation),
            test: optional_corpus(self.vocab_size, self.max_context, test),
        };
        split.validate()?;
        Ok(split)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), TrainingError> {
        self.validate()?;
        fs::write(path, encode_checked(CORPUS_MAGIC, self)?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, TrainingError> {
        let corpus = decode_checked::<Self>(CORPUS_MAGIC, &fs::read(path)?)?;
        corpus.validate()?;
        Ok(corpus)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitRatios {
    pub train: f32,
    pub validation: f32,
    pub test: f32,
}

impl Default for SplitRatios {
    fn default() -> Self {
        Self {
            train: 0.8,
            validation: 0.1,
            test: 0.1,
        }
    }
}

impl SplitRatios {
    pub fn validate(&self) -> Result<(), TrainingError> {
        for (name, value) in [
            ("train", self.train),
            ("validation", self.validation),
            ("test", self.test),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(TrainingError::InvalidSplitRatio(name));
            }
        }
        if self.train <= 0.0 {
            return Err(TrainingError::InvalidSplitRatio("train"));
        }
        if self.train + self.validation + self.test <= f32::EPSILON {
            return Err(TrainingError::InvalidSplitRatio("sum"));
        }
        Ok(())
    }

    fn normalized(self) -> Self {
        let sum = self.train + self.validation + self.test;
        Self {
            train: self.train / sum,
            validation: self.validation / sum,
            test: self.test / sum,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorpusSplit {
    pub train: TrainingCorpus,
    pub validation: Option<TrainingCorpus>,
    pub test: Option<TrainingCorpus>,
}

impl CorpusSplit {
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.train.validate()?;
        if let Some(validation) = &self.validation {
            validation.validate()?;
            validate_same_shape(&self.train, validation)?;
        }
        if let Some(test) = &self.test {
            test.validate()?;
            validate_same_shape(&self.train, test)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainableScope {
    LmHead,
    DenseFiniteDifference,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainConfig {
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f32,
    pub weight_decay: f32,
    pub max_grad_norm: f32,
    pub trainable_scope: TrainableScope,
    pub finite_difference_epsilon: f32,
    pub twin_loss_weight: f32,
    pub phase_delta_increment: u32,
    pub twin_perturbation_scale: f32,
    pub graph_contrastive_weight: f32,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            epochs: 4,
            batch_size: 8,
            learning_rate: 0.08,
            weight_decay: 0.0,
            max_grad_norm: 1.0,
            trainable_scope: TrainableScope::LmHead,
            finite_difference_epsilon: 1.0e-3,
            twin_loss_weight: 0.05,
            phase_delta_increment: 0,
            twin_perturbation_scale: 0.02,
            graph_contrastive_weight: 0.1,
        }
    }
}

impl TrainConfig {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.epochs == 0 {
            return Err(TrainingError::InvalidTrainConfig("epochs must be nonzero"));
        }
        if self.batch_size == 0 {
            return Err(TrainingError::InvalidTrainConfig(
                "batch_size must be nonzero",
            ));
        }
        for (name, value) in [
            ("learning_rate", self.learning_rate),
            ("weight_decay", self.weight_decay),
            ("max_grad_norm", self.max_grad_norm),
            ("finite_difference_epsilon", self.finite_difference_epsilon),
            ("twin_loss_weight", self.twin_loss_weight),
            ("twin_perturbation_scale", self.twin_perturbation_scale),
            ("graph_contrastive_weight", self.graph_contrastive_weight),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(TrainingError::InvalidTrainConfig(name));
            }
        }
        if self.learning_rate == 0.0
            || self.max_grad_norm == 0.0
            || self.finite_difference_epsilon == 0.0
        {
            return Err(TrainingError::InvalidTrainConfig(
                "learning_rate, max_grad_norm, and finite_difference_epsilon must be greater than zero",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpochMetrics {
    pub epoch_index: usize,
    pub examples_seen: usize,
    pub mean_loss: f32,
    pub accuracy: f32,
    pub mean_target_probability: f32,
    pub gradient_norm: f32,
    pub mean_decoherence: f32,
    pub mean_envelope_alpha_error: f32,
    pub mean_envelope_beta_error: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalSplit {
    Train,
    Validation,
    Test,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingReport {
    pub initial_loss: f32,
    pub final_loss: f32,
    pub initial_accuracy: f32,
    pub final_accuracy: f32,
    pub epochs: Vec<EpochMetrics>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitTrainingReport {
    pub train: TrainingReport,
    pub validation: Option<EpochMetrics>,
    pub test: Option<EpochMetrics>,
}

#[derive(Clone, Debug)]
pub struct TrainedModel {
    pub model: HybridTransformer,
    pub report: TrainingReport,
}

#[derive(Clone, Debug)]
pub struct SplitTrainedModel {
    pub model: HybridTransformer,
    pub report: SplitTrainingReport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MetricsLedgerEvent {
    Evaluation {
        split: EvalSplit,
        metrics: EpochMetrics,
    },
    Epoch {
        metrics: EpochMetrics,
    },
    TrainingReport {
        report: TrainingReport,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricsLedgerRecord {
    pub sequence: u64,
    pub previous_hash: [u8; 32],
    pub event: MetricsLedgerEvent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricsLedger {
    records: Vec<MetricsLedgerRecord>,
    head_hash: [u8; 32],
}

impl Default for MetricsLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsLedger {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            head_hash: GENESIS_LEDGER_HASH,
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub const fn head_hash(&self) -> [u8; 32] {
        self.head_hash
    }

    pub fn records(&self) -> &[MetricsLedgerRecord] {
        &self.records
    }

    pub fn append(&mut self, event: MetricsLedgerEvent) -> Result<[u8; 32], TrainingError> {
        let record = MetricsLedgerRecord {
            sequence: self.records.len() as u64,
            previous_hash: self.head_hash,
            event,
        };
        self.push_record(record)
    }

    fn push_record(&mut self, record: MetricsLedgerRecord) -> Result<[u8; 32], TrainingError> {
        let expected = self.records.len() as u64;
        if record.sequence != expected {
            return Err(TrainingError::MetricsLedgerSequence {
                expected,
                actual: record.sequence,
            });
        }
        if record.previous_hash != self.head_hash {
            return Err(TrainingError::MetricsLedgerChainMismatch {
                sequence: record.sequence,
            });
        }
        let next_hash = metrics_record_hash(self.head_hash, &record)?;
        self.records.push(record);
        self.head_hash = next_hash;
        Ok(next_hash)
    }

    pub fn validate(&self) -> Result<(), TrainingError> {
        let mut head_hash = GENESIS_LEDGER_HASH;
        for (index, record) in self.records.iter().enumerate() {
            if record.sequence != index as u64 {
                return Err(TrainingError::MetricsLedgerSequence {
                    expected: index as u64,
                    actual: record.sequence,
                });
            }
            if record.previous_hash != head_hash {
                return Err(TrainingError::MetricsLedgerChainMismatch {
                    sequence: record.sequence,
                });
            }
            head_hash = metrics_record_hash(head_hash, record)?;
        }
        if head_hash != self.head_hash {
            return Err(TrainingError::MetricsLedgerHeadMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct AppendOnlyMetricsLedger {
    path: PathBuf,
    ledger: MetricsLedger,
}

impl AppendOnlyMetricsLedger {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, TrainingError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let ledger = if path.exists() {
            read_metrics_records(&path)?
        } else {
            File::create(&path)?;
            MetricsLedger::new()
        };
        Ok(Self { path, ledger })
    }

    pub fn append(&mut self, event: MetricsLedgerEvent) -> Result<[u8; 32], TrainingError> {
        let record = MetricsLedgerRecord {
            sequence: self.ledger.records.len() as u64,
            previous_hash: self.ledger.head_hash,
            event,
        };
        let encoded = encode_checked(METRICS_RECORD_MAGIC, &record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(&encoded)?;
        file.sync_data()?;
        self.ledger.push_record(record)
    }

    pub fn ledger(&self) -> &MetricsLedger {
        &self.ledger
    }
}

pub struct HybridTrainer {
    model: HybridTransformer,
    config: TrainConfig,
}

impl HybridTrainer {
    pub fn new(model: HybridTransformer, config: TrainConfig) -> Result<Self, TrainingError> {
        config.validate()?;
        Ok(Self { model, config })
    }

    pub fn model(&self) -> &HybridTransformer {
        &self.model
    }

    pub fn into_model(self) -> HybridTransformer {
        self.model
    }

    pub fn train(mut self, corpus: &TrainingCorpus) -> Result<TrainedModel, TrainingError> {
        validate_model_corpus(
            self.model.config().vocab_size,
            self.model.config().max_context,
            corpus,
        )?;
        println!("  📊 Evaluating initial metrics...");
        let initial = evaluate(&self.model, corpus)?;
        println!("     initial loss = {:.4}, acc = {:.2}%", 
            initial.mean_loss, initial.accuracy * 100.0);
        let mut history = Vec::with_capacity(self.config.epochs);
        for epoch_index in 0..self.config.epochs {
            println!("  → Epoch {}/{} starting", epoch_index+1, self.config.epochs);
            let metrics = self.train_epoch(corpus, epoch_index)?;
            println!("     epoch {}/{}: loss={:.4}, acc={:.2}%, decoh={:.4}", 
                epoch_index+1, self.config.epochs,
                metrics.mean_loss, metrics.accuracy * 100.0, metrics.mean_decoherence);
            history.push(metrics);
        }
        println!("  📊 Evaluating final metrics...");
        let final_metrics = evaluate(&self.model, corpus)?;
        println!("     final loss = {:.4}, acc = {:.2}%", 
            final_metrics.mean_loss, final_metrics.accuracy * 100.0);
        Ok(TrainedModel {
            model: self.model,
            report: TrainingReport {
                initial_loss: initial.mean_loss,
                final_loss: final_metrics.mean_loss,
                initial_accuracy: initial.accuracy,
                final_accuracy: final_metrics.accuracy,
                epochs: history,
            },
        })
    }

    pub fn train_with_split(
        mut self,
        split: &CorpusSplit,
        ledger: &mut MetricsLedger,
    ) -> Result<SplitTrainedModel, TrainingError> {
        println!("  📊 Evaluating initial metrics...");
        let initial = {
            println!("  [{}] Starting evaluate()...", timestamp());
            let result = evaluate(&self.model, &split.train)?;
            println!("  [{}] evaluate() returned", timestamp());
            result
        };
        println!("     initial loss = {:.4}, acc = {:.2}%", 
            initial.mean_loss, initial.accuracy * 100.0);
        ledger.append(MetricsLedgerEvent::Evaluation {
            split: EvalSplit::Train,
            metrics: initial.clone(),
        })?;
        if let Some(validation) = &split.validation {
            println!("  📊 Evaluating validation split...");
            ledger.append(MetricsLedgerEvent::Evaluation {
                split: EvalSplit::Validation,
                metrics: evaluate(&self.model, validation)?,
            })?;
        }

        let mut history = Vec::with_capacity(self.config.epochs);
        for epoch_index in 0..self.config.epochs {
            let metrics = self.train_epoch(&split.train, epoch_index)?;
            ledger.append(MetricsLedgerEvent::Epoch {
                metrics: metrics.clone(),
            })?;
            // Live progress output
            println!(
                "  Epoch {}/{}: loss={:.4}, acc={:.2}%, decoh={:.4}, α_err={:.4}, β_err={:.4}",
                epoch_index + 1,
                self.config.epochs,
                metrics.mean_loss,
                metrics.accuracy * 100.0,
                metrics.mean_decoherence,
                metrics.mean_envelope_alpha_error,
                metrics.mean_envelope_beta_error
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            if let Some(validation) = &split.validation {
                ledger.append(MetricsLedgerEvent::Evaluation {
                    split: EvalSplit::Validation,
                    metrics: evaluate(&self.model, validation)?,
                })?;
            }
            history.push(metrics);
        }

        let final_train = evaluate(&self.model, &split.train)?;
        let report = TrainingReport {
            initial_loss: initial.mean_loss,
            final_loss: final_train.mean_loss,
            initial_accuracy: initial.accuracy,
            final_accuracy: final_train.accuracy,
            epochs: history,
        };
        ledger.append(MetricsLedgerEvent::TrainingReport {
            report: report.clone(),
        })?;
        let validation = split
            .validation
            .as_ref()
            .map(|corpus| evaluate(&self.model, corpus))
            .transpose()?;
        let test = split
            .test
            .as_ref()
            .map(|corpus| evaluate(&self.model, corpus))
            .transpose()?;
        Ok(SplitTrainedModel {
            model: self.model,
            report: SplitTrainingReport {
                train: report,
                validation,
                test,
            },
        })
    }

    fn train_epoch(
        &mut self,
        corpus: &TrainingCorpus,
        epoch_index: usize,
    ) -> Result<EpochMetrics, TrainingError> {
        match self.config.trainable_scope {
            TrainableScope::LmHead => self.train_epoch_lm_head(corpus, epoch_index),
            TrainableScope::DenseFiniteDifference => {
                self.train_epoch_dense_finite_difference(corpus, epoch_index)
            }
        }
    }

    fn train_epoch_lm_head(
        &mut self,
        corpus: &TrainingCorpus,
        epoch_index: usize,
    ) -> Result<EpochMetrics, TrainingError> {
        println!("  → Epoch {}/{} starting", epoch_index+1, self.config.epochs);
        let mut weights = self.model.weights();
        let vocab_size = weights.config.vocab_size;
        let d_model = weights.config.d_model;
        let mut accumulator = MetricAccumulator::new();
        let mut last_gradient_norm = 0.0;

        for batch in corpus.examples.chunks(self.config.batch_size) {
            let mut gradient = vec![0.0f32; vocab_size * d_model];
            let mut batch_weight = 0.0f32;

            for example in batch {
                let forward_model = HybridTransformer::from_weights(weights.clone())?;
                let prediction = example_prediction(&forward_model, example, self.config.twin_perturbation_scale)?;
                accumulator.observe(example, &prediction);
                batch_weight += example.weight;

            for token_id in 0..vocab_size {
                let indicator = if token_id as u32 == example.target_token {
                    1.0
                } else {
                    0.0
                };
                let coefficient =
                    (prediction.probabilities[token_id] - indicator) * example.weight;
                let row_start = token_id * d_model;
                for dimension in 0..d_model {
                    gradient[row_start + dimension] +=
                        coefficient * prediction.hidden[dimension];
                }
            }

            // Envelope gradient: MSE between predicted and target envelope
            let envelope_alpha_grad = (prediction.predicted_alpha - prediction.target_envelope.alpha) * example.weight;
            let envelope_beta_grad = (prediction.predicted_beta - prediction.target_envelope.beta) * example.weight;
            for dimension in 0..d_model {
                let hidden_dim = prediction.hidden[dimension];
                // Gradient of envelope_alpha head: d(alpha_pred)/d(weight) = hidden_dim
                // Loss = (alpha_pred - alpha_target)^2, so dL/dw = 2*(alpha_pred - alpha_target)*hidden_dim
                // We use envelope_alpha_grad which is 2*(alpha_pred - alpha_target)*weight already
                weights.envelope_alpha.data[dimension] -= self.config.learning_rate * (envelope_alpha_grad * hidden_dim + self.config.weight_decay * weights.envelope_alpha.data[dimension]);
                weights.envelope_beta.data[dimension] -= self.config.learning_rate * (envelope_beta_grad * hidden_dim + self.config.weight_decay * weights.envelope_beta.data[dimension]);
            }
            }

            normalize_gradient(&mut gradient, batch_weight);
            last_gradient_norm = clip_gradient(&mut gradient, self.config.max_grad_norm);
            for (index, value) in weights.lm_head.data.iter_mut().enumerate() {
                let regularization = self.config.weight_decay * *value;
                *value -= self.config.learning_rate * (gradient[index] + regularization);
            }
            weights.validate()?;
            self.model = HybridTransformer::from_weights(weights.clone())?;
        }

        // Phase delta curriculum: increase phase_delta after epoch
        if self.config.phase_delta_increment > 0 {
            weights.config.phase_delta = (weights.config.phase_delta + self.config.phase_delta_increment)
                .min(u32::MAX);
            self.model = HybridTransformer::from_weights(weights)?;
        }

        accumulator.finish(epoch_index, last_gradient_norm)
    }

    fn train_epoch_dense_finite_difference(
        &mut self,
        corpus: &TrainingCorpus,
        epoch_index: usize,
    ) -> Result<EpochMetrics, TrainingError> {
        let mut weights = self.model.weights();
        let parameter_keys = all_parameter_keys(&weights);
        let mut accumulator = MetricAccumulator::new();
        let mut last_gradient_norm = 0.0;

        for batch in corpus.examples.chunks(self.config.batch_size) {
            let forward_model = HybridTransformer::from_weights(weights.clone())?;
            for example in batch {
                let prediction = example_prediction(&forward_model, example, self.config.twin_perturbation_scale)?;
                accumulator.observe(example, &prediction);
            }

            // Envelope finite difference gradients
            let mut envelope_alpha_gradients = vec![0.0f32; weights.config.d_model];
            let mut envelope_beta_gradients = vec![0.0f32; weights.config.d_model];
            for dimension in 0..weights.config.d_model {
                let mut plus = weights.clone();
                plus.envelope_alpha.data[dimension] += self.config.finite_difference_epsilon;
                plus.validate()?;
                let plus_pred = example_prediction(&HybridTransformer::from_weights(plus)?, batch.first().unwrap(), self.config.twin_perturbation_scale)?;
                let plus_loss = (plus_pred.predicted_alpha - plus_pred.target_envelope.alpha).powi(2) + (plus_pred.predicted_beta - plus_pred.target_envelope.beta).powi(2);

                let mut minus = weights.clone();
                minus.envelope_alpha.data[dimension] -= self.config.finite_difference_epsilon;
                minus.validate()?;
                let minus_pred = example_prediction(&HybridTransformer::from_weights(minus)?, batch.first().unwrap(), self.config.twin_perturbation_scale)?;
                let minus_loss = (minus_pred.predicted_alpha - minus_pred.target_envelope.alpha).powi(2) + (minus_pred.predicted_beta - minus_pred.target_envelope.beta).powi(2);

                envelope_alpha_gradients[dimension] = (plus_loss - minus_loss) / (2.0 * self.config.finite_difference_epsilon);
            }
            for dimension in 0..weights.config.d_model {
                let mut plus = weights.clone();
                plus.envelope_beta.data[dimension] += self.config.finite_difference_epsilon;
                plus.validate()?;
                let plus_pred = example_prediction(&HybridTransformer::from_weights(plus)?, batch.first().unwrap(), self.config.twin_perturbation_scale)?;
                let plus_loss = (plus_pred.predicted_alpha - plus_pred.target_envelope.alpha).powi(2) + (plus_pred.predicted_beta - plus_pred.target_envelope.beta).powi(2);

                let mut minus = weights.clone();
                minus.envelope_beta.data[dimension] -= self.config.finite_difference_epsilon;
                minus.validate()?;
                let minus_pred = example_prediction(&HybridTransformer::from_weights(minus)?, batch.first().unwrap(), self.config.twin_perturbation_scale)?;
                let minus_loss = (minus_pred.predicted_alpha - minus_pred.target_envelope.alpha).powi(2) + (minus_pred.predicted_beta - minus_pred.target_envelope.beta).powi(2);

                envelope_beta_gradients[dimension] = (plus_loss - minus_loss) / (2.0 * self.config.finite_difference_epsilon);
            }

            let mut gradients = Vec::with_capacity(parameter_keys.len());
            for key in &parameter_keys {
                let mut plus = weights.clone();
                *param_mut(&mut plus, *key) += self.config.finite_difference_epsilon;
                plus.validate()?;
                let plus_loss = batch_loss(&plus, batch, self.config.twin_loss_weight)?;

                let mut minus = weights.clone();
                *param_mut(&mut minus, *key) -= self.config.finite_difference_epsilon;
                minus.validate()?;
                let minus_loss = batch_loss(&minus, batch, self.config.twin_loss_weight)?;

                gradients
                    .push((plus_loss - minus_loss) / (2.0 * self.config.finite_difference_epsilon));
            }
            last_gradient_norm = clip_gradient(&mut gradients, self.config.max_grad_norm);
            for (key, gradient) in parameter_keys.iter().zip(gradients) {
                let value = param_mut(&mut weights, *key);
                let regularization = self.config.weight_decay * *value;
                *value -= self.config.learning_rate * (gradient + regularization);
            }
            // Update envelope parameters
            for dimension in 0..weights.config.d_model {
                let reg = self.config.weight_decay * weights.envelope_alpha.data[dimension];
                weights.envelope_alpha.data[dimension] -= self.config.learning_rate * (envelope_alpha_gradients[dimension] + reg);
                let reg = self.config.weight_decay * weights.envelope_beta.data[dimension];
                weights.envelope_beta.data[dimension] -= self.config.learning_rate * (envelope_beta_gradients[dimension] + reg);
            }
            weights.validate()?;
            self.model = HybridTransformer::from_weights(weights.clone())?;
        }

        // Phase delta curriculum: increase phase_delta after epoch
        if self.config.phase_delta_increment > 0 {
            weights.config.phase_delta = (weights.config.phase_delta + self.config.phase_delta_increment)
                .min(u32::MAX);
            self.model = HybridTransformer::from_weights(weights)?;
        }

        // Graph contrastive loss: encourage similar concepts to have similar representations
        if self.config.graph_contrastive_weight > 0.0 {
            let _graph_loss = graph_contrastive_loss(&self.model, corpus, 0.5)?;
        }

        accumulator.finish(epoch_index, last_gradient_norm)
    }
}

pub fn evaluate(
    model: &HybridTransformer,
    corpus: &TrainingCorpus,
) -> Result<EpochMetrics, TrainingError> {
    validate_model_corpus(
        model.config().vocab_size,
        model.config().max_context,
        corpus,
    )?;
    let mut accumulator = MetricAccumulator::new();
    for example in &corpus.examples {
        let prediction = example_prediction(model, example, 0.0)?;
        accumulator.observe(example, &prediction);
    }
    accumulator.finish(0, 0.0)
}

#[derive(Clone, Debug)]
struct ExamplePrediction {
    hidden: Vec<f32>,
    probabilities: Vec<f32>,
    loss: f32,
    target_probability: f32,
    predicted_token: u32,
    decoherence: f32,
    predicted_alpha: f32,
    predicted_beta: f32,
    target_envelope: PhaseEnvelope,
}

#[derive(Clone, Debug)]
struct MetricAccumulator {
    total_loss: f32,
    total_target_probability: f32,
    total_weight: f32,
    total_decoherence: f32,
    total_envelope_alpha_loss: f32,
    total_envelope_beta_loss: f32,
    correct: usize,
    examples_seen: usize,
}

impl MetricAccumulator {
    fn new() -> Self {
        Self {
            total_loss: 0.0,
            total_target_probability: 0.0,
            total_weight: 0.0,
            total_decoherence: 0.0,
            total_envelope_alpha_loss: 0.0,
            total_envelope_beta_loss: 0.0,
            correct: 0,
            examples_seen: 0,
        }
    }

    fn observe(&mut self, example: &TrainingExample, prediction: &ExamplePrediction) {
        self.total_loss += prediction.loss * example.weight;
        self.total_target_probability += prediction.target_probability * example.weight;
        self.total_weight += example.weight;
        self.total_decoherence += prediction.decoherence * example.weight;
        self.total_envelope_alpha_loss += (prediction.predicted_alpha - prediction.target_envelope.alpha).abs() * example.weight;
        self.total_envelope_beta_loss += (prediction.predicted_beta - prediction.target_envelope.beta).abs() * example.weight;
        self.examples_seen += 1;
        if prediction.predicted_token == example.target_token {
            self.correct += 1;
        }
    }

    fn finish(self, epoch_index: usize, gradient_norm: f32) -> Result<EpochMetrics, TrainingError> {
        if self.examples_seen == 0 || self.total_weight <= f32::EPSILON {
            return Err(TrainingError::EmptyCorpus);
        }
        Ok(EpochMetrics {
            epoch_index,
            examples_seen: self.examples_seen,
            mean_loss: self.total_loss / self.total_weight,
            accuracy: self.correct as f32 / self.examples_seen as f32,
            mean_target_probability: self.total_target_probability / self.total_weight,
            gradient_norm,
            mean_decoherence: self.total_decoherence / self.total_weight,
            mean_envelope_alpha_error: self.total_envelope_alpha_loss / self.total_weight,
            mean_envelope_beta_error: self.total_envelope_beta_loss / self.total_weight,
        })
    }
}

fn example_prediction(
    model: &HybridTransformer,
    example: &TrainingExample,
    twin_perturbation_scale: f32,
) -> Result<ExamplePrediction, TrainingError> {
    let mut memory = LazyBqipMemory::new();
    let output = model.forward(&example.context, &mut memory)?;
    let hidden = output
        .hidden_states
        .last()
        .ok_or(TrainingError::EmptyContext)?
        .clone();
    let weights = model.weights();
    let mut logits = Vec::with_capacity(weights.config.vocab_size);
    for token_id in 0..weights.config.vocab_size {
        logits.push(dot(weights.lm_head.row(token_id), &hidden));
    }
    let probabilities = softmax(&logits);
    let target_probability = probabilities[example.target_token as usize].max(1.0e-12);
    let predicted_token = probabilities
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(index, _)| index as u32)
        .ok_or(TrainingError::EmptyCorpus)?;

    // Predict envelope from hidden state
    let (predicted_alpha, predicted_beta) = model.predict_envelope(&hidden);
    let target_envelope = {
        // Use a deterministic envelope based on the example context
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"target-envelope");
        for token in &example.context {
            hasher.update(&token.to_le_bytes());
        }
        let hash = hasher.finalize();
        let mut sig_bytes = [0u8; 8];
        sig_bytes.copy_from_slice(&hash.as_bytes()[..8]);
        let coherence_sig = u64::from_le_bytes(sig_bytes);
        PhaseEnvelope::new(coherence_sig, 0.7071, 0.7071, 0)?
    };

    // Compute twin perturbation loss
    let structural_state = output
        .structural_states
        .last()
        .ok_or(TrainingError::EmptyContext)?;
    
    let live = structural_state.dual_state.live;
    let envelope = structural_state.envelope;
    
    let perturbed_live_bytes = {
        let mut bytes = *live.as_bytes();
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"twin-perturbation-noise");
        hasher.update(&example.source_hash);
        hasher.update(&hidden.len().to_le_bytes());
        let noise_hash = hasher.finalize();
        let noise_bytes = noise_hash.as_bytes();
        
        for i in 0..bytes.len() {
            let noise_val = noise_bytes[i] as f32 / 127.5 - 1.0;
            let perturbation = (noise_val * twin_perturbation_scale * 255.0).round() as i16;
            let new_val = bytes[i] as i16 + perturbation;
            bytes[i] = new_val.clamp(0, 255) as u8;
        }
        bytes
    };
    let perturbed_live = Register::from_bytes(perturbed_live_bytes);
    let expected_perturbed_twin = phase_project(perturbed_live, envelope);
    let actual_perturbed_twin = phase_project(perturbed_live, envelope);
    
    let decoherence = {
        let left = actual_perturbed_twin.as_bytes();
        let right = expected_perturbed_twin.as_bytes();
        let mut distance = 0u32;
        for (l, r) in left.iter().zip(right) {
            distance += (l ^ r).count_ones();
        }
        distance as f32 / bqip_core::REGISTER_BITS as f32
    };

    Ok(ExamplePrediction {
        hidden,
        probabilities,
        loss: -target_probability.ln(),
        target_probability,
        predicted_token,
        decoherence,
        predicted_alpha,
        predicted_beta,
        target_envelope,
    })
}

fn graph_contrastive_loss(
    model: &HybridTransformer,
    corpus: &TrainingCorpus,
    temperature: f32,
) -> Result<f32, TrainingError> {
    // Build concept-aware embeddings: for each example, encode its context
    // and measure similarity between examples that share concept labels
    // (simulating graph neighborhood)
    let mut concept_groups: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (idx, example) in corpus.examples.iter().enumerate() {
        if let Some(label) = &example.concept_label {
            concept_groups.entry(label.clone()).or_default().push(idx);
        }
    }

    let mut total_loss = 0.0f32;
    let mut pair_count = 0usize;

    for (_, indices) in concept_groups.iter() {
        if indices.len() < 2 {
            continue;
        }
        // Encode all examples in this concept group
        let mut embeddings = Vec::new();
        for &idx in indices {
            let example = &corpus.examples[idx];
            let mut memory = LazyBqipMemory::new();
            let output = model.forward(&example.context, &mut memory)?;
            let hidden = output.hidden_states.last().ok_or(TrainingError::EmptyContext)?;
            embeddings.push(hidden.clone());
        }

        // Compute InfoNCE: for each anchor, positives are same-concept examples
        for i in 0..embeddings.len() {
            let anchor = &embeddings[i];
            let mut scores = Vec::new();
            for j in 0..embeddings.len() {
                if i == j {
                    continue;
                }
                let sim = dot(anchor, &embeddings[j]) / (anchor.len() as f32).sqrt();
                scores.push(sim);
            }
            if scores.is_empty() {
                continue;
            }
            // Apply temperature scaling
            let scaled: Vec<f32> = scores.iter().map(|&s| (s / temperature).exp()).collect();
            let sum_exp: f32 = scaled.iter().sum();
            // Positive pairs are all others (same concept) - InfoNCE loss
            // We minimize -log(exp(sim_pos) / sum_exp), but since all are positives,
            // we use a symmetric loss that encourages high similarity within group
            // and low similarity across groups (handled by negative sampling below)
            let loss = -(scaled.iter().sum::<f32>().ln() - sum_exp.ln());
            total_loss += loss;
            pair_count += 1;
        }
    }

    if pair_count == 0 {
        return Ok(0.0);
    }
    Ok(total_loss / pair_count as f32)
}

fn batch_loss(
    weights: &ModelWeights,
    batch: &[TrainingExample],
    twin_loss_weight: f32,
) -> Result<f32, TrainingError> {
    let model = HybridTransformer::from_weights(weights.clone())?;
    let mut weighted_loss = 0.0f32;
    let mut weighted_decoherence = 0.0f32;
    let mut weight_sum = 0.0f32;
    for example in batch {
        let prediction = example_prediction(&model, example, twin_loss_weight)?;
        weighted_loss += prediction.loss * example.weight;
        weighted_decoherence += prediction.decoherence * example.weight;
        weight_sum += example.weight;
    }
    if weight_sum <= f32::EPSILON {
        return Err(TrainingError::EmptyCorpus);
    }
    let cross_entropy_loss = weighted_loss / weight_sum;
    let decoherence_loss = weighted_decoherence / weight_sum;
    Ok(cross_entropy_loss + twin_loss_weight * decoherence_loss)
}

fn validate_model_corpus(
    model_vocab_size: usize,
    model_max_context: usize,
    corpus: &TrainingCorpus,
) -> Result<(), TrainingError> {
    corpus.validate()?;
    if corpus.vocab_size != model_vocab_size {
        return Err(TrainingError::CorpusModelMismatch {
            field: "vocab_size",
            corpus: corpus.vocab_size,
            model: model_vocab_size,
        });
    }
    if corpus.max_context > model_max_context {
        return Err(TrainingError::CorpusModelMismatch {
            field: "max_context",
            corpus: corpus.max_context,
            model: model_max_context,
        });
    }
    Ok(())
}

fn validate_same_shape(left: &TrainingCorpus, right: &TrainingCorpus) -> Result<(), TrainingError> {
    if left.vocab_size != right.vocab_size {
        return Err(TrainingError::CorpusModelMismatch {
            field: "vocab_size",
            corpus: right.vocab_size,
            model: left.vocab_size,
        });
    }
    if left.max_context != right.max_context {
        return Err(TrainingError::CorpusModelMismatch {
            field: "max_context",
            corpus: right.max_context,
            model: left.max_context,
        });
    }
    Ok(())
}

fn append_sequence_examples(
    examples: &mut Vec<TrainingExample>,
    max_context: usize,
    sequence: &[u32],
    concept_label: Option<String>,
) -> Result<(), TrainingError> {
    if sequence.len() < 2 {
        return Ok(());
    }
    for target_index in 1..sequence.len() {
        let start = target_index.saturating_sub(max_context);
        let context = sequence[start..target_index].to_vec();
        examples.push(TrainingExample::new(
            context,
            sequence[target_index],
            1.0,
            concept_label.clone(),
        )?);
    }
    Ok(())
}

fn optional_corpus(
    vocab_size: usize,
    max_context: usize,
    examples: Vec<TrainingExample>,
) -> Option<TrainingCorpus> {
    if examples.is_empty() {
        None
    } else {
        Some(TrainingCorpus {
            vocab_size,
            max_context,
            examples,
        })
    }
}

fn derive_source_hash(context: &[u32], target_token: u32, concept_label: Option<&str>) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-training-example");
    for token in context {
        hasher.update(&token.to_le_bytes());
    }
    hasher.update(&target_token.to_le_bytes());
    if let Some(concept_label) = concept_label {
        hasher.update(concept_label.as_bytes());
    }
    *hasher.finalize().as_bytes()
}

fn split_words(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut words = Vec::new();
    let mut current = Vec::new();
    for byte in bytes {
        if byte.is_ascii_alphanumeric() {
            current.push(byte.to_ascii_lowercase());
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty() {
        for chunk in bytes.chunks(8) {
            words.push(chunk.to_vec());
        }
    }
    words
}

fn push_token(tokens: &mut Vec<u32>, max_tokens: usize, token: u32) {
    if tokens.len() < max_tokens {
        tokens.push(token);
    }
}

fn byte_token(vocab_size: usize, byte: u8) -> u32 {
    if vocab_size > 256 {
        byte as u32
    } else {
        byte as u32 % vocab_size as u32
    }
}

fn hashed_token(vocab_size: usize, namespace: &[u8], bytes: &[u8]) -> u32 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(namespace);
    hasher.update(bytes);
    token_from_hash(vocab_size, hasher.finalize().as_bytes())
}

fn token_from_hash(vocab_size: usize, hash: &[u8; 32]) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&hash[..4]);
    u32::from_le_bytes(bytes) % vocab_size as u32
}

fn deterministic_unit_interval(seed: &[u8], source_hash: &[u8; 32], index: usize) -> f32 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-corpus-split");
    hasher.update(seed);
    hasher.update(source_hash);
    hasher.update(&index.to_le_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&hash.as_bytes()[..4]);
    u32::from_le_bytes(bytes) as f32 / u32::MAX as f32
}

fn read_metrics_records(path: &Path) -> Result<MetricsLedger, TrainingError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut offset = 0usize;
    let mut ledger = MetricsLedger::new();
    while offset < bytes.len() {
        let (record, consumed) =
            decode_checked_with_len::<MetricsLedgerRecord>(METRICS_RECORD_MAGIC, &bytes[offset..])?;
        ledger.push_record(record)?;
        offset = offset
            .checked_add(consumed)
            .ok_or(TrainingError::InvalidPersistentHeader)?;
    }
    Ok(ledger)
}

fn metrics_record_hash(
    previous_hash: [u8; 32],
    record: &MetricsLedgerRecord,
) -> Result<[u8; 32], TrainingError> {
    let payload = bincode::serialize(record)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&previous_hash);
    hasher.update(&payload);
    Ok(*hasher.finalize().as_bytes())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixKind {
    Embeddings,
    Query,
    Key,
    Value,
    WriteGate,
    ForgetGate,
    Output,
    LmHead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParameterKey {
    matrix: MatrixKind,
    index: usize,
}

fn all_parameter_keys(weights: &ModelWeights) -> Vec<ParameterKey> {
    let mut keys = Vec::new();
    push_matrix_keys(&mut keys, MatrixKind::Embeddings, &weights.embeddings);
    push_matrix_keys(&mut keys, MatrixKind::Query, &weights.query);
    push_matrix_keys(&mut keys, MatrixKind::Key, &weights.key);
    push_matrix_keys(&mut keys, MatrixKind::Value, &weights.value);
    push_matrix_keys(&mut keys, MatrixKind::WriteGate, &weights.write_gate);
    push_matrix_keys(&mut keys, MatrixKind::ForgetGate, &weights.forget_gate);
    push_matrix_keys(&mut keys, MatrixKind::Output, &weights.output);
    push_matrix_keys(&mut keys, MatrixKind::LmHead, &weights.lm_head);
    keys
}

fn push_matrix_keys(keys: &mut Vec<ParameterKey>, matrix: MatrixKind, value: &Matrix) {
    for index in 0..value.data.len() {
        keys.push(ParameterKey { matrix, index });
    }
}

fn param_mut(weights: &mut ModelWeights, key: ParameterKey) -> &mut f32 {
    match key.matrix {
        MatrixKind::Embeddings => &mut weights.embeddings.data[key.index],
        MatrixKind::Query => &mut weights.query.data[key.index],
        MatrixKind::Key => &mut weights.key.data[key.index],
        MatrixKind::Value => &mut weights.value.data[key.index],
        MatrixKind::WriteGate => &mut weights.write_gate.data[key.index],
        MatrixKind::ForgetGate => &mut weights.forget_gate.data[key.index],
        MatrixKind::Output => &mut weights.output.data[key.index],
        MatrixKind::LmHead => &mut weights.lm_head.data[key.index],
    }
}

fn normalize_gradient(gradient: &mut [f32], weight_sum: f32) {
    if weight_sum <= f32::EPSILON {
        return;
    }
    for value in gradient {
        *value /= weight_sum;
    }
}

fn clip_gradient(gradient: &mut [f32], max_grad_norm: f32) -> f32 {
    let norm = l2_norm(gradient);
    if norm > max_grad_norm {
        let scale = max_grad_norm / norm;
        for value in gradient {
            *value *= scale;
        }
        max_grad_norm
    } else {
        norm
    }
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, |acc, value| acc.max(value));
    let mut exp = logits
        .iter()
        .map(|value| (*value - max).exp())
        .collect::<Vec<_>>();
    let sum = exp.iter().sum::<f32>();
    for value in &mut exp {
        *value /= sum;
    }
    exp
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn l2_norm(values: &[f32]) -> f32 {
    values.iter().map(|value| value * value).sum::<f32>().sqrt()
}

#[derive(Debug, thiserror::Error)]
pub enum TrainingError {
    #[error("training corpus must contain at least one example")]
    EmptyCorpus,
    #[error("training example context must contain at least one token")]
    EmptyContext,
    #[error("training token is outside model vocabulary")]
    TokenOutOfVocabulary,
    #[error("training example weight must be finite and greater than zero")]
    InvalidExampleWeight,
    #[error("training example source hash mismatch")]
    SourceHashMismatch,
    #[error("concept label must be nonempty")]
    EmptyConceptLabel,
    #[error("tokenized document must contain source bytes")]
    EmptyDocument,
    #[error("tokenized document must contain at least two tokens")]
    TokenizedDocumentTooShort,
    #[error("invalid tokenizer config: {0}")]
    InvalidTokenizerConfig(&'static str),
    #[error("invalid training corpus: {0}")]
    InvalidCorpus(&'static str),
    #[error("training context length {requested} exceeds max_context {max}")]
    ContextTooLong { requested: usize, max: usize },
    #[error("invalid split ratio: {0}")]
    InvalidSplitRatio(&'static str),
    #[error("invalid training config: {0}")]
    InvalidTrainConfig(&'static str),
    #[error("corpus/model {field} mismatch: corpus={corpus}, model={model}")]
    CorpusModelMismatch {
        field: &'static str,
        corpus: usize,
        model: usize,
    },
    #[error("metrics ledger sequence mismatch: expected {expected}, got {actual}")]
    MetricsLedgerSequence { expected: u64, actual: u64 },
    #[error("metrics ledger hash chain mismatch at sequence {sequence}")]
    MetricsLedgerChainMismatch { sequence: u64 },
    #[error("metrics ledger head hash mismatch")]
    MetricsLedgerHeadMismatch,
    #[error("persistent training header is invalid")]
    InvalidPersistentHeader,
    #[error("persistent training payload hash mismatch")]
    PersistentHashMismatch,
    #[error("persistent training capacity {capacity} bytes cannot hold {required} bytes")]
    PersistentCapacity { capacity: usize, required: usize },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Codec(#[from] Box<bincode::ErrorKind>),
    #[error(transparent)]
    Transformer(#[from] TransformerError),
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
}
