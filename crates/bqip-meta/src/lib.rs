use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use bqip_core::{
    derive_register_id, DualState, InterfaceKind, PhaseEnvelope, Register, RegisterId,
    RegisterLane, REGISTER_BYTES,
};
use bqip_evolver::{
    CandidateGenome, EvolverError, FitnessTarget, MetaGaEvolver, MetaGaPolicy,
    ScoredCandidate,
};
use bqip_training::{
    ConceptTokenCompiler, ConceptTokenizerConfig, HybridTrainer, TrainConfig, TrainingCorpus,
};
use bqip_transformer::{HybridConfig, HybridTransformer, ModelWeights};
use serde::{Deserialize, Serialize};

/// Meta-learning orchestrator that combines GA exploration with gradient exploitation
/// and maintains a global embedding index for inference-aware adaptation.
pub struct MetaLearningOrchestrator {
    /// GA-based hyperparameter and architecture explorer
    ga_evolver: MetaGaEvolver,
    /// Current best model (evolved + fine-tuned)
    current_model: HybridTransformer,
    /// Embedding index for global inference sensing
    embedding_index: Arc<Mutex<EmbeddingIndex>>,
    /// History of meta-learning rounds
    history: Vec<MetaRound>,
    /// Configuration for meta-learning
    config: MetaConfig,
    /// Performance trend tracker
    trend_tracker: TrendTracker,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaConfig {
    /// Number of GA generations per meta-round
    pub ga_generations: u32,
    /// Population size for GA
    pub ga_population: usize,
    /// How many top candidates to fine-tune with gradients
    pub top_candidates_to_fine_tune: usize,
    /// Learning rate for fine-tuning
    pub fine_tune_lr: f32,
    /// Epochs for fine-tuning each candidate
    pub fine_tune_epochs: usize,
    /// Batch size for fine-tuning
    pub fine_tune_batch_size: usize,
    /// Embedding similarity threshold for concept grouping
    pub embedding_similarity_threshold: f32,
    /// How often to update the embedding index (in rounds)
    pub embedding_update_frequency: u32,
    /// Maximum size of embedding index before pruning
    pub max_embedding_index_size: usize,
    /// Minimum improvement to consider a meta-round successful
    pub min_improvement_threshold: f32,
    /// Patience for adaptive strategy adjustment
    pub strategy_patience: u32,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            ga_generations: 5,
            ga_population: 20,
            top_candidates_to_fine_tune: 3,
            fine_tune_lr: 0.08,
            fine_tune_epochs: 10,
            fine_tune_batch_size: 8,
            embedding_similarity_threshold: 0.85,
            embedding_update_frequency: 2,
            max_embedding_index_size: 1000,
            min_improvement_threshold: 0.02,
            strategy_patience: 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaRound {
    pub round: u32,
    pub ga_evolution: GaEvolutionResult,
    pub fine_tuning: Vec<FineTuneResult>,
    pub best_model_score: f32,
    pub embedding_stats: EmbeddingStats,
    pub strategy_adjustment: Option<StrategyAdjustment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GaEvolutionResult {
    pub generations: Vec<EvolutionRoundSummary>,
    pub best_candidate: CandidateGenome,
    pub best_fitness: f32,
    pub diversity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionRoundSummary {
    pub generation: u32,
    pub best_fitness: f32,
    pub avg_fitness: f32,
    pub diversity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FineTuneResult {
    pub candidate_id: u64,
    pub initial_loss: f32,
    pub final_loss: f32,
    pub improvement: f32,
    pub epochs: usize,
    pub model_hash: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingStats {
    pub total_embeddings: usize,
    pub concept_clusters: usize,
    pub avg_similarity: f32,
    pub coverage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StrategyAdjustment {
    IncreaseExploration { reason: String },
    IncreaseExploitation { reason: String },
    AdjustEmbeddingThreshold { old: f32, new: f32, reason: String },
    MaintainStrategy { reason: String },
}

/// Embedding-based model state index for global inference sensing
#[derive(Clone, Debug, Default)]
pub struct EmbeddingIndex {
    entries: Vec<EmbeddingEntry>,
    concept_clusters: HashMap<String, Vec<usize>>,
    similarity_graph: HashMap<usize, Vec<(usize, f32)>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    pub id: usize,
    pub model_hash: [u8; 32],
    pub embedding: Vec<f32>,
    pub concept_label: String,
    pub performance_score: f32,
    pub generation: u32,
    pub timestamp: u64,
}

/// Tracks performance trends for adaptive strategy
#[derive(Clone, Debug, Default)]
struct TrendTracker {
    improvements: VecDeque<f32>,
    stagnation_count: u32,
    exploration_phase: bool,
}

impl MetaLearningOrchestrator {
    /// Create a new meta-learning orchestrator
    pub fn new(
        node_public_key: [u8; REGISTER_BYTES],
        config: MetaConfig,
    ) -> Result<Self, MetaError> {
        let ga_policy = MetaGaPolicy::new(config.ga_population)?;
        let target = FitnessTarget::balanced(
            Register::deterministic(b"meta-target"),
            Register::deterministic(b"meta-novelty"),
        );
        let envelope = PhaseEnvelope::balanced(0xmeta_learn);

        let mut ga_evolver = MetaGaEvolver::new(
            node_public_key,
            envelope.clone(),
            target,
            ga_policy,
        )?;

        // Create initial model
        let model_config = HybridConfig::default();
        let current_model = HybridTransformer::new(
            model_config,
            node_public_key,
        )?;

        // Seed the GA population
        let seed_material = b"meta-seed-2026";
        ga_evolver.seed_population(seed_material);

        Ok(Self {
            ga_evolver,
            current_model,
            embedding_index: Arc::new(Mutex::new(EmbeddingIndex::default())),
            history: Vec::new(),
            config,
            trend_tracker: TrendTracker::default(),
        })
    }

    /// Run a complete meta-learning round
    pub fn run_meta_round(
        &mut self,
        corpus: &TrainingCorpus,
        round: u32,
    ) -> Result<MetaRound, MetaError> {
        // Phase 1: GA Evolution - explore hyperparameter and architecture space
        let ga_result = self.run_ga_evolution(round)?;

        // Phase 2: Fine-tune top candidates with gradient descent
        let fine_tune_results = self.fine_tune_candidates(
            &ga_result.top_candidates,
            corpus,
            round,
        )?;

        // Phase 3: Select best model and update embedding index
        let best_result = self.select_and_update_best(
            &fine_tune_results,
            corpus,
            round,
        )?;

        // Phase 4: Update embedding index for global inference sensing
        let embedding_stats = if round % self.config.embedding_update_frequency == 0 {
            self.update_embedding_index(&fine_tune_results, round)?
        } else {
            self.compute_embedding_stats()?
        };

        // Phase 5: Adaptive strategy adjustment based on trends
        let strategy_adjustment = self.adjust_strategy(
            &best_result,
            &embedding_stats,
            round,
        );

        // Build meta-round result
        let meta_round = MetaRound {
            round,
            ga_evolution: ga_result,
            fine_tuning: fine_tune_results,
            best_model_score: best_result.improvement,
            embedding_stats,
            strategy_adjustment,
        };

        self.history.push(meta_round.clone());

        Ok(meta_round)
    }

    /// Phase 1: Run GA evolution to explore candidate models
    fn run_ga_evolution(
        &mut self,
        round: u32,
    ) -> Result<GaEvolutionResult, MetaError> {
        let seed_material = format!"ga-seed-{}-{}", round, self.history.len()).into_bytes();
        let mut population = self.ga_evolver.seed_population(&seed_material);

        let mut generations = Vec::new();

        for gen in 0..self.config.ga_generations {
            let round_result = self.ga_evolver.evolve_round(
                &population,
                &format!"round-{}-gen-{}", round, gen).into_bytes(),
            )?;

            generations.push(EvolutionRoundSummary {
                generation: round_result.generation,
                best_fitness: round_result.ranked[0].score.total,
                avg_fitness: round_result
                    .ranked
                    .iter()
                    .map(|c| c.score.total)
                    .sum::<f32>()
                    / round_result.ranked.len() as f32,
                diversity: round_result.diversity,
            });

            population = round_result
                .ranked
                .into_iter()
                .map(|scored| scored.genome)
                .collect();
        }

        let best_candidate = population[0].clone();
        let best_fitness = generations.last().unwrap().best_fitness;
        let diversity = generations.last().unwrap().diversity;

        Ok(GaEvolutionResult {
            generations,
            best_candidate,
            best_fitness,
            diversity,
        })
    }

    /// Phase 2: Fine-tune top candidates with gradient-based training
    fn fine_tune_candidates(
        &self,
        candidates: &[CandidateGenome],
        corpus: &TrainingCorpus,
        round: u32,
    ) -> Result<Vec<FineTuneResult>, MetaError> {
        let top_n = candidates
            .len()
            .min(self.config.top_candidates_to_fine_tune);
        let mut results = Vec::new();

        for candidate in candidates.iter().take(top_n) {
            // Convert candidate genome to model weights
            let model_weights = self.candidate_to_weights(candidate)?;
            let model = HybridTransformer::from_weights(model_weights)?;

            // Configure training for fine-tuning
            let train_config = TrainConfig {
                epochs: self.config.fine_tune_epochs,
                batch_size: self.config.fine_tune_batch_size,
                learning_rate: self.config.fine_tune_lr,
                weight_decay: 0.0,
                max_grad_norm: 1.5,
                trainable_scope: bqip_training::TrainableScope::LmHead,
                finite_difference_epsilon: 1.0e-3,
                twin_loss_weight: 0.1,
                phase_delta_increment: 1,
                twin_perturbation_scale: 0.05,
                graph_contrastive_weight: 0.15,
            };

            let trainer = HybridTrainer::new(model, train_config)?;
            let trained = trainer.train(corpus)?;

            let improvement = trained.report.final_loss - trained.report.initial_loss;
            let model_hash = self.hash_model(&trained.model);

            results.push(FineTuneResult {
                candidate_id: candidate.id,
                initial_loss: trained.report.initial_loss,
                final_loss: trained.report.final_loss,
                improvement: -improvement, // Negative because lower loss is better
                epochs: trained.report.epochs.len(),
                model_hash,
            });
        }

        Ok(results)
    }

    /// Phase 3: Select best model and update current state
    fn select_and_update_best(
        &mut self,
        results: &[FineTuneResult],
        corpus: &TrainingCorpus,
        round: u32,
    ) -> Result<FineTuneResult, MetaError> {
        let best = results
            .iter()
            .max_by(|a, b| a.improvement.total_cmp(&b.improvement))
            .ok_or(MetaError::NoCandidates)?
            .clone();

        // Update trend tracker
        self.trend_tracker.update(best.improvement);

        Ok(best)
    }

    /// Phase 4: Update embedding index with model states
    fn update_embedding_index(
        &mut self,
        results: &[FineTuneResult],
        round: u32,
    ) -> Result<EmbeddingStats, MetaError> {
        let mut index = self.embedding_index.lock().unwrap();

        for result in results {
            // Create embedding from model (simplified: use model hash as feature vector)
            let embedding = self.create_embedding(result);
            let entry = EmbeddingEntry {
                id: index.entries.len(),
                model_hash: result.model_hash,
                embedding,
                concept_label: format!"round-{}", round),
                performance_score: result.improvement,
                generation: round,
                timestamp: self.current_timestamp(),
            };

            index.entries.push(entry);
        }

        // Prune if too large
        if index.entries.len() > self.config.max_embedding_index_size {
            self.prune_embedding_index(&mut index);
        }

        // Rebuild similarity graph
        self.rebuild_similarity_graph(&mut index);

        Ok(self.compute_embedding_stats()?)
    }

    /// Create embedding vector from model result
    fn create_embedding(&self, result: &FineTuneResult) -> Vec<f32> {
        // Simplified: use hash bytes normalized to [0,1]
        result
            .model_hash
            .iter()
            .map(|&b| b as f32 / 255.0)
            .collect()
    }

    /// Rebuild similarity graph based on embedding distances
    fn rebuild_similarity_graph(&self, index: &mut EmbeddingIndex) {
        index.similarity_graph.clear();
        let threshold = self.config.embedding_similarity_threshold;

        for i in 0..index.entries.len() {
            for j in (i + 1)..index.entries.len() {
                let sim = self.cosine_similarity(
                    &index.entries[i].embedding,
                    &index.entries[j].embedding,
                );
                if sim >= threshold {
                    index
                        .similarity_graph
                        .entry(i)
                        .or_default()
                        .push((j, sim));
                    index
                        .similarity_graph
                        .entry(j)
                        .or_default()
                        .push((i, sim));
                }
            }
        }
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    /// Prune embedding index to maintain size limit
    fn prune_embedding_index(&self, index: &mut EmbeddingIndex) {
        // Keep most recent and best performing entries
        index.entries.sort_by(|a, b| {
            b.performance_score
                .total_cmp(&a.performance_score)
                .then(b.generation.cmp(&a.generation))
        });
        index.entries.truncate(self.config.max_embedding_index_size / 2);
        index.entries.sort_by_key(|e| e.id);
    }

    /// Compute embedding statistics
    fn compute_embedding_stats(&self) -> Result<EmbeddingStats, MetaError> {
        let index = self.embedding_index.lock().unwrap();
        let total = index.entries.len();
        let clusters = index.concept_clusters.len();

        let avg_sim = if total > 1 {
            let mut sum = 0.0;
            let mut count = 0;
            for i in 0..total {
                for j in (i + 1)..total {
                    let sim = self.cosine_similarity(
                        &index.entries[i].embedding,
                        &index.entries[j].embedding,
                    );
                    sum += sim;
                    count += 1;
                }
            }
            if count > 0 {
                sum / count as f32
            } else {
                0.0
            }
        } else {
            0.0
        };

        Ok(EmbeddingStats {
            total_embeddings: total,
            concept_clusters: clusters,
            avg_similarity: avg_sim,
            coverage: total as f32 / self.config.max_embedding_index_size as f32,
        })
    }

    /// Phase 5: Adapt strategy based on performance trends
    fn adjust_strategy(
        &mut self,
        best_result: &FineTuneResult,
        embedding_stats: &EmbeddingStats,
        round: u32,
    ) -> Option<StrategyAdjustment> {
        let improvement = best_result.improvement;
        let below_threshold = improvement < self.config.min_improvement_threshold;

        // Check if we're in stagnation
        if below_threshold {
            self.trend_tracker.stagnation_count += 1;
        } else {
            self.trend_tracker.stagnation_count = 0;
        }

        let adjustment = if self.trend_tracker.stagnation_count >= self.config.strategy_patience {
            if self.trend_tracker.exploration_phase {
                // Shift to exploitation
                self.trend_tracker.exploration_phase = false;
                self.trend_tracker.stagnation_count = 0;
                Some(StrategyAdjustment::IncreaseExploitation {
                    reason: "Stagnation during exploration phase, shifting to exploitation"
                        .to_string(),
                })
            } else {
                // Increase exploration
                self.trend_tracker.exploration_phase = true;
                self.trend_tracker.stagnation_count = 0;
                Some(StrategyAdjustment::IncreaseExploration {
                    reason: "Stagnation during exploitation phase, increasing exploration"
                        .to_string(),
                })
            }
        } else if embedding_stats.avg_similarity > 0.95 {
            // Too similar embeddings, need more diversity
            let old_threshold = self.config.embedding_similarity_threshold;
            self.config.embedding_similarity_threshold = (old_threshold * 0.95).max(0.7);
            Some(StrategyAdjustment::AdjustEmbeddingThreshold {
                old: old_threshold,
                new: self.config.embedding_similarity_threshold,
                reason: "Embeddings too similar, lowering threshold for diversity".to_string(),
            })
        } else if embedding_stats.coverage < 0.3 {
            // Not enough exploration of embedding space
            let old_threshold = self.config.embedding_similarity_threshold;
            self.config.embedding_similarity_threshold = (old_threshold * 1.05).min(0.95);
            Some(StrategyAdjustment::AdjustEmbeddingThreshold {
                old: old_threshold,
                new: self.config.embedding_similarity_threshold,
                reason: "Low embedding coverage, raising threshold for broader exploration"
                    .to_string(),
            })
        } else {
            Some(StrategyAdjustment::MaintainStrategy {
                reason: "Performance within acceptable range, maintaining current strategy"
                    .to_string(),
            })
        };

        // Apply GA policy adjustments based on strategy
        if let Some(StrategyAdjustment::IncreaseExploration { .. }) = adjustment {
            self.ga_evolver.policy.mutation_rate =
                (self.ga_evolver.policy.mutation_rate * 1.2).min(0.5);
            self.ga_evolver.policy.exploration_pressure =
                (self.ga_evolver.policy.exploration_pressure * 1.15).min(0.8);
        } else if let Some(StrategyAdjustment::IncreaseExploitation { .. }) = adjustment {
            self.ga_evolver.policy.mutation_rate =
                (self.ga_evolver.policy.mutation_rate * 0.8).max(0.05);
            self.ga_evolver.policy.crossover_rate =
                (self.ga_evolver.policy.crossover_rate * 1.1).min(0.95);
        }

        adjustment
    }

    /// Convert candidate genome to model weights
    fn candidate_to_weights(
        &self,
        candidate: &CandidateGenome,
    ) -> Result<ModelWeights, MetaError> {
        let config = HybridConfig::default();
        let weights = ModelWeights {
            config: config.clone(),
            node_public_key: self.ga_evolver.node_public_key,
            embeddings: HybridTransformer::deterministic_matrix(
                config.vocab_size,
                config.d_model,
                b"meta-embeddings",
                candidate.id,
            ),
            query: HybridTransformer::deterministic_matrix(
                config.d_model,
                config.d_model,
                b"meta-query",
                candidate.id,
            ),
            key: HybridTransformer::deterministic_matrix(
                config.d_model,
                config.d_model,
                b"meta-key",
                candidate.id,
            ),
            value: HybridTransformer::deterministic_matrix(
                config.d_model,
                config.d_model,
                b"meta-value",
                candidate.id,
            ),
            write_gate: HybridTransformer::deterministic_matrix(
                config.d_model,
                config.d_model,
                b"meta-write",
                candidate.id,
            ),
            forget_gate: HybridTransformer::deterministic_matrix(
                config.d_model,
                config.d_model,
                b"meta-forget",
                candidate.id,
            ),
            output: HybridTransformer::deterministic_matrix(
                config.d_model,
                config.d_model,
                b"meta-output",
                candidate.id,
            ),
            lm_head: HybridTransformer::deterministic_matrix(
                config.vocab_size,
                config.d_model,
                b"meta-lmhead",
                candidate.id,
            ),
            envelope_alpha: HybridTransformer::deterministic_matrix(
                1,
                config.d_model,
                b"meta-env-alpha",
                candidate.id,
            ),
            envelope_beta: HybridTransformer::deterministic_matrix(
                1,
                config.d_model,
                b"meta-env-beta",
                candidate.id,
            ),
        };
        Ok(weights)
    }

    /// Hash model for identification
    fn hash_model(&self, model: &HybridTransformer) -> [u8; 32] {
        let weights = model.weights();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&bincode::serialize(&weights.config).unwrap());
        hasher.update(weights.embeddings.data.as_slice());
        *hasher.finalize().as_bytes()
    }

    /// Get current timestamp
    fn current_timestamp(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Get meta-learning history
    pub fn history(&self) -> &[MetaRound] {
        &self.history
    }

    /// Get current best model
    pub fn current_model(&self) -> &HybridTransformer {
        &self.current_model
    }

    /// Get GA evolver reference
    pub fn ga_evolver(&self) -> &MetaGaEvolver {
        &self.ga_evolver
    }

    /// Get embedding index snapshot
    pub fn embedding_index(&self) -> EmbeddingIndex {
        self.embedding_index.lock().unwrap().clone()
    }
}

impl TrendTracker {
    fn update(&mut self, improvement: f32) {
        self.improvements.push_back(improvement);
        if self.improvements.len() > 10 {
            self.improvements.pop_front();
        }
    }
}

impl EmbeddingIndex {
    /// Find similar models for knowledge transfer
    pub fn find_similar(
        &self,
        embedding: &[f32],
        threshold: f32,
        max_results: usize,
    ) -> Vec<(usize, f32)> {
        let mut similarities: Vec<(usize, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let sim = cosine_similarity(embedding, &entry.embedding);
                (entry.id, sim)
            })
            .filter(|(_, sim)| *sim >= threshold)
            .collect();
        similarities.sort_by(|a, b| b.1.total_cmp(&a.1));
        similarities.truncate(max_results);
        similarities
    }

    /// Get cluster members
    pub fn cluster_members(&self, cluster_label: &str) -> Vec<&EmbeddingEntry> {
        self.concept_clusters
            .get(cluster_label)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| self.entries.get(i))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    #[error("GA evolution error: {0}")]
    GaError(#[from] EvolverError),
    #[error("Transformer error: {0}")]
    TransformerError(#[from] bqip_transformer::TransformerError),
    #[error("Training error: {0}")]
    TrainingError(#[from] bqip_training::TrainingError),
    #[error("No candidates available for fine-tuning")]
    NoCandidates,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] Box<bincode::ErrorKind>),
}

// Helper trait for deterministic matrix generation
impl HybridTransformer {
    fn deterministic_matrix(
        rows: usize,
        cols: usize,
        label: &'static [u8],
        seed: u64,
    ) -> bqip_transformer::Matrix {
        use bqip_transformer::Matrix;
        let mut data = Vec::with_capacity(rows * cols);
        let scale = (cols as f32).sqrt().recip();
        for row in 0..rows {
            for col in 0..cols {
                let mut hasher = blake3::Hasher::new();
                hasher.update(label);
                hasher.update(&seed.to_le_bytes());
                hasher.update(&row.to_le_bytes());
                hasher.update(&col.to_le_bytes());
                let hash = hasher.finalize();
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&hash.as_bytes()[..4]);
                let unit = u32::from_le_bytes(bytes) as f32 / u32::MAX as f32;
                data.push((unit * 2.0 - 1.0) * scale);
            }
        }
        Matrix { rows, cols, data }
    }
}
