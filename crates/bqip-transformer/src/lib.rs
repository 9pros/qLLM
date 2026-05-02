use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use bqip_core::{
    derive_register_id, DualState, InterfaceKind, PhaseEnvelope, Register, RegisterId,
    RegisterLane, REGISTER_BYTES,
};
use memmap2::{MmapMut, MmapOptions};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const MODEL_MAGIC: &[u8; 8] = b"BQIPWGT1";
const MEMORY_MAGIC: &[u8; 8] = b"BQIPMEM1";
const LOG_MAGIC: &[u8; 8] = b"BQIPLOG1";
const LOG_RECORD_MAGIC: &[u8; 8] = b"BQIPLR01";
const CHECKPOINT_MAGIC: &[u8; 8] = b"BQIPCKP1";
const HEADER_BYTES: usize = 48;
const GENESIS_LOG_HASH: [u8; 32] = [0u8; 32];

fn encode_checked<T: Serialize>(magic: &[u8; 8], value: &T) -> Result<Vec<u8>, TransformerError> {
    let payload = bincode::serialize(value)?;
    let hash = blake3::hash(&payload);
    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len());
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(hash.as_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

fn decode_checked<T: DeserializeOwned>(
    magic: &[u8; 8],
    bytes: &[u8],
) -> Result<T, TransformerError> {
    Ok(decode_checked_with_len(magic, bytes)?.0)
}

fn decode_checked_with_len<T: DeserializeOwned>(
    magic: &[u8; 8],
    bytes: &[u8],
) -> Result<(T, usize), TransformerError> {
    if bytes.len() < HEADER_BYTES || &bytes[..8] != magic {
        return Err(TransformerError::InvalidPersistentHeader);
    }

    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&bytes[8..16]);
    let payload_len = u64::from_le_bytes(len_bytes) as usize;
    let required = HEADER_BYTES
        .checked_add(payload_len)
        .ok_or(TransformerError::InvalidPersistentHeader)?;
    if required > bytes.len() {
        return Err(TransformerError::PersistentCapacity {
            capacity: bytes.len(),
            required,
        });
    }

    let payload = &bytes[HEADER_BYTES..required];
    let expected_hash = &bytes[16..48];
    let actual_hash = blake3::hash(payload);
    if expected_hash != actual_hash.as_bytes() {
        return Err(TransformerError::PersistentHashMismatch);
    }

    Ok((bincode::deserialize(payload)?, required))
}

fn decode_mapped_memory(bytes: &[u8]) -> Result<Option<LazyBqipMemory>, TransformerError> {
    if bytes.len() < HEADER_BYTES || bytes[..HEADER_BYTES].iter().all(|byte| *byte == 0) {
        return Ok(None);
    }
    Ok(Some(decode_checked(MEMORY_MAGIC, bytes)?))
}

fn chain_hash(previous: [u8; 32], encoded_record: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&previous);
    hasher.update(encoded_record);
    *hasher.finalize().as_bytes()
}

fn checkpoint_path_for(log_path: &Path) -> PathBuf {
    log_path.with_extension("checkpoint")
}

fn read_checkpoint(path: &Path) -> Result<Option<LazyMemoryCheckpoint>, TransformerError> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(path)?;
    let mut checkpoint = decode_checked::<LazyMemoryCheckpoint>(CHECKPOINT_MAGIC, &bytes)?;
    checkpoint.memory.rebuild_index();
    checkpoint.memory.validate_graph()?;
    Ok(Some(checkpoint))
}

