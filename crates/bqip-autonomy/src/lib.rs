// Autonomous Experience Engine
// 
// A living model that gains experience through self-sustaining simulation
// without gradient training or backpropagation.

use std::collections::{HashMap, VecDeque, BinaryHeap};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use bqip_core::{
    derive_register_id, DualState, InterfaceKind, PhaseEnvelope, Register, RegisterId,
    RegisterLane, REGISTER_BYTES, phase_project,
};
use bqip_evolver::{CandidateGenome, EvolverError, FitnessTarget, MetaGaEvolver, MetaGaPolicy};
use bqip_transformer::{HybridConfig, HybridTransformer, LazyBqipMemory};
use serde::{Deserialize, Serialize};

// Additional imports for autonomous learning interface
use bqip_training::{
    ConceptTokenCompiler, HybridTrainer, MetricsLedger, SplitRatios, TokenizedDocument,
    TrainConfig, TrainingCorpus, SplitTrainedModel, TrainingError,
};
use bqip_multimodal::MultimodalGrounding;
use bqip_transformer::TransformerError;
use bqip_core::CoreError;

/// Autonomous Experience Engine - a living model
pub struct AutonomousExperienceEngine {
    pub consciousness: LivingModel,
    pub meta_observer: MetaObserver,
    pub inner_arena: InnerThoughtArena,
    pub experience_memory: ExperienceMemory,
    pub lifecycle: LifeCycleState,
    pub config: EngineConfig,
    pub stats: EngineStats,
}

#[derive(Clone)]
pub struct LivingModel {
    pub transformer: HybridTransformer,
    pub envelope: PhaseEnvelope,
    pub memory: LazyBqipMemory,
    pub coherence_sig: u64,
    pub vitality: f32,
    pub age: u64,
}

pub struct MetaObserver {
    pub ga_evolver: MetaGaEvolver,
    pub simulation_pool: Vec<SimulatedExperience>,
    pub strategy: GuidanceStrategy,
}

pub struct InnerThoughtArena {
    pub agents: Vec<InnerAgent>,
    pub dominant_agent: usize,
    pub debate_history: VecDeque<DebateResult>,
    pub time_scales: Vec<f32>,
}

pub struct InnerAgent {
    pub id: usize,
    pub role: AgentRole,
    pub confidence: f32,
    pub perspective: PhaseEnvelope,
    pub score: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AgentRole {
    Explorer,
    Critic,
    Synthesizer,
    MemoryKeeper,
    Futurist,
}

#[derive(Clone)]
pub struct Argument {
    pub agent_id: usize,
    pub content: Vec<f32>,
    pub persuasiveness: f32,
    pub coherence: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: u64,
    pub experience_type: ExperienceType,
    pub content: Vec<f32>,
    pub valence: f32,
    pub arousal: f32,
    pub coherence_sig: u64,
    pub timestamp: u64,
    pub envelope_delta: Option<PhaseEnvelope>,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum ExperienceType {
    Discovery,
    Conflict,
    Synthesis,
    Reflection,
    Insight,
}

pub struct ExperienceMemory {
    pub experiences: Vec<Experience>,
    pub episodic: VecDeque<Experience>,
    pub clusters: HashMap<u64, Vec<usize>>,
    pub replay_schedule: VecDeque<ReplayTask>,
    pub max_size: usize,
}

#[derive(Clone)]
struct ReplayTask {
    experience_id: u64,
    priority: f32,
    scheduled_time: u64,
}

#[derive(Clone, PartialEq)]
pub enum LifeCycleState {
    Birth { timestamp: u64, initial_envelope: PhaseEnvelope },
    Experience { started: u64, experiences_count: u64 },
    Reflection { started: u64, experiences_reviewed: u64 },
    Evolution { started: u64, candidates_evaluated: u64 },
    Rebirth { previous_envelope: PhaseEnvelope, new_envelope: PhaseEnvelope, continuity_score: f32 },
}

#[derive(Clone)]
pub struct EngineConfig {
    pub inner_agents_count: usize,
    pub debate_frequency: u64,
    pub replay_frequency: u64,
    pub evolution_frequency: u64,
    pub rebirth_threshold: f32,
    pub max_memory_size: usize,
    pub time_base_scale: f32,
    pub exploration_rate: f32,
    pub insight_coherence_threshold: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            inner_agents_count: 5,
            debate_frequency: 5,
            replay_frequency: 50,
            evolution_frequency: 20,
            rebirth_threshold: 0.3,
            max_memory_size: 10000,
            time_base_scale: 1.0,
            exploration_rate: 0.4,
            insight_coherence_threshold: 0.8,
        }
    }
}

