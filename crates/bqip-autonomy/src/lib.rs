use bqip_core::{DualState, PhaseEnvelope, Register, REGISTER_BYTES};
use bqip_multimodal::MultimodalGrounding;
use bqip_training::{
    ConceptTokenCompiler, CorpusSplit, HybridTrainer, MetricsLedger, SplitRatios,
    SplitTrainedModel, TokenizedDocument, TrainConfig, TrainingCorpus, TrainingError,
};
use bqip_transformer::HybridTransformer;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InferenceObservation {
    pub provider: String,
    pub model: String,
    pub concept_label: String,
    pub prompt: String,
    pub response: String,
    pub confidence: f32,
    pub latency_ms: u64,
    pub cost_microunits: u64,
    pub observed_at_unix_millis: u64,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub source_hash: [u8; REGISTER_BYTES],
}

impl InferenceObservation {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        concept_label: impl Into<String>,
        prompt: impl Into<String>,
        response: impl Into<String>,
        confidence: f32,
        latency_ms: u64,
        cost_microunits: u64,
        observed_at_unix_millis: u64,
        envelope: PhaseEnvelope,
    ) -> Result<Self, AutonomyError> {
        let provider = provider.into();
        let model = model.into();
        let concept_label = concept_label.into();
        let prompt = prompt.into();
        let response = response.into();
        let source_hash = inference_hash(&provider, &model, &concept_label, &prompt, &response);
        let live = inference_register(
            &source_hash,
            confidence,
            latency_ms,
            cost_microunits,
            observed_at_unix_millis,
        );
        let observation = Self {
            provider,
            model,
            concept_label,
            prompt,
            response,
            confidence,
            latency_ms,
            cost_microunits,
            observed_at_unix_millis,
            envelope,
            state: DualState::from_live(live, envelope),
            source_hash,
        };
        observation.validate()?;
        Ok(observation)
    }

    pub fn validate(&self) -> Result<(), AutonomyError> {
        if self.provider.trim().is_empty()
            || self.model.trim().is_empty()
            || self.concept_label.trim().is_empty()
            || self.prompt.trim().is_empty()
            || self.response.trim().is_empty()
        {
            return Err(AutonomyError::EmptyInferenceField);
        }
        validate_unit(self.confidence, "confidence")?;
        self.envelope.validate()?;
        if self.source_hash
            != inference_hash(
                &self.provider,
                &self.model,
                &self.concept_label,
                &self.prompt,
                &self.response,
            )
        {
            return Err(AutonomyError::InferenceHashMismatch);
        }
        if self.state.twin != bqip_core::phase_project(self.state.live, self.envelope) {
            return Err(AutonomyError::InvalidTwinProjection);
        }
        Ok(())
    }

    pub fn tokenized(
        &self,
        compiler: &ConceptTokenCompiler,
    ) -> Result<TokenizedDocument, AutonomyError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.provider.as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(self.model.as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(self.prompt.as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(self.response.as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(&self.confidence.to_bits().to_le_bytes());
        Ok(compiler.compile(self.concept_label.clone(), &bytes)?)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectiveInferenceSuperposition {
    pub concept_label: String,
    pub observation_count: usize,
    pub provider_count: usize,
    pub consensus: f32,
    pub conflict: f32,
    pub mean_confidence: f32,
    pub mean_latency_ms: f32,
    pub total_cost_microunits: u64,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub source_hash: [u8; REGISTER_BYTES],
}

impl CollectiveInferenceSuperposition {
    pub fn from_observations(
        concept_label: impl Into<String>,
        observations: &[InferenceObservation],
        envelope: PhaseEnvelope,
    ) -> Result<Self, AutonomyError> {
        let concept_label = concept_label.into();
        if concept_label.trim().is_empty() {
            return Err(AutonomyError::EmptyInferenceField);
        }
        if observations.is_empty() {
            return Err(AutonomyError::EmptyObservationSet);
        }
        envelope.validate()?;
        let mut providers = Vec::<String>::new();
        let mut collective_live = Register::zero();
        let mut total_confidence = 0.0f32;
        let mut total_latency = 0.0f32;
        let mut total_cost = 0u64;
        let mut response_hashes = Vec::with_capacity(observations.len());
        for observation in observations {
            observation.validate()?;
            if canonical(&observation.concept_label) != canonical(&concept_label) {
                return Err(AutonomyError::ConceptMismatch);
            }
            if !providers
                .iter()
                .any(|provider| provider == &observation.provider)
            {
                providers.push(observation.provider.clone());
            }
            collective_live = collective_live.xor(&observation.state.live);
            total_confidence += observation.confidence;
            total_latency += observation.latency_ms as f32;
            total_cost = total_cost.saturating_add(observation.cost_microunits);
            response_hashes.push(blake3::hash(observation.response.as_bytes()));
        }
        let pairs = response_hashes.len() * response_hashes.len().saturating_sub(1) / 2;
        let mut similarity_sum = 0.0f32;
        for left in 0..response_hashes.len() {
            for right in left + 1..response_hashes.len() {
                similarity_sum += 1.0
                    - normalized_hash_hamming(
                        response_hashes[left].as_bytes(),
                        response_hashes[right].as_bytes(),
                    );
            }
        }
        let consensus = if pairs == 0 {
            1.0
        } else {
            similarity_sum / pairs as f32
        };
        let conflict = 1.0 - consensus;
        let source_hash = collective_hash(&concept_label, observations);
        let state = DualState::from_live(
            collective_live.xor(&Register::from_bytes(source_hash)),
            envelope,
        );
        Ok(Self {
            concept_label,
            observation_count: observations.len(),
            provider_count: providers.len(),
            consensus,
            conflict,
            mean_confidence: total_confidence / observations.len() as f32,
            mean_latency_ms: total_latency / observations.len() as f32,
            total_cost_microunits: total_cost,
            envelope,
            state,
            source_hash,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExplorationKind {
    Endpoint {
        url: String,
    },
    InferencePrompt {
        provider: String,
        model: String,
        prompt: String,
    },
    VisualProbe {
        content_hash: [u8; REGISTER_BYTES],
    },
    VideoProbe {
        content_hash: [u8; REGISTER_BYTES],
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplorationCandidate {
    pub id: u64,
    pub concept_label: String,
    pub kind: ExplorationKind,
    pub expected_value: f32,
    pub novelty_state: Register,
    pub confidence_prior: f32,
    pub estimated_cost_microunits: u64,
    pub estimated_latency_ms: u64,
}

impl ExplorationCandidate {
    pub fn validate(&self) -> Result<(), AutonomyError> {
        if self.concept_label.trim().is_empty() {
            return Err(AutonomyError::EmptyInferenceField);
        }
        validate_unit(self.expected_value, "expected_value")?;
        validate_unit(self.confidence_prior, "confidence_prior")?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValueFitnessWeights {
    pub expected_value: f32,
    pub novelty: f32,
    pub confidence: f32,
    pub cost_penalty: f32,
    pub latency_penalty: f32,
}

impl Default for ValueFitnessWeights {
    fn default() -> Self {
        Self {
            expected_value: 0.34,
            novelty: 0.26,
            confidence: 0.22,
            cost_penalty: 0.1,
            latency_penalty: 0.08,
        }
    }
}

impl ValueFitnessWeights {
    pub fn validate(&self) -> Result<(), AutonomyError> {
        for (name, value) in [
            ("expected_value", self.expected_value),
            ("novelty", self.novelty),
            ("confidence", self.confidence),
            ("cost_penalty", self.cost_penalty),
            ("latency_penalty", self.latency_penalty),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(AutonomyError::InvalidFitnessWeight(name));
            }
        }
        if self.expected_value + self.novelty + self.confidence <= f32::EPSILON {
            return Err(AutonomyError::InvalidFitnessWeight("positive sum"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValueFitnessScore {
    pub candidate_id: u64,
    pub total: f32,
    pub expected_value: f32,
    pub novelty: f32,
    pub confidence: f32,
    pub cost_penalty: f32,
    pub latency_penalty: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RankedExploration {
    pub candidate: ExplorationCandidate,
    pub score: ValueFitnessScore,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutonomousLearningConfig {
    pub vocab_size: usize,
    pub max_context: usize,
    pub split_ratios: SplitRatios,
    pub fitness_weights: ValueFitnessWeights,
}

impl AutonomousLearningConfig {
    pub fn validate(&self) -> Result<(), AutonomyError> {
        if self.vocab_size == 0 || self.max_context == 0 {
            return Err(AutonomyError::InvalidAutonomyConfig(
                "vocab_size and max_context must be nonzero",
            ));
        }
        self.split_ratios.validate()?;
        self.fitness_weights.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AutonomousLearner {
    compiler: ConceptTokenCompiler,
    config: AutonomousLearningConfig,
}

impl AutonomousLearner {
    pub fn new(
        compiler: ConceptTokenCompiler,
        config: AutonomousLearningConfig,
    ) -> Result<Self, AutonomyError> {
        config.validate()?;
        if compiler.config().vocab_size != config.vocab_size {
            return Err(AutonomyError::InvalidAutonomyConfig(
                "compiler vocab_size must match autonomy vocab_size",
            ));
        }
        Ok(Self { compiler, config })
    }

    pub fn rank_exploration(
        &self,
        candidates: &[ExplorationCandidate],
        memory_basis: Register,
    ) -> Result<Vec<RankedExploration>, AutonomyError> {
        let mut ranked = candidates
            .iter()
            .map(|candidate| {
                candidate.validate()?;
                Ok(RankedExploration {
                    candidate: candidate.clone(),
                    score: score_candidate(candidate, memory_basis, &self.config.fitness_weights)?,
                })
            })
            .collect::<Result<Vec<_>, AutonomyError>>()?;
        ranked.sort_by(|left, right| {
            right
                .score
                .total
                .total_cmp(&left.score.total)
                .then_with(|| left.candidate.id.cmp(&right.candidate.id))
        });
        Ok(ranked)
    }

    pub fn inference_documents(
        &self,
        observations: &[InferenceObservation],
    ) -> Result<Vec<TokenizedDocument>, AutonomyError> {
        if observations.is_empty() {
            return Err(AutonomyError::EmptyObservationSet);
        }
        observations
            .iter()
            .map(|observation| observation.tokenized(&self.compiler))
            .collect()
    }

    pub fn multimodal_documents(
        &self,
        groundings: &[MultimodalGrounding],
    ) -> Result<Vec<TokenizedDocument>, AutonomyError> {
        if groundings.is_empty() {
            return Err(AutonomyError::EmptyObservationSet);
        }
        Ok(groundings
            .iter()
            .map(|grounding| grounding.document.clone())
            .collect())
    }

    pub fn build_flash_corpus(
        &self,
        inference_observations: &[InferenceObservation],
        multimodal_groundings: &[MultimodalGrounding],
    ) -> Result<TrainingCorpus, AutonomyError> {
        let mut documents = Vec::new();
        documents.extend(
            self.inference_documents(inference_observations)
                .unwrap_or_default(),
        );
        documents.extend(
            self.multimodal_documents(multimodal_groundings)
                .unwrap_or_default(),
        );
        if documents.is_empty() {
            return Err(AutonomyError::EmptyObservationSet);
        }
        Ok(TrainingCorpus::from_tokenized_documents(
            self.config.vocab_size,
            self.config.max_context,
            &documents,
        )?)
    }

    pub fn flash_learn(
        &self,
        model: HybridTransformer,
        corpus: &TrainingCorpus,
        train_config: TrainConfig,
        ledger: &mut MetricsLedger,
        split_seed: &[u8],
    ) -> Result<SplitTrainedModel, AutonomyError> {
        let split: CorpusSplit = corpus.split(self.config.split_ratios, split_seed)?;
        Ok(HybridTrainer::new(model, train_config)?.train_with_split(&split, ledger)?)
    }
}

pub fn score_candidate(
    candidate: &ExplorationCandidate,
    memory_basis: Register,
    weights: &ValueFitnessWeights,
) -> Result<ValueFitnessScore, AutonomyError> {
    candidate.validate()?;
    weights.validate()?;
    let novelty = normalized_register_hamming(candidate.novelty_state, memory_basis);
    let cost_penalty = normalized_cost(candidate.estimated_cost_microunits);
    let latency_penalty = normalized_latency(candidate.estimated_latency_ms);
    let positive_sum = weights.expected_value + weights.novelty + weights.confidence;
    let positive = candidate.expected_value * weights.expected_value
        + novelty * weights.novelty
        + candidate.confidence_prior * weights.confidence;
    let penalty = cost_penalty * weights.cost_penalty + latency_penalty * weights.latency_penalty;
    let total = (positive / positive_sum - penalty).clamp(0.0, 1.0);
    Ok(ValueFitnessScore {
        candidate_id: candidate.id,
        total,
        expected_value: candidate.expected_value,
        novelty,
        confidence: candidate.confidence_prior,
        cost_penalty,
        latency_penalty,
    })
}

fn validate_unit(value: f32, name: &'static str) -> Result<(), AutonomyError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(AutonomyError::InvalidUnitValue(name));
    }
    Ok(())
}

fn inference_hash(
    provider: &str,
    model: &str,
    concept_label: &str,
    prompt: &str,
    response: &str,
) -> [u8; REGISTER_BYTES] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-inference-observation");
    hasher.update(provider.as_bytes());
    hasher.update(model.as_bytes());
    hasher.update(concept_label.as_bytes());
    hasher.update(prompt.as_bytes());
    hasher.update(response.as_bytes());
    *hasher.finalize().as_bytes()
}

fn inference_register(
    source_hash: &[u8; REGISTER_BYTES],
    confidence: f32,
    latency_ms: u64,
    cost_microunits: u64,
    observed_at_unix_millis: u64,
) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-inference-register");
    hasher.update(source_hash);
    hasher.update(&confidence.to_bits().to_le_bytes());
    hasher.update(&latency_ms.to_le_bytes());
    hasher.update(&cost_microunits.to_le_bytes());
    hasher.update(&observed_at_unix_millis.to_le_bytes());
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn collective_hash(
    concept_label: &str,
    observations: &[InferenceObservation],
) -> [u8; REGISTER_BYTES] {
    let mut hashes = observations
        .iter()
        .map(|observation| observation.source_hash)
        .collect::<Vec<_>>();
    hashes.sort_unstable();
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-collective-inference");
    hasher.update(canonical(concept_label).as_bytes());
    for hash in hashes {
        hasher.update(&hash);
    }
    *hasher.finalize().as_bytes()
}

fn canonical(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn normalized_hash_hamming(left: &[u8; REGISTER_BYTES], right: &[u8; REGISTER_BYTES]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum::<u32>() as f32
        / 256.0
}

fn normalized_register_hamming(left: Register, right: Register) -> f32 {
    normalized_hash_hamming(left.as_bytes(), right.as_bytes())
}

fn normalized_cost(cost_microunits: u64) -> f32 {
    let cost = cost_microunits as f32 / 1_000_000.0;
    cost / (1.0 + cost)
}

fn normalized_latency(latency_ms: u64) -> f32 {
    let latency = latency_ms as f32 / 10_000.0;
    latency / (1.0 + latency)
}

#[derive(Debug, thiserror::Error)]
pub enum AutonomyError {
    #[error("inference field must be nonempty")]
    EmptyInferenceField,
    #[error("invalid unit value: {0}")]
    InvalidUnitValue(&'static str),
    #[error("inference source hash mismatch")]
    InferenceHashMismatch,
    #[error("dual-state twin projection does not match phase envelope")]
    InvalidTwinProjection,
    #[error("observation set must be nonempty")]
    EmptyObservationSet,
    #[error("inference concept label mismatch")]
    ConceptMismatch,
    #[error("invalid fitness weight: {0}")]
    InvalidFitnessWeight(&'static str),
    #[error("invalid autonomy config: {0}")]
    InvalidAutonomyConfig(&'static str),
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
    #[error(transparent)]
    Training(#[from] TrainingError),
}