fn write_checkpoint(
    path: &Path,
    checkpoint: &LazyMemoryCheckpoint,
) -> Result<(), TransformerError> {
    let encoded = encode_checked(CHECKPOINT_MAGIC, checkpoint)?;
    let tmp_path = path.with_extension("checkpoint.tmp");
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(&encoded)?;
        file.sync_data()?;
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MixerKind {
    SoftmaxAttention,
    DeltaNet,
    GatedDeltaNet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenStructuralState {
    pub token_id: u32,
    pub position: usize,
    pub register_id: RegisterId,
    pub envelope: PhaseEnvelope,
    pub dual_state: DualState,
    pub memory_node: LazyNodeId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HybridConfig {
    pub vocab_size: usize,
    pub d_model: usize,
    pub max_context: usize,
    pub mixer: MixerKind,
    pub num_heads: usize,
    pub phase_delta: u32,
    pub phase_bucket_size: u32,
    pub delta_step: f32,
    pub forget_floor: f32,
    pub alpha: f32,
    pub beta: f32,
    pub structural_feedback: f32,
    /// Per-head phase deltas (length = num_heads). If empty, uses global phase_delta for all heads.
    pub head_phase_deltas: Vec<u32>,
    /// If true, head_phase_deltas become trainable parameters
    pub learnable_head_delta: bool,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            vocab_size: 4096,
            d_model: 64,
            max_context: 2048,
            mixer: MixerKind::GatedDeltaNet,
            num_heads: 4,
            phase_delta: 2,
            phase_bucket_size: 64,
            delta_step: 0.35,
            forget_floor: 0.02,
            alpha: std::f32::consts::FRAC_1_SQRT_2,
            beta: std::f32::consts::FRAC_1_SQRT_2,
            structural_feedback: 0.08,
            head_phase_deltas: vec![2; 4],  // same as global phase_delta
            learnable_head_delta: false,
        }
    }
}

impl HybridConfig {
    pub fn validate(&self) -> Result<(), TransformerError> {
        if self.vocab_size == 0 {
            return Err(TransformerError::InvalidConfig(
                "vocab_size must be nonzero",
            ));
        }
        if self.d_model == 0 {
            return Err(TransformerError::InvalidConfig("d_model must be nonzero"));
        }
        if self.num_heads == 0 {
            return Err(TransformerError::InvalidConfig("num_heads must be nonzero"));
        }
        if self.d_model % self.num_heads != 0 {
            return Err(TransformerError::InvalidConfig(
                "d_model must be divisible by num_heads",
            ));
        }
        if self.head_phase_deltas.len() != self.num_heads {
            return Err(TransformerError::InvalidConfig(
                "head_phase_deltas length must equal num_heads",
            ));
        }
        for delta in &self.head_phase_deltas {
            if *delta > 64 {
                return Err(TransformerError::InvalidConfig(
                    "head_phase_deltas entries must be <= 64",
                ));
            }
        }
        if self.max_context == 0 {
            return Err(TransformerError::InvalidConfig(
                "max_context must be nonzero",
            ));
        }
        if self.phase_bucket_size == 0 {
            return Err(TransformerError::InvalidConfig(
                "phase_bucket_size must be nonzero",
            ));
        }
        if !(self.delta_step > 0.0 && self.delta_step <= 1.0) || !self.delta_step.is_finite() {
            return Err(TransformerError::InvalidConfig(
                "delta_step must be finite and in (0, 1]",
            ));
        }
        if !(0.0..1.0).contains(&self.forget_floor) || !self.forget_floor.is_finite() {
            return Err(TransformerError::InvalidConfig(
                "forget_floor must be finite and in [0, 1)",
            ));
        }
        PhaseEnvelope::new(1, self.alpha, self.beta, 0)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HybridOutput {
    pub hidden_states: Vec<Vec<f32>>,
    pub structural_states: Vec<TokenStructuralState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenLogit {
    pub token_id: u32,
    pub logit: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DualStatePrediction {
    pub predicted_token_id: u32,
    pub position: usize,
    pub register_id: RegisterId,
    pub envelope: PhaseEnvelope,
    pub dual_state: DualState,
    pub top_logits: Vec<TokenLogit>,
    pub context_output: HybridOutput,
}

impl DualStatePrediction {
    pub fn commit(
        &self,
        memory: &mut impl StructuralMemory,
        phase_delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        memory.insert_with_phase_combining(
            self.register_id,
            self.envelope,
            self.dual_state,
            phase_delta,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelWeights {
    pub config: HybridConfig,
    pub node_public_key: [u8; REGISTER_BYTES],
    pub embeddings: Matrix,
    pub query: Matrix,
    pub key: Matrix,
    pub value: Matrix,
    pub write_gate: Matrix,
    pub forget_gate: Matrix,
    pub output: Matrix,
    pub lm_head: Matrix,
    pub envelope_alpha: Matrix,
    pub envelope_beta: Matrix,
    pub head_phase_deltas: Matrix,
    pub phase_gate_bias: Matrix,  // shape: (1, num_heads)
}

impl ModelWeights {
    pub fn validate(&self) -> Result<(), TransformerError> {
        self.config.validate()?;
        self.embeddings.validate_shape(
            self.config.vocab_size,
            self.config.d_model,
            "embeddings",
        )?;
        self.query
            .validate_shape(self.config.d_model, self.config.d_model, "query")?;
        self.key
            .validate_shape(self.config.d_model, self.config.d_model, "key")?;
        self.value
            .validate_shape(self.config.d_model, self.config.d_model, "value")?;
        self.write_gate
            .validate_shape(self.config.d_model, self.config.d_model, "write_gate")?;
        self.forget_gate
            .validate_shape(self.config.d_model, self.config.d_model, "forget_gate")?;
        self.output
            .validate_shape(self.config.d_model, self.config.d_model, "output")?;
        self.lm_head
            .validate_shape(self.config.vocab_size, self.config.d_model, "lm_head")?;
        self.envelope_alpha
            .validate_shape(1, self.config.d_model, "envelope_alpha")?;
        self.envelope_beta
            .validate_shape(1, self.config.d_model, "envelope_beta")?;
        self.head_phase_deltas
            .validate_shape(self.config.num_heads, 1, "head_phase_deltas")?;
        self.phase_gate_bias
            .validate_shape(1, self.config.num_heads, "phase_gate_bias")?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct HybridTransformer {
    config: HybridConfig,
    node_public_key: [u8; REGISTER_BYTES],
    embeddings: Matrix,
    query: Matrix,
    key: Matrix,
    value: Matrix,
    write_gate: Matrix,
    forget_gate: Matrix,
    output: Matrix,
    lm_head: Matrix,
    envelope_alpha: Matrix,
    envelope_beta: Matrix,
    head_phase_deltas: Matrix,
    phase_gate_bias: Matrix,
}

impl HybridTransformer {
    pub fn new(
        config: HybridConfig,
        node_public_key: [u8; REGISTER_BYTES],
    ) -> Result<Self, TransformerError> {
        config.validate()?;
        let embeddings = Matrix::deterministic(
            config.vocab_size,
            config.d_model,
            b"bqip-transformer:embedding",
        );
        let query = Matrix::deterministic(config.d_model, config.d_model, b"bqip-transformer:q");
        let key = Matrix::deterministic(config.d_model, config.d_model, b"bqip-transformer:k");
        let value = Matrix::deterministic(config.d_model, config.d_model, b"bqip-transformer:v");
        let write_gate = Matrix::deterministic(
            config.d_model,
            config.d_model,
            b"bqip-transformer:write-gate",
        );
        let forget_gate = Matrix::deterministic(
            config.d_model,
            config.d_model,
            b"bqip-transformer:forget-gate",
        );
        let output = Matrix::deterministic(config.d_model, config.d_model, b"bqip-transformer:o");
        let lm_head = Matrix::deterministic(
            config.vocab_size,
            config.d_model,
            b"bqip-transformer:lm-head",
        );
        let envelope_alpha = Matrix::deterministic(
            1,
            config.d_model,
            b"bqip-transformer:env-alpha",
        );
        let envelope_beta = Matrix::deterministic(
            1,
            config.d_model,
            b"bqip-transformer:env-beta",
        );
        let head_phase_deltas = Matrix::from_data(
            config.num_heads,
            1,
            config.head_phase_deltas.iter().map(|&d| d as f32).collect(),
        )?;
        let phase_gate_bias = Matrix::deterministic(
            1,
            config.num_heads,
            b"bqip-transformer:phase-gate-bias",
        );

        Ok(Self {
            config,
            node_public_key,
            embeddings,
            query,
            key,
            value,
            write_gate,
            forget_gate,
            output,
            lm_head,
            envelope_alpha,
            envelope_beta,
            head_phase_deltas,
            phase_gate_bias,
        })
    }

    pub fn from_weights(weights: ModelWeights) -> Result<Self, TransformerError> {
        weights.validate()?;
        Ok(Self {
            config: weights.config,
            node_public_key: weights.node_public_key,
            embeddings: weights.embeddings,
            query: weights.query,
            key: weights.key,
            value: weights.value,
            write_gate: weights.write_gate,
            forget_gate: weights.forget_gate,
            output: weights.output,
            lm_head: weights.lm_head,
            envelope_alpha: weights.envelope_alpha,
            envelope_beta: weights.envelope_beta,
            head_phase_deltas: weights.head_phase_deltas,
            phase_gate_bias: weights.phase_gate_bias,
        })
    }

    pub fn weights(&self) -> ModelWeights {
        ModelWeights {
            config: self.config.clone(),
            node_public_key: self.node_public_key,
            embeddings: self.embeddings.clone(),
            query: self.query.clone(),
            key: self.key.clone(),
            value: self.value.clone(),
            write_gate: self.write_gate.clone(),
            forget_gate: self.forget_gate.clone(),
            output: self.output.clone(),
            lm_head: self.lm_head.clone(),
            envelope_alpha: self.envelope_alpha.clone(),
            envelope_beta: self.envelope_beta.clone(),
            head_phase_deltas: self.head_phase_deltas.clone(),
            phase_gate_bias: self.phase_gate_bias.clone(),
        }
    }

    pub fn predict_envelope(&self, hidden: &[f32]) -> (f32, f32) {
        let alpha_logits = self.envelope_alpha.mul_vec(hidden);
        let beta_logits = self.envelope_beta.mul_vec(hidden);
        // Use sigmoid to get values in (0,1), then normalize to unit circle
        let alpha = 1.0 / (1.0 + (-alpha_logits[0]).exp());
        let beta = 1.0 / (1.0 + (-beta_logits[0]).exp());
        // Normalize so alpha^2 + beta^2 = 1
        let norm = (alpha * alpha + beta * beta).sqrt();
        if norm > 0.0 {
            (alpha / norm, beta / norm)
        } else {
            (std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2)
        }
    }

    pub fn predict_envelope_from_context(
        &self,
        token_ids: &[u32],
        memory: &mut impl StructuralMemory,
    ) -> Result<(f32, f32), TransformerError> {
        let output = self.forward_with_memory(token_ids, memory)?;
        let hidden = output
            .hidden_states
            .last()
            .ok_or(TransformerError::EmptyContext)?;
        Ok(self.predict_envelope(hidden))
    }

    pub fn save_weights(&self, path: impl AsRef<Path>) -> Result<(), TransformerError> {
        let bytes = encode_checked(MODEL_MAGIC, &self.weights())?;
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn load_weights(path: impl AsRef<Path>) -> Result<Self, TransformerError> {
        let bytes = fs::read(path)?;
        let weights = decode_checked::<ModelWeights>(MODEL_MAGIC, &bytes)?;
        Self::from_weights(weights)
    }

    pub fn config(&self) -> &HybridConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut HybridConfig {
        &mut self.config
    }

    pub fn increment_phase_delta(&mut self, inc: u32) {
        self.config.phase_delta = (self.config.phase_delta + inc).min(64);
    }

    pub fn lm_head(&self) -> &Matrix {
        &self.lm_head
    }

    pub fn forward(
        &self,
        token_ids: &[u32],
        memory: &mut LazyBqipMemory,
    ) -> Result<HybridOutput, TransformerError> {
        self.forward_with_memory(token_ids, memory)
    }

    pub fn forward_persistent(
        &self,
        token_ids: &[u32],
        memory: &mut MmapLazyBqipMemory,
    ) -> Result<HybridOutput, TransformerError> {
        self.forward_with_memory(token_ids, memory)
    }

    pub fn forward_logged(
        &self,
        token_ids: &[u32],
        memory: &mut AppendLogLazyBqipMemory,
    ) -> Result<HybridOutput, TransformerError> {
        self.forward_with_memory(token_ids, memory)
    }

    pub fn predict_dual_state(
        &self,
        token_ids: &[u32],
        memory: &mut LazyBqipMemory,
        top_k: usize,
    ) -> Result<DualStatePrediction, TransformerError> {
        self.predict_dual_state_with_memory(token_ids, memory, top_k)
    }

    pub fn predict_dual_state_persistent(
        &self,
        token_ids: &[u32],
        memory: &mut MmapLazyBqipMemory,
        top_k: usize,
    ) -> Result<DualStatePrediction, TransformerError> {
        self.predict_dual_state_with_memory(token_ids, memory, top_k)
    }

    pub fn predict_dual_state_logged(
        &self,
        token_ids: &[u32],
        memory: &mut AppendLogLazyBqipMemory,
        top_k: usize,
    ) -> Result<DualStatePrediction, TransformerError> {
        self.predict_dual_state_with_memory(token_ids, memory, top_k)
    }

    fn forward_with_memory(
        &self,
        token_ids: &[u32],
        memory: &mut impl StructuralMemory,
    ) -> Result<HybridOutput, TransformerError> {
        if token_ids.len() > self.config.max_context {
            return Err(TransformerError::ContextTooLong {
                requested: token_ids.len(),
                max: self.config.max_context,
            });
        }
        if token_ids
            .iter()
            .any(|token| *token as usize >= self.config.vocab_size)
        {
            return Err(TransformerError::TokenOutOfVocabulary);
        }

        let mut inputs = Vec::with_capacity(token_ids.len());
        let mut envelopes = Vec::with_capacity(token_ids.len());

        for (position, token_id) in token_ids.iter().copied().enumerate() {
            let mut embedding = self.embeddings.row(token_id as usize).to_vec();
            apply_position_encoding(&mut embedding, position);
            inputs.push(embedding);
            envelopes.push(self.envelope_for_token(token_id)?);
        }

        let queries = inputs
            .iter()
            .map(|input| self.query.mul_vec(input))
            .collect::<Vec<_>>();
        let keys = inputs
            .iter()
            .map(|input| self.key.mul_vec(input))
            .collect::<Vec<_>>();
        let values = inputs
            .iter()
            .map(|input| self.value.mul_vec(input))
            .collect::<Vec<_>>();
        let write_gates = inputs
            .iter()
            .map(|input| self.write_gate.mul_vec(input))
            .collect::<Vec<_>>();
        let forget_gates = inputs
            .iter()
            .map(|input| self.forget_gate.mul_vec(input))
            .collect::<Vec<_>>();

        let mixed_values = match self.config.mixer {
            MixerKind::SoftmaxAttention => softmax_attention_mix(
                &queries,
                &keys,
                &values,
                &envelopes,
                self.config.phase_delta,
            ),
            MixerKind::DeltaNet => {
                deltanet_mix(&queries, &keys, &values, &envelopes, self.config.delta_step)
            }
            MixerKind::GatedDeltaNet => gated_deltanet_mix(
                &queries,
                &keys,
                &values,
                &write_gates,
                &forget_gates,
                &envelopes,
                self.config.num_heads,
                self.config.delta_step,
                self.config.forget_floor,
                &self.config.head_phase_deltas,
            ),
        };

        let mut hidden_states = Vec::with_capacity(token_ids.len());
        let mut structural_states = Vec::with_capacity(token_ids.len());

        for query_index in 0..token_ids.len() {
            let projected = self.output.mul_vec(&mixed_values[query_index]);
            let mut hidden = inputs[query_index].clone();
            add_scaled(&mut hidden, &projected, 1.0);

            let live = embedding_to_register(token_ids[query_index], query_index, &hidden);
            let dual_state = DualState::from_live(live, envelopes[query_index]);
            let register_id = derive_register_id(
                RegisterLane::GenericEndpoint,
                query_index as u64,
                format!("token:{}:{}", token_ids[query_index], query_index).as_bytes(),
                InterfaceKind::Application,
                &self.node_public_key,
            );

            let memory_node = memory.insert_with_phase_combining(
                register_id,
                envelopes[query_index],
                dual_state,
                self.config.phase_delta,
            )?;
            let materialized = memory.materialize(memory_node)?;
            let feedback = register_to_feedback(materialized.live, self.config.d_model);
            add_scaled(&mut hidden, &feedback, self.config.structural_feedback);

            hidden_states.push(hidden);
            structural_states.push(TokenStructuralState {
                token_id: token_ids[query_index],
                position: query_index,
                register_id,
                envelope: envelopes[query_index],
                dual_state,
                memory_node,
            });
        }

        Ok(HybridOutput {
            hidden_states,
            structural_states,
        })
    }

    fn predict_dual_state_with_memory(
        &self,
        token_ids: &[u32],
        memory: &mut impl StructuralMemory,
        top_k: usize,
    ) -> Result<DualStatePrediction, TransformerError> {
        if token_ids.is_empty() {
            return Err(TransformerError::EmptyContext);
        }

        let context_output = self.forward_with_memory(token_ids, memory)?;
        let last_hidden = context_output
            .hidden_states
            .last()
            .ok_or(TransformerError::EmptyContext)?;
        let top_logits = self.top_logits(last_hidden, top_k.max(1));
        let predicted_token_id = top_logits[0].token_id;
        let position = token_ids.len();
        let mut predicted_hidden = self.embeddings.row(predicted_token_id as usize).to_vec();
        apply_position_encoding(&mut predicted_hidden, position);
        add_scaled(&mut predicted_hidden, last_hidden, 1.0);
        let envelope = self.envelope_for_token(predicted_token_id)?;
        let live = embedding_to_register(predicted_token_id, position, &predicted_hidden);
        let dual_state = DualState::from_live(live, envelope);
        let register_id = derive_register_id(
            RegisterLane::GenericEndpoint,
            position as u64,
            format!("prediction:{}:{}", predicted_token_id, position).as_bytes(),
            InterfaceKind::Application,
            &self.node_public_key,
        );

        Ok(DualStatePrediction {
            predicted_token_id,
            position,
            register_id,
            envelope,
            dual_state,
            top_logits,
            context_output,
        })
    }

    fn top_logits(&self, hidden: &[f32], top_k: usize) -> Vec<TokenLogit> {
        let mut logits = (0..self.config.vocab_size)
            .map(|token_id| TokenLogit {
                token_id: token_id as u32,
                logit: dot(self.lm_head.row(token_id), hidden),
            })
            .collect::<Vec<_>>();
        logits.sort_by(|left, right| {
            right
                .logit
                .total_cmp(&left.logit)
                .then_with(|| left.token_id.cmp(&right.token_id))
        });
        logits.truncate(top_k.min(self.config.vocab_size));
        logits
    }

    fn envelope_for_token(&self, token_id: u32) -> Result<PhaseEnvelope, TransformerError> {
        let bucket = token_id / self.config.phase_bucket_size;
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bqip-transformer:phase");
        hasher.update(&bucket.to_le_bytes());
        let hash = hasher.finalize();
        let mut sig_bytes = [0u8; 8];
        sig_bytes.copy_from_slice(&hash.as_bytes()[..8]);
        Ok(PhaseEnvelope::new(
            u64::from_le_bytes(sig_bytes),
            self.config.alpha,
            self.config.beta,
            0,
        )?)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LazyNodeId(usize);

impl LazyNodeId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LazyNode {
    pub id: LazyNodeId,
    pub register_id: RegisterId,
    pub envelope: PhaseEnvelope,
    pub depth: u32,
    kind: LazyNodeKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum LazyNodeKind {
    Leaf(DualState),
    Xor { left: LazyNodeId, right: LazyNodeId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LazyMemoryLogRecord {
    pub sequence: u64,
    pub previous_hash: [u8; 32],
    pub event: LazyMemoryLogEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LazyMemoryCheckpoint {
    pub record_count: u64,
    pub head_hash: [u8; 32],
    pub memory: LazyBqipMemory,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LazyMemoryLogEvent {
    InsertLeaf {
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
    },
    CombineLazy {
        left: LazyNodeId,
        right: LazyNodeId,
        delta: u32,
    },
    InsertWithPhaseCombining {
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    },
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct LazyBqipMemory {
    nodes: Vec<LazyNode>,
    latest_by_sig: HashMap<u64, LazyNodeId>,
}

pub trait StructuralMemory {
    fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError>;

    fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError>;
}

impl LazyBqipMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: LazyNodeId) -> Option<&LazyNode> {
        self.nodes.get(id.0)
    }

    pub fn insert_leaf(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
    ) -> LazyNodeId {
        let id = LazyNodeId(self.nodes.len());
        self.nodes.push(LazyNode {
            id,
            register_id,
            envelope,
            depth: 0,
            kind: LazyNodeKind::Leaf(state),
        });
        self.latest_by_sig.insert(envelope.coherence_sig, id);
        id
    }

    pub fn combine_lazy(
        &mut self,
        left: LazyNodeId,
        right: LazyNodeId,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        self.combine_inputs_are_valid(left, right, delta)?;
        let left_node = self.require_node(left)?;

        let id = LazyNodeId(self.nodes.len());
        let right_depth = self.require_node(right)?.depth;
        let depth = left_node.depth.max(right_depth) + 1;
        let envelope = left_node.envelope;
        let register_id = left_node.register_id;
        self.nodes.push(LazyNode {
            id,
            register_id,
            envelope,
            depth,
            kind: LazyNodeKind::Xor { left, right },
        });
        self.latest_by_sig.insert(envelope.coherence_sig, id);
        Ok(id)
    }

    pub fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        self.validate_insert_with_phase_combining(envelope, delta)?;
        let previous = self.latest_by_sig.get(&envelope.coherence_sig).copied();
        let leaf = self.insert_leaf(register_id, envelope, state);
        match previous {
            Some(previous) if previous != leaf => self.combine_lazy(previous, leaf, delta),
            _ => Ok(leaf),
        }
    }

    pub fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError> {
        self.require_node(id)?;
        let mut values = vec![None; self.nodes.len()];
        let mut stack = vec![(id, false)];

        while let Some((node_id, expanded)) = stack.pop() {
            if values[node_id.0].is_some() {
                continue;
            }
            let node = self.require_node(node_id)?;
            match node.kind {
                LazyNodeKind::Leaf(state) => values[node_id.0] = Some(state),
                LazyNodeKind::Xor { left, right } if expanded => {
                    let left_state = values[left.0].ok_or(TransformerError::DagEvaluationOrder)?;
                    let right_state =
                        values[right.0].ok_or(TransformerError::DagEvaluationOrder)?;
                    values[node_id.0] = Some(left_state.xor(&right_state));
                }
                LazyNodeKind::Xor { left, right } => {
                    stack.push((node_id, true));
                    stack.push((right, false));
                    stack.push((left, false));
                }
            }
        }

        values[id.0].ok_or(TransformerError::DagEvaluationOrder)
    }

    pub fn latest_for_sig(&self, coherence_sig: u64) -> Option<LazyNodeId> {
        self.latest_by_sig.get(&coherence_sig).copied()
    }

    fn rebuild_index(&mut self) {
        self.latest_by_sig.clear();
        for node in &self.nodes {
            self.latest_by_sig
                .insert(node.envelope.coherence_sig, node.id);
        }
    }

    fn validate_graph(&self) -> Result<(), TransformerError> {
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id.0 != index {
                return Err(TransformerError::InvalidPersistentGraph(
                    "lazy node id does not match storage index",
                ));
            }
            match node.kind {
                LazyNodeKind::Leaf(_) => {}
                LazyNodeKind::Xor { left, right } => {
                    let left_node =
                        self.nodes
                            .get(left.0)
                            .ok_or(TransformerError::InvalidPersistentGraph(
                                "left lazy edge points outside node table",
                            ))?;
                    let right_node =
                        self.nodes
                            .get(right.0)
                            .ok_or(TransformerError::InvalidPersistentGraph(
                                "right lazy edge points outside node table",
                            ))?;
                    if left.0 >= index || right.0 >= index {
                        return Err(TransformerError::InvalidPersistentGraph(
                            "lazy edge must point to an earlier node",
                        ));
                    }
                    if !left_node
                        .envelope
                        .compatible_with(&right_node.envelope, u32::MAX)
                    {
                        return Err(TransformerError::InvalidPersistentGraph(
                            "lazy XOR edge joins incompatible coherence signatures",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_insert_with_phase_combining(
        &self,
        envelope: PhaseEnvelope,
        delta: u32,
    ) -> Result<(), TransformerError> {
        if let Some(previous) = self.latest_by_sig.get(&envelope.coherence_sig).copied() {
            let previous_node = self.require_node(previous)?;
            if !previous_node.envelope.compatible_with(&envelope, delta) {
                return Err(TransformerError::IncompatiblePhase);
            }
        }
        Ok(())
    }

    fn combine_inputs_are_valid(
        &self,
        left: LazyNodeId,
        right: LazyNodeId,
        delta: u32,
    ) -> Result<(), TransformerError> {
        let left_node = self.require_node(left)?;
        let right_node = self.require_node(right)?;
        if !left_node
            .envelope
            .compatible_with(&right_node.envelope, delta)
        {
            return Err(TransformerError::IncompatiblePhase);
        }
        Ok(())
    }

    fn apply_log_event(
        &mut self,
        event: LazyMemoryLogEvent,
    ) -> Result<LazyNodeId, TransformerError> {
        match event {
            LazyMemoryLogEvent::InsertLeaf {
                register_id,
                envelope,
                state,
            } => Ok(self.insert_leaf(register_id, envelope, state)),
            LazyMemoryLogEvent::CombineLazy { left, right, delta } => {
                self.combine_lazy(left, right, delta)
            }
            LazyMemoryLogEvent::InsertWithPhaseCombining {
                register_id,
                envelope,
                state,
                delta,
            } => self.insert_with_phase_combining(register_id, envelope, state, delta),
        }
    }

    fn require_node(&self, id: LazyNodeId) -> Result<&LazyNode, TransformerError> {
        self.nodes
            .get(id.0)
            .ok_or(TransformerError::MissingLazyNode { id: id.0 })
    }
}

impl StructuralMemory for LazyBqipMemory {
    fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        LazyBqipMemory::insert_with_phase_combining(self, register_id, envelope, state, delta)
    }

    fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError> {
        LazyBqipMemory::materialize(self, id)
    }
}

pub struct MmapLazyBqipMemory {
    path: PathBuf,
    capacity: usize,
    mmap: MmapMut,
    memory: LazyBqipMemory,
}

impl MmapLazyBqipMemory {
    pub fn open(path: impl AsRef<Path>, capacity: usize) -> Result<Self, TransformerError> {
        if capacity < HEADER_BYTES {
            return Err(TransformerError::PersistentCapacity {
                capacity,
                required: HEADER_BYTES,
            });
        }

        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        file.set_len(capacity as u64)?;
        let mmap = unsafe { MmapOptions::new().len(capacity).map_mut(&file)? };
        let mut persistent = Self {
            path,
            capacity,
            mmap,
            memory: LazyBqipMemory::new(),
        };

        persistent.memory = match decode_mapped_memory(&persistent.mmap)? {
            Some(mut memory) => {
                memory.rebuild_index();
                memory.validate_graph()?;
                memory
            }
            None => LazyBqipMemory::new(),
        };
        persistent.flush()?;
        Ok(persistent)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> usize {
        self.memory.len()
    }

    pub fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    pub fn node(&self, id: LazyNodeId) -> Option<&LazyNode> {
        self.memory.node(id)
    }

    pub fn insert_leaf(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
    ) -> Result<LazyNodeId, TransformerError> {
        let id = self.memory.insert_leaf(register_id, envelope, state);
        self.flush()?;
        Ok(id)
    }

    pub fn combine_lazy(
        &mut self,
        left: LazyNodeId,
        right: LazyNodeId,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        let id = self.memory.combine_lazy(left, right, delta)?;
        self.flush()?;
        Ok(id)
    }

    pub fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        let id = self
            .memory
            .insert_with_phase_combining(register_id, envelope, state, delta)?;
        self.flush()?;
        Ok(id)
    }

    pub fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError> {
        self.memory.materialize(id)
    }

    pub fn latest_for_sig(&self, coherence_sig: u64) -> Option<LazyNodeId> {
        self.memory.latest_for_sig(coherence_sig)
    }

    pub fn as_memory(&self) -> &LazyBqipMemory {
        &self.memory
    }

    pub fn flush(&mut self) -> Result<(), TransformerError> {
        let encoded = encode_checked(MEMORY_MAGIC, &self.memory)?;
        if encoded.len() > self.capacity {
            return Err(TransformerError::PersistentCapacity {
                capacity: self.capacity,
                required: encoded.len(),
            });
        }
        self.mmap[..encoded.len()].copy_from_slice(&encoded);
        if encoded.len() < self.capacity {
            self.mmap[encoded.len()] = 0;
        }
        self.mmap.flush()?;
        Ok(())
    }
}

impl StructuralMemory for MmapLazyBqipMemory {
    fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        MmapLazyBqipMemory::insert_with_phase_combining(self, register_id, envelope, state, delta)
    }

    fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError> {
        MmapLazyBqipMemory::materialize(self, id)
    }
}

pub struct AppendLogLazyBqipMemory {
    path: PathBuf,
    checkpoint_path: PathBuf,
    file: File,
    memory: LazyBqipMemory,
    record_count: u64,
    head_hash: [u8; 32],
}

impl AppendLogLazyBqipMemory {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TransformerError> {
        let path = path.as_ref().to_path_buf();
        let checkpoint_path = checkpoint_path_for(&path);
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;
        let metadata_len = file.metadata()?.len();

        if metadata_len == 0 {
            file.write_all(LOG_MAGIC)?;
            file.sync_data()?;
        }

        let bytes = fs::read(&path)?;
        let checkpoint = read_checkpoint(&checkpoint_path)?;
        let (memory, record_count, head_hash) = replay_memory_log(&bytes, checkpoint)?;
        Ok(Self {
            path,
            checkpoint_path,
            file,
            memory,
            record_count,
            head_hash,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn checkpoint_path(&self) -> &Path {
        &self.checkpoint_path
    }

    pub fn len(&self) -> usize {
        self.memory.len()
    }

    pub fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    pub fn record_count(&self) -> u64 {
        self.record_count
    }

    pub fn head_hash(&self) -> [u8; 32] {
        self.head_hash
    }

    pub fn node(&self, id: LazyNodeId) -> Option<&LazyNode> {
        self.memory.node(id)
    }

    pub fn insert_leaf(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
    ) -> Result<LazyNodeId, TransformerError> {
        let event = LazyMemoryLogEvent::InsertLeaf {
            register_id,
            envelope,
            state,
        };
        self.append_event(event.clone())?;
        self.memory.apply_log_event(event)
    }

    pub fn combine_lazy(
        &mut self,
        left: LazyNodeId,
        right: LazyNodeId,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        self.memory.combine_inputs_are_valid(left, right, delta)?;
        let event = LazyMemoryLogEvent::CombineLazy { left, right, delta };
        self.append_event(event.clone())?;
        self.memory.apply_log_event(event)
    }

    pub fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        self.memory
            .validate_insert_with_phase_combining(envelope, delta)?;
        let event = LazyMemoryLogEvent::InsertWithPhaseCombining {
            register_id,
            envelope,
            state,
            delta,
        };
        self.append_event(event.clone())?;
        self.memory.apply_log_event(event)
    }

    pub fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError> {
        self.memory.materialize(id)
    }

    pub fn latest_for_sig(&self, coherence_sig: u64) -> Option<LazyNodeId> {
        self.memory.latest_for_sig(coherence_sig)
    }

    pub fn as_memory(&self) -> &LazyBqipMemory {
        &self.memory
    }

    pub fn flush(&mut self) -> Result<(), TransformerError> {
        self.file.flush()?;
        self.file.sync_data()?;
        Ok(())
    }

    pub fn compact(&mut self) -> Result<(), TransformerError> {
        self.flush()?;
        let checkpoint = LazyMemoryCheckpoint {
            record_count: self.record_count,
            head_hash: self.head_hash,
            memory: self.memory.clone(),
        };
        checkpoint.memory.validate_graph()?;
        write_checkpoint(&self.checkpoint_path, &checkpoint)?;

        let mut compacted = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        compacted.write_all(LOG_MAGIC)?;
        compacted.sync_data()?;
        self.file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&self.path)?;
        Ok(())
    }

    fn append_event(&mut self, event: LazyMemoryLogEvent) -> Result<(), TransformerError> {
        let record = LazyMemoryLogRecord {
            sequence: self.record_count,
            previous_hash: self.head_hash,
            event,
        };
        let encoded = encode_checked(LOG_RECORD_MAGIC, &record)?;
        self.file.write_all(&encoded)?;
        self.file.flush()?;
        self.file.sync_data()?;
        self.record_count += 1;
        self.head_hash = chain_hash(self.head_hash, &encoded);
        Ok(())
    }
}

impl StructuralMemory for AppendLogLazyBqipMemory {
    fn insert_with_phase_combining(
        &mut self,
        register_id: RegisterId,
        envelope: PhaseEnvelope,
        state: DualState,
        delta: u32,
    ) -> Result<LazyNodeId, TransformerError> {
        AppendLogLazyBqipMemory::insert_with_phase_combining(
            self,
            register_id,
            envelope,
            state,
            delta,
        )
    }

    fn materialize(&self, id: LazyNodeId) -> Result<DualState, TransformerError> {
        AppendLogLazyBqipMemory::materialize(self, id)
    }
}

fn replay_memory_log(
    bytes: &[u8],
    checkpoint: Option<LazyMemoryCheckpoint>,
) -> Result<(LazyBqipMemory, u64, [u8; 32]), TransformerError> {
    if bytes.len() < LOG_MAGIC.len() || &bytes[..LOG_MAGIC.len()] != LOG_MAGIC {
        return Err(TransformerError::InvalidAppendLog(
            "append log header is invalid",
        ));
    }

    let mut first_record = None;
    if bytes.len() > LOG_MAGIC.len() {
        if bytes.len() - LOG_MAGIC.len() < HEADER_BYTES {
            return Err(TransformerError::InvalidAppendLog(
                "append log ended inside a record header",
            ));
        }
        let (record, _) = decode_checked_with_len::<LazyMemoryLogRecord>(
            LOG_RECORD_MAGIC,
            &bytes[LOG_MAGIC.len()..],
        )?;
        first_record = Some(record);
    }

    let use_checkpoint = match (&checkpoint, &first_record) {
        (Some(_), None) => true,
        (Some(checkpoint), Some(record)) => record.sequence == checkpoint.record_count,
        _ => false,
    };

    let mut memory = if use_checkpoint {
        checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.memory.clone())
            .unwrap_or_default()
    } else {
        LazyBqipMemory::new()
    };
    let mut expected_sequence = if use_checkpoint {
        checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.record_count)
            .unwrap_or(0)
    } else {
        0
    };
    let mut expected_hash = if use_checkpoint {
        checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.head_hash)
            .unwrap_or(GENESIS_LOG_HASH)
    } else {
        GENESIS_LOG_HASH
    };

    if let Some(checkpoint) = &checkpoint {
        checkpoint.memory.validate_graph()?;
        if !use_checkpoint && checkpoint.record_count > 0 {
            match first_record {
                Some(record) if record.sequence == 0 => {}
                Some(record) => {
                    return Err(TransformerError::AppendLogSequence {
                        expected: checkpoint.record_count,
                        actual: record.sequence,
                    });
                }
                None => {}
            }
        }
    }

    let mut offset = LOG_MAGIC.len();
    while offset < bytes.len() {
        if bytes.len() - offset < HEADER_BYTES {
            return Err(TransformerError::InvalidAppendLog(
                "append log ended inside a record header",
            ));
        }
        let (record, consumed) =
            decode_checked_with_len::<LazyMemoryLogRecord>(LOG_RECORD_MAGIC, &bytes[offset..])?;
        if record.sequence != expected_sequence {
            return Err(TransformerError::AppendLogSequence {
                expected: expected_sequence,
                actual: record.sequence,
            });
        }
        if record.previous_hash != expected_hash {
            return Err(TransformerError::AppendLogChainMismatch {
                sequence: record.sequence,
            });
        }
        let encoded_record = &bytes[offset..offset + consumed];
        memory.apply_log_event(record.event)?;
        expected_hash = chain_hash(expected_hash, encoded_record);
        offset += consumed;
        expected_sequence += 1;
    }

    memory.validate_graph()?;
    Ok((memory, expected_sequence, expected_hash))
}

pub fn phase_attention_gate(left: PhaseEnvelope, right: PhaseEnvelope, delta: u32) -> f32 {
    if left.compatible_with(&right, delta) {
        1.0
    } else {
        0.03125
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TransformerError {
    #[error("invalid config: {0}")]
    InvalidConfig(&'static str),
    #[error("invalid weights: {0}")]
    InvalidWeights(&'static str),
    #[error(
        "invalid weight shape for {name}: expected {expected_rows}x{expected_cols}, got {actual_rows}x{actual_cols}"
    )]
    InvalidWeightShape {
        name: &'static str,
        expected_rows: usize,
        expected_cols: usize,
        actual_rows: usize,
        actual_cols: usize,
    },
    #[error("context length {requested} exceeds max_context {max}")]
    ContextTooLong { requested: usize, max: usize },
    #[error("token id is outside configured vocabulary")]
    TokenOutOfVocabulary,
    #[error("prediction context must contain at least one token")]
    EmptyContext,
    #[error("phase envelopes are not compatible")]
    IncompatiblePhase,
    #[error("lazy DAG node {id} does not exist")]
    MissingLazyNode { id: usize },
    #[error("lazy DAG evaluation order was inconsistent")]
    DagEvaluationOrder,
    #[error("persistent region capacity {capacity} bytes cannot hold {required} bytes")]
    PersistentCapacity { capacity: usize, required: usize },
    #[error("persistent region header is invalid")]
    InvalidPersistentHeader,
    #[error("persistent region payload hash mismatch")]
    PersistentHashMismatch,
    #[error("persistent memory graph is invalid: {0}")]
    InvalidPersistentGraph(&'static str),
    #[error("append log is invalid: {0}")]
    InvalidAppendLog(&'static str),
    #[error("append log sequence mismatch: expected {expected}, got {actual}")]
    AppendLogSequence { expected: u64, actual: u64 },
    #[error("append log hash chain mismatch at sequence {sequence}")]
    AppendLogChainMismatch { sequence: u64 },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Codec(#[from] Box<bincode::ErrorKind>),
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f32>,
}

impl Matrix {
    fn deterministic(rows: usize, cols: usize, label: &'static [u8]) -> Self {
        let mut data = Vec::with_capacity(rows * cols);
        let scale = (cols as f32).sqrt().recip();
        for row in 0..rows {
            for col in 0..cols {
                let mut hasher = blake3::Hasher::new();
                hasher.update(label);
                hasher.update(&row.to_le_bytes());
                hasher.update(&col.to_le_bytes());
                let hash = hasher.finalize();
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&hash.as_bytes()[..4]);
                let unit = u32::from_le_bytes(bytes) as f32 / u32::MAX as f32;
                data.push((unit * 2.0 - 1.0) * scale);
            }
        }
        Self { rows, cols, data }
    }

    pub fn from_data(rows: usize, cols: usize, data: Vec<f32>) -> Result<Self, TransformerError> {
        let matrix = Self { rows, cols, data };
        matrix.validate()?;
        Ok(matrix)
    }

    pub fn validate(&self) -> Result<(), TransformerError> {
        if self.rows == 0 || self.cols == 0 {
            return Err(TransformerError::InvalidWeights(
                "matrix dimensions must be nonzero",
            ));
        }
        if self.data.len() != self.rows * self.cols {
            return Err(TransformerError::InvalidWeights(
                "matrix data length does not match rows * cols",
            ));
        }
        if !self.data.iter().all(|value| value.is_finite()) {
            return Err(TransformerError::InvalidWeights(
                "matrix contains non-finite values",
            ));
        }
        Ok(())
    }

    pub fn validate_shape(
        &self,
        rows: usize,
        cols: usize,
        name: &'static str,
    ) -> Result<(), TransformerError> {
        self.validate()?;
        if self.rows != rows || self.cols != cols {
            return Err(TransformerError::InvalidWeightShape {
                name,
                expected_rows: rows,
                expected_cols: cols,
                actual_rows: self.rows,
                actual_cols: self.cols,
            });
        }
        Ok(())
    }

    pub fn row(&self, row: usize) -> &[f32] {
        let start = row * self.cols;
        &self.data[start..start + self.cols]
    }

    fn mul_vec(&self, input: &[f32]) -> Vec<f32> {
        debug_assert_eq!(input.len(), self.cols);
        let mut out = vec![0.0f32; self.rows];
        for row in 0..self.rows {
            out[row] = dot(self.row(row), input);
        }
        out
    }
}

fn apply_position_encoding(vector: &mut [f32], position: usize) {
    let dimensions = vector.len() as f32;
    for (dimension, value) in vector.iter_mut().enumerate() {
        let denominator = 10000.0f32.powf((dimension as f32) / dimensions);
        let angle = position as f32 / denominator;
        *value += if dimension % 2 == 0 {
            angle.sin()
        } else {
            angle.cos()
        };
    }
}

fn embedding_to_register(token_id: u32, position: usize, hidden: &[f32]) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-transformer:live-register");
    hasher.update(&token_id.to_le_bytes());
    hasher.update(&position.to_le_bytes());
    for value in hidden {
        hasher.update(&value.to_bits().to_le_bytes());
    }
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn register_to_feedback(register: Register, d_model: usize) -> Vec<f32> {
    let bytes = register.as_bytes();
    (0..d_model)
        .map(|index| {
            let byte = bytes[index % REGISTER_BYTES] as f32;
            byte / 127.5 - 1.0
        })
        .collect()
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn add_scaled(target: &mut [f32], source: &[f32], scale: f32) {
    debug_assert_eq!(target.len(), source.len());
    for (target, source) in target.iter_mut().zip(source) {
        *target += *source * scale;
    }
}

fn softmax_attention_mix(
    queries: &[Vec<f32>],
    keys: &[Vec<f32>],
    values: &[Vec<f32>],
    envelopes: &[PhaseEnvelope],
    phase_delta: u32,
) -> Vec<Vec<f32>> {
    let d_model = queries.first().map_or(0, Vec::len);
    let scale = (d_model as f32).sqrt().recip();
    let mut mixed = Vec::with_capacity(queries.len());

    for query_index in 0..queries.len() {
        let mut scores = Vec::with_capacity(keys.len());
        for key_index in 0..keys.len() {
            let raw = dot(&queries[query_index], &keys[key_index]) * scale;
            let gate =
                phase_attention_gate(envelopes[query_index], envelopes[key_index], phase_delta);
            scores.push(raw + gate.ln());
        }

        let weights = softmax(&scores);
        let mut attended = vec![0.0f32; d_model];
        for (weight, value) in weights.iter().zip(values) {
            add_scaled(&mut attended, value, *weight);
        }
        mixed.push(attended);
    }

    mixed
}

fn deltanet_mix(
    queries: &[Vec<f32>],
    keys: &[Vec<f32>],
    values: &[Vec<f32>],
    envelopes: &[PhaseEnvelope],
    delta_step: f32,
) -> Vec<Vec<f32>> {
    let d_model = queries.first().map_or(0, Vec::len);
    let mut phase_banks: HashMap<u64, Vec<f32>> = HashMap::new();
    let mut outputs = Vec::with_capacity(queries.len());

    for index in 0..queries.len() {
        let query = normalized(&queries[index]);
        let key = normalized(&keys[index]);
        let value = &values[index];
        let bank = phase_banks
            .entry(envelopes[index].coherence_sig)
            .or_insert_with(|| vec![0.0; d_model * d_model]);

        let prediction = mat_vec(bank, &key, d_model);
        let mut error = value.clone();
        add_scaled(&mut error, &prediction, -1.0);
        outer_update(bank, &error, &key, delta_step, d_model);
        outputs.push(mat_vec(bank, &query, d_model));
    }

    outputs
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FastWeightBankKey {
    head: usize,
    coherence_sig: u64,
}

fn gated_deltanet_mix(
    queries: &[Vec<f32>],
    keys: &[Vec<f32>],
    values: &[Vec<f32>],
    write_gates: &[Vec<f32>],
    forget_gates: &[Vec<f32>],
    envelopes: &[PhaseEnvelope],
    num_heads: usize,
    delta_step: f32,
    forget_floor: f32,
    head_phase_deltas: &[u32],
) -> Vec<Vec<f32>> {
    let d_model = queries.first().map_or(0, Vec::len);
    let head_dim = d_model / num_heads;
    let mut phase_banks: HashMap<FastWeightBankKey, Vec<f32>> = HashMap::new();
    let mut outputs = Vec::with_capacity(queries.len());

    for index in 0..queries.len() {
        let mut token_output = vec![0.0f32; d_model];
        for head in 0..num_heads {
            let start = head * head_dim;
            let end = start + head_dim;
            let query = normalized(&queries[index][start..end]);
            let key = normalized(&keys[index][start..end]);
            let value = &values[index][start..end];
            let write_gate = head_gate(&write_gates[index], head, head_dim);
            let forget_gate = forget_floor
                + (1.0 - forget_floor) * head_gate(&forget_gates[index], head, head_dim);
            
            // Per-head phase delta: coarse-grain coherence signature
            let head_delta = head_phase_deltas[head];
            let coherence_sig = envelopes[index].coherence_sig;
            let coarse_sig = if head_delta >= 64 {
                0
            } else {
                coherence_sig >> head_delta
            };
            let bank_key = FastWeightBankKey {
                head,
                coherence_sig: coarse_sig,
            };
            let bank = phase_banks
                .entry(bank_key)
                .or_insert_with(|| vec![0.0; head_dim * head_dim]);

            for weight in bank.iter_mut() {
                *weight *= forget_gate;
            }

            let prediction = mat_vec(bank, &key, head_dim);
            let mut error = value.to_vec();
            add_scaled(&mut error, &prediction, -1.0);
            outer_update(bank, &error, &key, delta_step * write_gate, head_dim);

            let head_output = mat_vec(bank, &query, head_dim);
            token_output[start..end].copy_from_slice(&head_output);
        }
        outputs.push(token_output);
    }

    outputs
}

fn head_gate(gates: &[f32], head: usize, head_dim: usize) -> f32 {
    let start = head * head_dim;
    let end = start + head_dim;
    let mean = gates[start..end].iter().sum::<f32>() / head_dim as f32;
    sigmoid(mean)
}

fn sigmoid(value: f32) -> f32 {
    if value >= 0.0 {
        let exp = (-value).exp();
        1.0 / (1.0 + exp)
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

fn normalized(vector: &[f32]) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return vec![0.0; vector.len()];
    }
    vector.iter().map(|value| value / norm).collect()
}

fn mat_vec(matrix: &[f32], vector: &[f32], width: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; width];
    for row in 0..width {
        let start = row * width;
        out[row] = dot(&matrix[start..start + width], vector);
    }
    out
}

fn outer_update(matrix: &mut [f32], error: &[f32], key: &[f32], step: f32, width: usize) {
    for row in 0..width {
        for col in 0..width {
            matrix[row * width + col] += step * error[row] * key[col];
        }
    }
}

fn softmax(scores: &[f32]) -> Vec<f32> {
    let max = scores
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, |acc, score| acc.max(score));
    let mut exp = scores
        .iter()
        .map(|score| (*score - max).exp())
        .collect::<Vec<_>>();
    let sum = exp.iter().sum::<f32>();
    for value in &mut exp {
        *value /= sum;
    }
    exp
}

#[cfg(test)]
mod tests {
    use super::*;
    use bqip_core::{phase_project, InterfaceKind, RegisterLane};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn config() -> HybridConfig {
        HybridConfig {
            vocab_size: 256,
            d_model: 32,
            max_context: 32,
            mixer: MixerKind::GatedDeltaNet,
            num_heads: 4,
            phase_delta: 1,
            phase_bucket_size: 16,
            delta_step: 0.4,
            forget_floor: 0.05,
            alpha: std::f32::consts::FRAC_1_SQRT_2,
            beta: std::f32::consts::FRAC_1_SQRT_2,
            structural_feedback: 0.05,
        }
    }

    fn register_id(index: u64) -> RegisterId {
        derive_register_id(
            RegisterLane::GenericEndpoint,
            index,
            b"test",
            InterfaceKind::Application,
            &[1u8; REGISTER_BYTES],
        )
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bqip-transformer-{label}-{}-{nanos}.bin",
            std::process::id()
        ))
    }

    #[test]
    fn lazy_memory_matches_eager_xor() {
        let envelope = PhaseEnvelope::balanced(99);
        let mut memory = LazyBqipMemory::new();
        let states = (0..8)
            .map(|index| DualState::deterministic(&[index], envelope))
            .collect::<Vec<_>>();

        let mut latest = None;
        for (index, state) in states.iter().copied().enumerate() {
            latest = Some(
                memory
                    .insert_with_phase_combining(register_id(index as u64), envelope, state, 0)
                    .unwrap(),
            );
        }

        let eager = states
            .iter()
            .fold(DualState::zero(), |acc, state| acc.xor(state));
        let lazy = memory.materialize(latest.unwrap()).unwrap();
        assert_eq!(lazy, eager);
    }

    #[test]
    fn incompatible_phase_is_rejected_for_lazy_combine() {
        let env_a = PhaseEnvelope::balanced(1);
        let env_b = PhaseEnvelope::balanced(2);
        let mut memory = LazyBqipMemory::new();
        let left = memory.insert_leaf(register_id(0), env_a, DualState::deterministic(b"a", env_a));
        let right =
            memory.insert_leaf(register_id(1), env_b, DualState::deterministic(b"b", env_b));

        assert!(matches!(
            memory.combine_lazy(left, right, 0),
            Err(TransformerError::IncompatiblePhase)
        ));
    }

    #[test]
    fn phase_gate_prefers_compatible_attention() {
        let env_a = PhaseEnvelope::balanced(123);
        let env_b = PhaseEnvelope::balanced(123);
        let env_c = PhaseEnvelope::balanced(456);

        assert_eq!(phase_attention_gate(env_a, env_b, 0), 1.0);
        assert!(phase_attention_gate(env_a, env_c, 0) < 0.05);
    }

    #[test]
    fn all_mixers_execute_and_diverge() {
        let mut softmax_config = config();
        softmax_config.mixer = MixerKind::SoftmaxAttention;
        let mut delta_config = config();
        delta_config.mixer = MixerKind::DeltaNet;
        let softmax_model = HybridTransformer::new(softmax_config, [8u8; REGISTER_BYTES]).unwrap();
        let delta_model = HybridTransformer::new(delta_config, [8u8; REGISTER_BYTES]).unwrap();
        let gated_model = HybridTransformer::new(config(), [8u8; REGISTER_BYTES]).unwrap();
        let tokens = [11, 12, 13, 40];

        let mut softmax_memory = LazyBqipMemory::new();
        let mut delta_memory = LazyBqipMemory::new();
        let mut gated_memory = LazyBqipMemory::new();
        let softmax = softmax_model.forward(&tokens, &mut softmax_memory).unwrap();
        let delta = delta_model.forward(&tokens, &mut delta_memory).unwrap();
        let gated = gated_model.forward(&tokens, &mut gated_memory).unwrap();

        assert_eq!(softmax.hidden_states.len(), tokens.len());
        assert_eq!(delta.hidden_states.len(), tokens.len());
        assert_eq!(gated.hidden_states.len(), tokens.len());
        assert_ne!(softmax.hidden_states, delta.hidden_states);
        assert_ne!(delta.hidden_states, gated.hidden_states);
    }

    #[test]
    fn invalid_head_partition_is_rejected() {
        let mut bad = config();
        bad.num_heads = 3;

        assert!(matches!(
            HybridTransformer::new(bad, [8u8; REGISTER_BYTES]),
            Err(TransformerError::InvalidConfig(
                "d_model must be divisible by num_heads"
            ))
        ));
    }

    #[test]
    fn gated_deltanet_keeps_phase_banks_separate_per_head() {
        let env_a = PhaseEnvelope::balanced(1);
        let env_b = PhaseEnvelope::balanced(2);
        let queries = vec![vec![0.2, 0.4, -0.3, 0.9], vec![0.5, -0.1, 0.7, 0.2]];
        let keys = vec![vec![0.1, 0.8, 0.2, -0.4], vec![0.6, 0.3, -0.7, 0.5]];
        let values = vec![vec![0.9, -0.2, 0.3, 0.4], vec![0.2, 0.6, -0.5, 0.8]];
        let gates = vec![vec![0.0; 4], vec![0.0; 4]];

        let cross_phase = gated_deltanet_mix(
            &queries,
            &keys,
            &values,
            &gates,
            &gates,
            &[env_a, env_b],
            2,
            0.5,
            0.0,
        );
        let single_phase = gated_deltanet_mix(
            &[queries[1].clone()],
            &[keys[1].clone()],
            &[values[1].clone()],
            &[gates[1].clone()],
            &[gates[1].clone()],
            &[env_b],
            2,
            0.5,
            0.0,
        );
        let same_phase = gated_deltanet_mix(
            &queries,
            &keys,
            &values,
            &gates,
            &gates,
            &[env_b, env_b],
            2,
            0.5,
            0.0,
        );

        assert_eq!(cross_phase[1], single_phase[0]);
        assert_ne!(same_phase[1], single_phase[0]);
    }

    #[test]
    fn forward_records_structural_memory() {
        let model = HybridTransformer::new(config(), [9u8; REGISTER_BYTES]).unwrap();
        let mut memory = LazyBqipMemory::new();
        let output = model.forward(&[3, 4, 5, 19, 20], &mut memory).unwrap();

        assert_eq!(output.hidden_states.len(), 5);
        assert_eq!(output.structural_states.len(), 5);
        assert!(memory.len() >= 5);
        for hidden in output.hidden_states {
            assert_eq!(hidden.len(), model.config().d_model);
            assert!(hidden.iter().all(|value| value.is_finite()));
        }
        for state in output.structural_states {
            assert_eq!(
                state.dual_state.twin,
                phase_project(state.dual_state.live, state.envelope)
            );
            assert!(memory.node(state.memory_node).is_some());
        }
    }

    #[test]
    fn forward_is_deterministic_for_same_memory_history() {
        let model = HybridTransformer::new(config(), [3u8; REGISTER_BYTES]).unwrap();
        let tokens = [7, 8, 9, 10];

        let mut left_memory = LazyBqipMemory::new();
        let mut right_memory = LazyBqipMemory::new();
        let left = model.forward(&tokens, &mut left_memory).unwrap();
        let right = model.forward(&tokens, &mut right_memory).unwrap();

        assert_eq!(left.hidden_states, right.hidden_states);
        assert_eq!(left_memory.len(), right_memory.len());
    }

    #[test]
    fn model_weights_round_trip_preserves_forward_output() {
        let path = temp_path("weights");
        let model = HybridTransformer::new(config(), [4u8; REGISTER_BYTES]).unwrap();
        model.save_weights(&path).unwrap();
        let loaded = HybridTransformer::load_weights(&path).unwrap();

        let mut left_memory = LazyBqipMemory::new();
        let mut right_memory = LazyBqipMemory::new();
        let tokens = [4, 8, 12, 16, 20];
        let left = model.forward(&tokens, &mut left_memory).unwrap();
        let right = loaded.forward(&tokens, &mut right_memory).unwrap();

        assert_eq!(model.weights(), loaded.weights());
        assert_eq!(left.hidden_states, right.hidden_states);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn mmap_memory_reopens_and_materializes_same_lazy_state() {
        let path = temp_path("memory");
        let envelope = PhaseEnvelope::balanced(77);
        let states = (0..6)
            .map(|index| DualState::deterministic(&[index], envelope))
            .collect::<Vec<_>>();
        let eager = states
            .iter()
            .fold(DualState::zero(), |acc, state| acc.xor(state));

        let latest = {
            let mut memory = MmapLazyBqipMemory::open(&path, 64 * 1024).unwrap();
            let mut latest = None;
            for (index, state) in states.iter().copied().enumerate() {
                latest = Some(
                    memory
                        .insert_with_phase_combining(register_id(index as u64), envelope, state, 0)
                        .unwrap(),
                );
            }
            assert_eq!(memory.materialize(latest.unwrap()).unwrap(), eager);
            latest.unwrap()
        };

        let reopened = MmapLazyBqipMemory::open(&path, 64 * 1024).unwrap();
        assert_eq!(reopened.len(), 11);
        assert_eq!(
            reopened.latest_for_sig(envelope.coherence_sig),
            Some(latest)
        );
        assert_eq!(reopened.materialize(latest).unwrap(), eager);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn append_log_reopens_and_materializes_same_lazy_state() {
        let path = temp_path("append-log");
        let envelope = PhaseEnvelope::balanced(177);
        let states = (0..6)
            .map(|index| DualState::deterministic(&[index], envelope))
            .collect::<Vec<_>>();
        let eager = states
            .iter()
            .fold(DualState::zero(), |acc, state| acc.xor(state));

        let latest = {
            let mut memory = AppendLogLazyBqipMemory::open(&path).unwrap();
            let mut latest = None;
            for (index, state) in states.iter().copied().enumerate() {
                latest = Some(
                    memory
                        .insert_with_phase_combining(register_id(index as u64), envelope, state, 0)
                        .unwrap(),
                );
            }
            assert_eq!(memory.record_count(), 6);
            assert_eq!(memory.len(), 11);
            assert_eq!(memory.materialize(latest.unwrap()).unwrap(), eager);
            latest.unwrap()
        };

        let reopened = AppendLogLazyBqipMemory::open(&path).unwrap();
        assert_eq!(reopened.record_count(), 6);
        assert_eq!(reopened.len(), 11);
        assert_eq!(
            reopened.latest_for_sig(envelope.coherence_sig),
            Some(latest)
        );
        assert_eq!(reopened.materialize(latest).unwrap(), eager);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn append_log_compacts_to_checkpoint_and_continues_chain() {
        let path = temp_path("append-log-compact");
        let checkpoint_path = checkpoint_path_for(&path);
        let envelope = PhaseEnvelope::balanced(277);
        let states = (0..10)
            .map(|index| DualState::deterministic(&[index], envelope))
            .collect::<Vec<_>>();

        {
            let mut memory = AppendLogLazyBqipMemory::open(&path).unwrap();
            for (index, state) in states.iter().take(8).copied().enumerate() {
                memory
                    .insert_with_phase_combining(register_id(index as u64), envelope, state, 0)
                    .unwrap();
            }
            let pre_compact_hash = memory.head_hash();
            memory.compact().unwrap();
            assert_eq!(memory.record_count(), 8);
            assert_eq!(memory.head_hash(), pre_compact_hash);
            assert_eq!(fs::metadata(&path).unwrap().len(), LOG_MAGIC.len() as u64);
            assert!(checkpoint_path.exists());
        }

        let latest = {
            let mut memory = AppendLogLazyBqipMemory::open(&path).unwrap();
            assert_eq!(memory.record_count(), 8);
            assert_eq!(memory.len(), 15);
            let mut latest = None;
            for (index, state) in states.iter().copied().enumerate().skip(8) {
                latest = Some(
                    memory
                        .insert_with_phase_combining(register_id(index as u64), envelope, state, 0)
                        .unwrap(),
                );
            }
            assert_eq!(memory.record_count(), 10);
            latest.unwrap()
        };

        let reopened = AppendLogLazyBqipMemory::open(&path).unwrap();
        let eager = states
            .iter()
            .fold(DualState::zero(), |acc, state| acc.xor(state));
        assert_eq!(reopened.record_count(), 10);
        assert_eq!(reopened.len(), 19);
        assert_eq!(reopened.materialize(latest).unwrap(), eager);

        fs::remove_file(path).unwrap();
        fs::remove_file(checkpoint_path).unwrap();
    }

    #[test]
    fn forward_persistent_writes_reopenable_memory() {
        let path = temp_path("forward-persistent");
        let model = HybridTransformer::new(config(), [5u8; REGISTER_BYTES]).unwrap();
        let last_node = {
            let mut memory = MmapLazyBqipMemory::open(&path, 128 * 1024).unwrap();
            let output = model
                .forward_persistent(&[6, 7, 8, 24, 25, 26], &mut memory)
                .unwrap();
            assert_eq!(output.hidden_states.len(), 6);
            output.structural_states.last().unwrap().memory_node
        };

        let reopened = MmapLazyBqipMemory::open(&path, 128 * 1024).unwrap();
        assert!(reopened.len() >= 6);
        assert!(reopened.materialize(last_node).unwrap().live != Register::zero());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn forward_logged_writes_reopenable_replay_log() {
        let path = temp_path("forward-logged");
        let model = HybridTransformer::new(config(), [15u8; REGISTER_BYTES]).unwrap();
        let last_node = {
            let mut memory = AppendLogLazyBqipMemory::open(&path).unwrap();
            let output = model
                .forward_logged(&[6, 7, 8, 24, 25, 26], &mut memory)
                .unwrap();
            assert_eq!(output.hidden_states.len(), 6);
            assert_eq!(memory.record_count(), 6);
            output.structural_states.last().unwrap().memory_node
        };

        let reopened = AppendLogLazyBqipMemory::open(&path).unwrap();
        assert!(reopened.len() >= 6);
        assert_eq!(reopened.record_count(), 6);
        assert!(reopened.materialize(last_node).unwrap().live != Register::zero());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn dual_state_prediction_is_deterministic_and_phase_projected() {
        let model = HybridTransformer::new(config(), [25u8; REGISTER_BYTES]).unwrap();
        let tokens = [3, 5, 8, 13];
        let mut left_memory = LazyBqipMemory::new();
        let mut right_memory = LazyBqipMemory::new();

        let left = model
            .predict_dual_state(&tokens, &mut left_memory, 5)
            .unwrap();
        let right = model
            .predict_dual_state(&tokens, &mut right_memory, 5)
            .unwrap();

        assert_eq!(left.predicted_token_id, right.predicted_token_id);
        assert_eq!(left.position, tokens.len());
        assert_eq!(left.top_logits.len(), 5);
        assert_eq!(left.dual_state, right.dual_state);
        assert_eq!(
            left.dual_state.twin,
            phase_project(left.dual_state.live, left.envelope)
        );
        assert!(left
            .top_logits
            .windows(2)
            .all(|pair| pair[0].logit >= pair[1].logit));
    }

    #[test]
    fn dual_state_prediction_rejects_empty_context() {
        let model = HybridTransformer::new(config(), [26u8; REGISTER_BYTES]).unwrap();
        let mut memory = LazyBqipMemory::new();

        assert!(matches!(
            model.predict_dual_state(&[], &mut memory, 4),
            Err(TransformerError::EmptyContext)
        ));
        assert!(memory.is_empty());
    }

    #[test]
    fn dual_state_prediction_commit_replays_from_append_log() {
        let path = temp_path("prediction-commit");
        let model = HybridTransformer::new(config(), [27u8; REGISTER_BYTES]).unwrap();
        let committed_node = {
            let mut memory = AppendLogLazyBqipMemory::open(&path).unwrap();
            let prediction = model
                .predict_dual_state_logged(&[9, 10, 11], &mut memory, 3)
                .unwrap();
            let before_commit = memory.record_count();
            let committed = prediction
                .commit(&mut memory, model.config().phase_delta)
                .unwrap();
            assert_eq!(memory.record_count(), before_commit + 1);
            assert_eq!(
                memory.materialize(committed).unwrap(),
                prediction.dual_state
            );
            committed
        };

        let reopened = AppendLogLazyBqipMemory::open(&path).unwrap();
        assert_eq!(reopened.record_count(), 4);
        assert!(reopened.materialize(committed_node).unwrap().live != Register::zero());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn mmap_memory_rejects_hash_mismatch() {
        let path = temp_path("corrupt-memory");
        let envelope = PhaseEnvelope::balanced(88);
        {
            let mut memory = MmapLazyBqipMemory::open(&path, 64 * 1024).unwrap();
            memory
                .insert_with_phase_combining(
                    register_id(0),
                    envelope,
                    DualState::deterministic(b"corrupt", envelope),
                    0,
                )
                .unwrap();
        }

        let mut bytes = fs::read(&path).unwrap();
        bytes[HEADER_BYTES] ^= 0xff;
        fs::write(&path, bytes).unwrap();

        assert!(matches!(
            MmapLazyBqipMemory::open(&path, 64 * 1024),
            Err(TransformerError::PersistentHashMismatch)
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn append_log_rejects_hash_mismatch() {
        let path = temp_path("corrupt-log");
        let envelope = PhaseEnvelope::balanced(188);
        {
            let mut memory = AppendLogLazyBqipMemory::open(&path).unwrap();
            memory
                .insert_with_phase_combining(
                    register_id(0),
                    envelope,
                    DualState::deterministic(b"corrupt-log", envelope),
                    0,
                )
                .unwrap();
        }

        let mut bytes = fs::read(&path).unwrap();
        bytes[LOG_MAGIC.len() + HEADER_BYTES] ^= 0xff;
        fs::write(&path, bytes).unwrap();

        assert!(matches!(
            AppendLogLazyBqipMemory::open(&path),
            Err(TransformerError::PersistentHashMismatch)
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn append_log_rejects_sequence_gap() {
        let path = temp_path("sequence-gap");
        let envelope = PhaseEnvelope::balanced(199);
        let record = LazyMemoryLogRecord {
            sequence: 1,
            previous_hash: GENESIS_LOG_HASH,
            event: LazyMemoryLogEvent::InsertWithPhaseCombining {
                register_id: register_id(0),
                envelope,
                state: DualState::deterministic(b"sequence-gap", envelope),
                delta: 0,
            },
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(LOG_MAGIC);
        bytes.extend_from_slice(&encode_checked(LOG_RECORD_MAGIC, &record).unwrap());
        fs::write(&path, bytes).unwrap();

        assert!(matches!(
            AppendLogLazyBqipMemory::open(&path),
            Err(TransformerError::AppendLogSequence {
                expected: 0,
                actual: 1
            })
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn append_log_rejects_valid_record_with_wrong_chain_hash() {
        let path = temp_path("wrong-chain");
        let envelope = PhaseEnvelope::balanced(299);
        let first = LazyMemoryLogRecord {
            sequence: 0,
            previous_hash: GENESIS_LOG_HASH,
            event: LazyMemoryLogEvent::InsertWithPhaseCombining {
                register_id: register_id(0),
                envelope,
                state: DualState::deterministic(b"first", envelope),
                delta: 0,
            },
        };
        let first_encoded = encode_checked(LOG_RECORD_MAGIC, &first).unwrap();
        let second = LazyMemoryLogRecord {
            sequence: 1,
            previous_hash: [9u8; 32],
            event: LazyMemoryLogEvent::InsertWithPhaseCombining {
                register_id: register_id(1),
                envelope,
                state: DualState::deterministic(b"second", envelope),
                delta: 0,
            },
        };

        let mut bytes = Vec::new();
        bytes.extend_from_slice(LOG_MAGIC);
        bytes.extend_from_slice(&first_encoded);
        bytes.extend_from_slice(&encode_checked(LOG_RECORD_MAGIC, &second).unwrap());
        fs::write(&path, bytes).unwrap();

        assert!(matches!(
            AppendLogLazyBqipMemory::open(&path),
            Err(TransformerError::AppendLogChainMismatch { sequence: 1 })
        ));
        fs::remove_file(path).unwrap();
    }
}