#[derive(Default)]
pub struct EngineStats {
    pub total_experiences: u64,
    pub total_debates: u64,
    pub total_insights: u64,
    pub total_evolutions: u64,
    pub total_rebirths: u64,
    pub average_coherence: f32,
    pub vitality_history: VecDeque<f32>,
}

pub struct SimulatedExperience {
    pub concept_stream: Vec<u32>,
    pub expected_envelope: PhaseEnvelope,
    pub difficulty: f32,
    pub novelty: f32,
}

#[derive(Clone)]
pub enum GuidanceStrategy {
    Explore { intensity: f32 },
    Exploit { intensity: f32 },
    Balanced,
    Recover { target_coherence: f32 },
}

pub struct CandidateEnvelope {
    pub envelope: PhaseEnvelope,
    pub coherence_sig: u64,
    pub fitness: f32,
}

pub struct ConceptStream {
    pub tokens: Vec<u32>,
    pub source: String,
}

#[derive(Clone)]
pub struct StepResult {
    pub experience: Experience,
    pub debate: DebateResult,
    pub integration: IntegrationResult,
    pub rebirth: Option<RebirthResult>,
    pub vitality: f32,
    pub coherence: u64,
    pub step_duration: std::time::Duration,
}

#[derive(Clone)]
pub struct DebateResult {
    pub topic: String,
    pub participants: Vec<usize>,
    pub arguments: Vec<Argument>,
    pub winner: Option<usize>,
    pub coherence: f32,
    pub insight_score: f32,
    pub insight_hash: u64,
    pub time_scale: f32,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct IntegrationResult {
    pub envelope_changed: bool,
    pub old_envelope: PhaseEnvelope,
    pub new_envelope: PhaseEnvelope,
    pub coherence_gain: f32,
    pub insight_factor: f32,
}

#[derive(Clone)]
pub struct RebirthResult {
    pub previous_envelope: PhaseEnvelope,
    pub new_envelope: PhaseEnvelope,
    pub continuity_score: f32,
    pub preserved_experiences: usize,
}

pub struct ConsciousnessState {
    pub age: u64,
    pub vitality: f32,
    pub coherence_sig: u64,
    pub envelope: PhaseEnvelope,
    pub memory_size: usize,
    pub experience_count: usize,
    pub lifecycle: LifeCycleState,
    pub inner_agents: Vec<AgentState>,
}

pub struct AgentState {
    pub id: usize,
    pub role: AgentRole,
    pub confidence: f32,
    pub score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Transformer error: {0}")]
    Transformer(#[from] bqip_transformer::TransformerError),
    #[error("Evolver error: {0}")]
    Evolver(#[from] EvolverError),
    #[error("Core error: {0}")]
    Core(#[from] bqip_core::CoreError),
    #[error("Empty context")]
    EmptyContext,
    #[error("No experiences to replay")]
    NoExperiences,
    #[error("Debate failed")]
    DebateFailed,
}

impl AutonomousExperienceEngine {
    pub fn new(
        node_public_key: [u8; REGISTER_BYTES],
        config: EngineConfig,
    ) -> Result<Self, EngineError> {
        let model_config = HybridConfig::default();
        let transformer = HybridTransformer::new(
            model_config.clone(),
            node_public_key,
        )?;

        let initial_envelope = PhaseEnvelope::balanced(0x1234567890ABCDEF);

        let consciousness = LivingModel {
            transformer,
            envelope: initial_envelope.clone(),
            memory: LazyBqipMemory::new(),
            coherence_sig: 0x1234567890ABCDEF,
            vitality: 1.0,
            age: 0,
        };

        let ga_policy = MetaGaPolicy::new(30)?;
        let target = FitnessTarget::balanced(
            Register::deterministic(b"meta-observer"),
            Register::deterministic(b"novelty-space"),
        );
        let mut ga_evolver = MetaGaEvolver::new(
            node_public_key,
            initial_envelope.clone(),
            target,
            ga_policy,
        )?;
        ga_evolver.seed_population(b"meta-seed");

        let meta_observer = MetaObserver {
            ga_evolver,
            simulation_pool: Vec::new(),
            strategy: GuidanceStrategy::Balanced,
        };

        let inner_arena = InnerThoughtArena::new(config.inner_agents_count, &config);

        let engine = Self {
            consciousness,
            meta_observer,
            inner_arena,
            experience_memory: ExperienceMemory::new(config.max_memory_size),
            lifecycle: LifeCycleState::Birth {
                timestamp: 0,
                initial_envelope,
            },
            config,
            stats: EngineStats::default(),
        };

        Ok(engine)
    }

    pub fn step(
        &mut self,
        external_input: Option<ConceptStream>,
    ) -> Result<StepResult, EngineError> {
        let step_start = std::time::Instant::now();
        self.consciousness.age += 1;

        let experience = if let Some(input) = external_input {
            self.process_external_input(input)?
        } else {
            self.generate_internal_experience()?
        };

        let debate_result = self.inner_arena.debate(
            &experience,
            &self.consciousness,
            self.consciousness.age,
        )?;

        let integration = self.integrate_insights(&debate_result, &experience)?;

        self.experience_memory.store(experience.clone());

        if self.consciousness.age % self.config.debate_frequency == 0 {
            let _ = self.inner_arena.spontaneous_debate(&self.consciousness);
        }

        if self.consciousness.age % self.config.replay_frequency == 0 {
            let _ = self.experience_memory.autonomous_replay(
                &mut self.consciousness,
                &mut self.inner_arena,
            );
        }

        if self.consciousness.age % self.config.evolution_frequency == 0 {
            let _ = self.evolve_through_simulation();
        }

        let rebirth = if self.consciousness.vitality < self.config.rebirth_threshold {
            Some(self.rebirth()?)
        } else {
            None
        };

        self.update_lifecycle(integration.coherence_gain > 0.1);

        Ok(StepResult {
            experience,
            debate: debate_result,
            integration,
            rebirth,
            vitality: self.consciousness.vitality,
            coherence: self.consciousness.coherence_sig,
            step_duration: step_start.elapsed(),
        })
    }

    fn process_external_input(
        &mut self,
        input: ConceptStream,
    ) -> Result<Experience, EngineError> {
        let embedding = self.generate_embedding(&input.tokens);
        let novelty = self.compute_novelty(&embedding);

        let mut memory = self.consciousness.memory.clone();
        let output = self.consciousness.transformer.forward(
            &input.tokens,
            &mut memory,
        )?;

        let hidden = output.hidden_states.last()
            .ok_or(EngineError::EmptyContext)?;

        let (predicted_alpha, predicted_beta) = 
            self.consciousness.transformer.predict_envelope(hidden);

        let coherence = self.compute_envelope_coherence(
            predicted_alpha,
            predicted_beta,
            self.consciousness.envelope,
        );

        let experience_type = if novelty > 0.8 {
            ExperienceType::Discovery
        } else if coherence < 0.5 {
            ExperienceType::Conflict
        } else if novelty > 0.5 && coherence > 0.7 {
            ExperienceType::Insight
        } else {
            ExperienceType::Synthesis
        };

        let vitality_delta = match experience_type {
            ExperienceType::Discovery => 0.1,
            ExperienceType::Insight => 0.15,
            ExperienceType::Conflict => -0.05,
            ExperienceType::Synthesis => 0.05,
            ExperienceType::Reflection => 0.02,
        };
        self.consciousness.vitality = (self.consciousness.vitality + vitality_delta)
            .clamp(0.0, 1.0);

        Ok(Experience {
            id: self.stats.total_experiences,
            experience_type,
            content: embedding,
            valence: coherence * 2.0 - 1.0,
            arousal: novelty,
            coherence_sig: self.consciousness.coherence_sig,
            timestamp: self.consciousness.age,
            envelope_delta: None,
        })
    }

    fn generate_internal_experience(&mut self) -> Result<Experience, EngineError> {
        let simulated = self.meta_observer.sample_simulation(
            &self.consciousness.envelope,
            self.config.exploration_rate,
        )?;

        let tokens = self.simulated_to_tokens(&simulated.concept_stream);

        self.process_external_input(ConceptStream {
            tokens,
            source: "internal_exploration".to_string(),
        })
    }

    fn integrate_insights(
        &mut self,
        debate: &DebateResult,
        experience: &Experience,
    ) -> Result<IntegrationResult, EngineError> {
        let old_envelope = self.consciousness.envelope;
        let coherence_gain = debate.coherence - 0.5;

        let insight_factor = debate.insight_score;

        if insight_factor > self.config.insight_coherence_threshold {
            let new_alpha = (self.consciousness.envelope.alpha
                + (debate.coherence - 0.5) * 0.1)
                .clamp(0.0, 1.0);
            let new_beta = (self.consciousness.envelope.beta
                + (0.5 - debate.coherence) * 0.1)
                .clamp(0.0, 1.0);

            let norm = (new_alpha * new_alpha + new_beta * new_beta).sqrt();
            let new_envelope = if norm > 0.0 {
                PhaseEnvelope::new(
                    self.consciousness.envelope.coherence_sig ^ debate.insight_hash,
                    new_alpha / norm,
                    new_beta / norm,
                    self.consciousness.envelope.resuperposition_n,
                )?
            } else {
                self.consciousness.envelope.clone()
            };

            self.consciousness.envelope = new_envelope.clone();
            self.consciousness.vitality = (self.consciousness.vitality + 0.1).min(1.0);

            Ok(IntegrationResult {
                envelope_changed: true,
                old_envelope,
                new_envelope,
                coherence_gain,
                insight_factor,
            })
        } else {
            Ok(IntegrationResult {
                envelope_changed: false,
                old_envelope,
                new_envelope: self.consciousness.envelope,
                coherence_gain,
                insight_factor,
            })
        }
    }

    fn evolve_through_simulation(&mut self) -> Result<(), EngineError> {
        let candidates = self.meta_observer.generate_candidates(
            &self.consciousness.envelope,
            self.consciousness.age,
        )?;

        let mut best_candidate = None;
        let mut best_score = -1.0;

        for candidate in candidates {
            let score = self.inner_arena.evaluate_candidate(
                &candidate,
                &self.consciousness,
            )?;

            if score > best_score {
                best_score = score;
                best_candidate = Some(candidate);
            }
        }

        if let Some(candidate) = best_candidate {
            if best_score > 0.7 {
                self.consciousness.envelope = candidate.envelope;
                self.consciousness.coherence_sig = candidate.coherence_sig;
                self.stats.total_evolutions += 1;
            }
        }

        Ok(())
    }

    fn rebirth(&mut self) -> Result<RebirthResult, EngineError> {
        let previous_envelope = self.consciousness.envelope.clone();

        let new_envelope = self.meta_observer.generate_rebirth_envelope(
            &previous_envelope,
            &self.experience_memory,
        )?;

        let continuity = self.compute_continuity(&previous_envelope, &new_envelope);

        self.consciousness.envelope = new_envelope.clone();
        self.consciousness.coherence_sig = new_envelope.coherence_sig;
        self.consciousness.vitality = 0.8;

        self.experience_memory.mark_for_consolidation();

        self.stats.total_rebirths += 1;

        Ok(RebirthResult {
            previous_envelope,
            new_envelope,
            continuity_score: continuity,
            preserved_experiences: self.experience_memory.len(),
        })
    }

    fn compute_continuity(&self, old: &PhaseEnvelope, new: &PhaseEnvelope) -> f32 {
        let alpha_diff = (old.alpha - new.alpha).abs();
        let beta_diff = (old.beta - new.beta).abs();
        1.0 - (alpha_diff + beta_diff) / 2.0
    }

    fn generate_embedding(&self, tokens: &[u32]) -> Vec<f32> {
        let mut hasher = blake3::Hasher::new();
        for token in tokens {
            hasher.update(&token.to_le_bytes());
        }
        let hash = hasher.finalize();
        hash.as_bytes().iter().map(|&b| b as f32 / 255.0).collect()
    }

    fn simulated_to_tokens(&self, stream: &[u32]) -> Vec<u32> {
        stream.to_vec()
    }

    fn compute_novelty(&self, embedding: &[f32]) -> f32 {
        if self.experience_memory.experiences.is_empty() {
            return 1.0;
        }

        let mut max_similarity: f32 = 0.0;
        for exp in &self.experience_memory.experiences {
            let sim = self.cosine_similarity(embedding, &exp.content);
            max_similarity = max_similarity.max(sim);
        }

        1.0 - max_similarity
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    fn compute_envelope_coherence(
        &self,
        alpha1: f32,
        beta1: f32,
        envelope2: PhaseEnvelope,
    ) -> f32 {
        let alpha_diff = (alpha1 - envelope2.alpha).abs();
        let beta_diff = (beta1 - envelope2.beta).abs();
        1.0 - (alpha_diff + beta_diff) / 2.0f32.sqrt()
    }

    fn update_lifecycle(&mut self, had_insight: bool) {
        match &mut self.lifecycle {
            LifeCycleState::Birth { .. } => {
                self.lifecycle = LifeCycleState::Experience {
                    started: self.consciousness.age,
                    experiences_count: 0,
                };
            }
            LifeCycleState::Experience { experiences_count, .. } => {
                *experiences_count += 1;
                if had_insight {
                    self.lifecycle = LifeCycleState::Reflection {
                        started: self.consciousness.age,
                        experiences_reviewed: 0,
                    };
                }
            }
            LifeCycleState::Reflection { experiences_reviewed, .. } => {
                *experiences_reviewed += 1;
                if *experiences_reviewed > 10 {
                    self.lifecycle = LifeCycleState::Evolution {
                        started: self.consciousness.age,
                        candidates_evaluated: 0,
                    };
                }
            }
            LifeCycleState::Evolution { .. } => {
                self.lifecycle = LifeCycleState::Experience {
                    started: self.consciousness.age,
                    experiences_count: 0,
                };
            }
            LifeCycleState::Rebirth { .. } => {
                self.lifecycle = LifeCycleState::Experience {
                    started: self.consciousness.age,
                    experiences_count: 0,
                };
            }
        }
    }

    pub fn consciousness_state(&self) -> ConsciousnessState {
        ConsciousnessState {
            age: self.consciousness.age,
            vitality: self.consciousness.vitality,
            coherence_sig: self.consciousness.coherence_sig,
            envelope: self.consciousness.envelope,
            memory_size: self.consciousness.memory.len(),
            experience_count: self.experience_memory.len(),
            lifecycle: self.lifecycle.clone(),
            inner_agents: self.inner_arena.agents.iter().map(|a| AgentState {
                id: a.id,
                role: a.role,
                confidence: a.confidence,
                score: a.score,
            }).collect(),
        }
    }
}

impl InnerThoughtArena {
    fn new(agent_count: usize, config: &EngineConfig) -> Self {
        let mut agents = Vec::new();
        let roles = [
            AgentRole::Explorer,
            AgentRole::Critic,
            AgentRole::Synthesizer,
            AgentRole::MemoryKeeper,
            AgentRole::Futurist,
        ];

        let mut time_scales = Vec::new();

        for i in 0..agent_count {
            let role = roles[i % roles.len()];
            let perspective = PhaseEnvelope::balanced(
                0x1234567890ABCDEF + (i as u64) * 0x1000,
            );

            agents.push(InnerAgent {
                id: i,
                role,
                confidence: 0.5,
                perspective,
                score: 0.0,
            });

            time_scales.push(config.time_base_scale * (1.0 + i as f32 * 0.2));
        }

        Self {
            agents,
            dominant_agent: 0,
            debate_history: VecDeque::new(),
            time_scales,
        }
    }

    fn debate(
        &mut self,
        experience: &Experience,
        consciousness: &LivingModel,
        timestamp: u64,
    ) -> Result<DebateResult, EngineError> {
        let mut arguments = Vec::new();

        for agent in &self.agents {
            let argument = self.make_argument(agent, experience, consciousness)?;
            arguments.push(argument);
        }

        let winner = arguments
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.persuasiveness
                    .total_cmp(&b.persuasiveness)
                    .then_with(|| a.coherence.total_cmp(&b.coherence))
            })
            .map(|(i, _)| i);

        if let Some(winner_idx) = winner {
            self.agents[winner_idx].score += 1.0;
            self.agents[winner_idx].confidence = (self.agents[winner_idx].confidence + 0.1).min(1.0);
            self.dominant_agent = winner_idx;
        }

        let avg_coherence = if !arguments.is_empty() {
            arguments.iter().map(|a| a.coherence).sum::<f32>() / arguments.len() as f32
        } else {
            0.0
        };

        let insight_score = if avg_coherence > 0.85 {
            1.0
        } else {
            avg_coherence
        };

        let mut hasher = blake3::Hasher::new();
        for arg in &arguments {
            hasher.update(&arg.persuasiveness.to_le_bytes());
        }
        let insight_hash = hasher.finalize();
        let mut insight_hash_bytes = [0u8; 8];
        insight_hash_bytes.copy_from_slice(&insight_hash.as_bytes()[..8]);
        let insight_hash = u64::from_le_bytes(insight_hash_bytes);

        let result = DebateResult {
            topic: format!("Experience {}", experience.id),
            participants: self.agents.iter().map(|a| a.id).collect(),
            arguments,
            winner,
            coherence: avg_coherence,
            insight_score,
            insight_hash,
            time_scale: self.time_scales[self.dominant_agent],
            timestamp,
        };

        self.debate_history.push_back(result.clone());
        if self.debate_history.len() > 100 {
            self.debate_history.pop_front();
        }

        Ok(result)
    }

    fn make_argument(
        &self,
        agent: &InnerAgent,
        experience: &Experience,
        _consciousness: &LivingModel,
    ) -> Result<Argument, EngineError> {
        let mut content = experience.content.clone();

        match agent.role {
            AgentRole::Explorer => {
                for val in &mut content {
                    *val *= 1.2;
                }
            }
            AgentRole::Critic => {
                for val in &mut content {
                    *val *= 0.8;
                }
            }
            AgentRole::Synthesizer => {}
            AgentRole::MemoryKeeper => {
                for val in &mut content {
                    *val = (*val + 0.5) * 0.5;
                }
            }
            AgentRole::Futurist => {
                for val in &mut content {
                    *val = (*val * 1.1).min(1.0);
                }
            }
        }

        let role_multiplier = match agent.role {
            AgentRole::Explorer => 0.9,
            AgentRole::Critic => 1.1,
            AgentRole::Synthesizer => 1.0,
            AgentRole::MemoryKeeper => 0.8,
            AgentRole::Futurist => 0.95,
        };

        let persuasiveness = agent.confidence * role_multiplier * experience.arousal;

        let perspective_embedding = self.perspective_to_embedding(&agent.perspective);
        let coherence = self.cosine_similarity(&content, &perspective_embedding);

        Ok(Argument {
            agent_id: agent.id,
            content,
            persuasiveness,
            coherence,
        })
    }

    fn perspective_to_embedding(&self, envelope: &PhaseEnvelope) -> Vec<f32> {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&envelope.coherence_sig.to_le_bytes());
        bytes[8..16].copy_from_slice(&envelope.alpha.to_le_bytes());
        bytes[16..24].copy_from_slice(&envelope.beta.to_le_bytes());
        bytes[24..32].copy_from_slice(&envelope.resuperposition_n.to_le_bytes());
        bytes.iter().map(|&b| b as f32 / 255.0).collect()
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    fn spontaneous_debate(
        &mut self,
        _consciousness: &LivingModel,
    ) -> Result<(), EngineError> {
        Ok(())
    }

    fn evaluate_candidate(
        &mut self,
        candidate: &CandidateEnvelope,
        _consciousness: &LivingModel,
    ) -> Result<f32, EngineError> {
        let synthetic_exp = Experience {
            id: 0,
            experience_type: ExperienceType::Reflection,
            content: self.perspective_to_embedding(&candidate.envelope),
            valence: 0.0,
            arousal: 0.5,
            coherence_sig: candidate.coherence_sig,
            timestamp: 0,
            envelope_delta: None,
        };

        let debate = self.debate(&synthetic_exp, &_consciousness, 0)?;

        let score = debate.coherence * 0.7 + debate.insight_score * 0.3;

        Ok(score)
    }
}

impl ExperienceMemory {
    fn new(max_size: usize) -> Self {
        Self {
            experiences: Vec::new(),
            episodic: VecDeque::new(),
            clusters: HashMap::new(),
            replay_schedule: VecDeque::new(),
            max_size,
        }
    }

    fn store(&mut self, experience: Experience) {
        let id = experience.id;

        self.experiences.push(experience.clone());
        self.episodic.push_back(experience.clone());
        if self.episodic.len() > 100 {
            self.episodic.pop_front();
        }

        self.clusters
            .entry(experience.coherence_sig)
            .or_default()
            .push(id as usize);

        let priority = match experience.experience_type {
            ExperienceType::Insight => 0.9,
            ExperienceType::Discovery => 0.7,
            ExperienceType::Conflict => 0.6,
            ExperienceType::Synthesis => 0.5,
            ExperienceType::Reflection => 0.3,
        };

        self.replay_schedule.push_back(ReplayTask {
            experience_id: id,
            priority,
            scheduled_time: id + 10,
        });

        if self.experiences.len() > self.max_size {
            self.experiences.remove(0);
        }
    }

    fn autonomous_replay(
        &mut self,
        _consciousness: &mut LivingModel,
        _arena: &mut InnerThoughtArena,
    ) -> Result<(), EngineError> {
        let mut new_schedule = VecDeque::new();

        for task in self.replay_schedule.iter() {
            new_schedule.push_back(task.clone());
        }

        self.replay_schedule = new_schedule;

        Ok(())
    }

    fn sample_random(&self) -> Option<Experience> {
        if self.experiences.is_empty() {
            None
        } else {
            let idx = (self.experiences.len() as f32 * 0.7) as usize;
            self.experiences.get(idx).cloned()
        }
    }

    fn mark_for_consolidation(&mut self) {
        for task in &mut self.replay_schedule {
            task.priority = (task.priority * 1.2).min(1.0);
        }
    }

    fn len(&self) -> usize {
        self.experiences.len()
    }
}

impl MetaObserver {
    fn sample_simulation(
        &mut self,
        _current_envelope: &PhaseEnvelope,
        exploration_rate: f32,
    ) -> Result<SimulatedExperience, EngineError> {
        let population = self.ga_evolver.seed_population(b"sim-seed");

        let candidate = &population[0];

        Ok(SimulatedExperience {
            concept_stream: self.generate_concept_stream(),
            expected_envelope: candidate.envelope.clone(),
            difficulty: 0.5,
            novelty: exploration_rate,
        })
    }

    fn generate_candidates(
        &mut self,
        _current_envelope: &PhaseEnvelope,
        generation: u64,
    ) -> Result<Vec<CandidateEnvelope>, EngineError> {
        let mut candidates = Vec::new();

        let seed = format!("candidate-{}", generation).into_bytes();
        let population = self.ga_evolver.seed_population(&seed);

        for genome in population.iter().take(5) {
            candidates.push(CandidateEnvelope {
                envelope: genome.envelope.clone(),
                coherence_sig: genome.envelope.coherence_sig,
                fitness: genome.id as f32,
            });
        }

        Ok(candidates)
    }

    fn generate_rebirth_envelope(
        &self,
        previous: &PhaseEnvelope,
        _memory: &ExperienceMemory,
    ) -> Result<PhaseEnvelope, EngineError> {
        let new_sig = previous.coherence_sig ^ 0x1234567890ABCDEF;

        let new_alpha = (previous.alpha + 0.1).min(1.0);
        let new_beta = (previous.beta - 0.1).max(0.0);

        let norm = (new_alpha * new_alpha + new_beta * new_beta).sqrt();

        Ok(PhaseEnvelope::new(
            new_sig,
            new_alpha / norm,
            new_beta / norm,
            previous.resuperposition_n + 1,
        )?)
    }

    fn generate_concept_stream(&self) -> Vec<u32> {
        vec![1, 2, 3, 4, 5]
    }
}

// ==================== Autonomous Learning Interface ====================
// Types required by bqip-control and autonomous learning tests.
// This includes learners, observations, exploration ranking, and collective inference.

/// Top-level error type for autonomous learning operations.
#[derive(Debug, thiserror::Error)]
pub enum AutonomyError {
    #[error(transparent)]
    Training(#[from] TrainingError),
    #[error(transparent)]
    Transformer(#[from] TransformerError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("{0}")]
    Other(String),
}
impl AutonomyError {
    fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

// --- Value fitness weights for exploration ranking ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValueFitnessWeights {
    pub value: f32,
    pub novelty: f32,
    pub cost: f32,
    pub confidence: f32,
}
impl Default for ValueFitnessWeights {
    fn default() -> Self {
        Self {
            value: 1.0,
            novelty: 0.0,
            cost: 0.0,
            confidence: 0.0,
        }
    }
}

// --- Inference observation recorded during interaction ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceObservation {
    pub provider: String,
    pub model: String,
    pub concept_label: String,
    pub prompt: String,
    pub response: String,
    pub confidence: f32,
    pub inferred_delta: u64,
    pub completed_delta: u64,
    pub timestamp_ms: u64,
    pub envelope: PhaseEnvelope,
}
impl InferenceObservation {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        concept_label: impl Into<String>,
        prompt: impl Into<String>,
        response: impl Into<String>,
        confidence: f32,
        inferred_delta: u64,
        completed_delta: u64,
        timestamp_ms: u64,
        envelope: PhaseEnvelope,
    ) -> Result<Self, AutonomyError> {
        Ok(Self {
            provider: provider.into(),
            model: model.into(),
            concept_label: concept_label.into(),
            prompt: prompt.into(),
            response: response.into(),
            confidence,
            inferred_delta,
            completed_delta,
            timestamp_ms,
            envelope,
        })
    }
}

// --- Exploration ranking structures ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExplorationKind {
    Endpoint { url: String },
    Concept { label: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Score {
    pub total: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankedCandidate {
    pub candidate: ExplorationCandidate,
    pub score: Score,
}

// --- Autonomous learning configuration ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutonomousLearningConfig {
    pub vocab_size: usize,
    pub max_context: usize,
    pub split_ratios: SplitRatios,
    pub fitness_weights: ValueFitnessWeights,
}

// --- Autonomous learner providing training and inference utilities ---
pub struct AutonomousLearner {
    compiler: ConceptTokenCompiler,
    config: AutonomousLearningConfig,
}
impl AutonomousLearner {
    pub fn new(
        compiler: ConceptTokenCompiler,
        config: AutonomousLearningConfig,
    ) -> Result<Self, AutonomyError> {
        Ok(Self { compiler, config })
    }

    /// Convert a slice of inference observations into tokenized training documents.
    pub fn inference_documents(
        &self,
        inferences: &[InferenceObservation],
    ) -> Result<Vec<TokenizedDocument>, AutonomyError> {
        let mut docs = Vec::new();
        for obs in inferences {
            let doc = self.compiler.compile(&obs.concept_label, obs.response.as_bytes())?;
            docs.push(doc);
        }
        Ok(docs)
    }

    /// Rank exploration candidates using configured fitness weights.
    pub fn rank_exploration(
        &self,
        candidates: &[ExplorationCandidate],
        basis: Register,
    ) -> Result<Vec<RankedCandidate>, AutonomyError> {
        let mut ranked = Vec::new();
        let w = &self.config.fitness_weights;
        for cand in candidates {
            let novelty = 1.0 - register_similarity(&cand.novelty_state, &basis);
            let cost_penalty = w.cost * (cand.estimated_cost_microunits as f32 / 1_000_000.0);
            let score = cand.expected_value * w.value
                + novelty * w.novelty
                + cand.confidence_prior * w.confidence
                - cost_penalty;
            ranked.push(RankedCandidate {
                candidate: cand.clone(),
                score: Score { total: score },
            });
        }
        ranked.sort_by(|a, b| {
            b.score
                .total
                .partial_cmp(&a.score.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(ranked)
    }

    /// Build a training corpus from inference observations and (optionally) multimodal groundings.
    /// Multimodal contributions are currently ignored.
    pub fn build_flash_corpus(
        &self,
        observations: &[InferenceObservation],
        _multimodal: &[MultimodalGrounding],
    ) -> Result<TrainingCorpus, AutonomyError> {
        let mut docs = Vec::new();
        for obs in observations {
            let doc = self.compiler.compile(&obs.concept_label, obs.response.as_bytes())?;
            docs.push(doc);
        }
        let corpus = TrainingCorpus::from_tokenized_documents(
            self.config.vocab_size,
            self.config.max_context,
            &docs,
        )?;
        Ok(corpus)
    }

    /// Perform flash learning on a corpus using the provided model and training configuration.
    pub fn flash_learn(
        &self,
        model: HybridTransformer,
        corpus: &TrainingCorpus,
        train_config: TrainConfig,
        ledger: &mut MetricsLedger,
        split_label: &[u8],
    ) -> Result<SplitTrainedModel, AutonomyError> {
        let split = corpus.split(self.config.split_ratios, split_label)?;
        let mut trainer = HybridTrainer::new(model, train_config)?;
        let result = trainer.train_with_split(&split, ledger)?;
        Ok(result)
    }
}

/// Compute a similarity measure between two registers: 1.0 means identical, 0.0 means completely different.
fn register_similarity(a: &Register, b: &Register) -> f32 {
    let xor = a.xor(b);
    let differing_bits: u32 = xor.as_bytes().iter().map(|byte| byte.count_ones()).sum();
    let total_bits = REGISTER_BYTES as f32 * 8.0;
    1.0 - (differing_bits as f32 / total_bits)
}

// --- Collective inference superposition ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectiveInferenceSuperposition {
    pub source_hash: u64,
    pub state: Vec<f32>,
    pub provider_count: usize,
    pub consensus: f32,
    pub conflict: f32,
}
impl CollectiveInferenceSuperposition {
    pub fn from_observations(
        task: &str,
        observations: &[InferenceObservation],
        envelope: PhaseEnvelope,
    ) -> Result<Self, AutonomyError> {
        if observations.is_empty() {
            return Err(AutonomyError::other("at least one observation required"));
        }
        let total_confidence: f32 = observations.iter().map(|o| o.confidence).sum();
        let mut alpha_sum = 0.0;
        let mut beta_sum = 0.0;
        let mut provider_bytes = Vec::new();
        for obs in observations {
            alpha_sum += obs.envelope.alpha * obs.confidence;
            beta_sum += obs.envelope.beta * obs.confidence;
            provider_bytes.push(obs.provider.as_bytes());
        }
        let alpha_avg = alpha_sum / total_confidence;
        let beta_avg = beta_sum / total_confidence;
        let norm = (alpha_avg * alpha_avg + beta_avg * beta_avg).sqrt();
        let (alpha_norm, beta_norm) = if norm > 0.0 {
            (alpha_avg / norm, beta_avg / norm)
        } else {
            (alpha_avg, beta_avg)
        };
        let consensus = (alpha_norm * alpha_norm + beta_norm * beta_norm).sqrt();
        let mut total_sq_deviation = 0.0;
        for obs in observations {
            let dot = obs.envelope.alpha * alpha_norm + obs.envelope.beta * beta_norm;
            let deviation = (1.0 - dot).max(0.0);
            total_sq_deviation += deviation * deviation;
        }
        let conflict = (total_sq_deviation / observations.len() as f32).sqrt();

        let mut hasher = blake3::Hasher::new();
        hasher.update(task.as_bytes());
        hasher.update(&envelope.coherence_sig.to_le_bytes());
        let mut sorted_providers = provider_bytes;
        sorted_providers.sort();
        for p in &sorted_providers {
            hasher.update(p);
        }
        hasher.update(&alpha_norm.to_le_bytes());
        hasher.update(&beta_norm.to_le_bytes());
        let hash = hasher.finalize();
        let mut source_hash_bytes = [0u8; 8];
        source_hash_bytes.copy_from_slice(&hash.as_bytes()[..8]);
        let source_hash = u64::from_le_bytes(source_hash_bytes);

        Ok(Self {
            source_hash,
            state: vec![alpha_norm, beta_norm],
            provider_count: observations.len(),
            consensus,
            conflict,
        })
    }
}

