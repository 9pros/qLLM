use blake3::Hash;
use serde::{Deserialize, Serialize};

pub mod tepe_engine;
pub mod collapse_commit;

use bqip_core::{
    derive_register_id, phase_project, DualState, InterfaceKind, PhaseEnvelope, Register,
    RegisterId, RegisterLane, REGISTER_BYTES,
};

/// TEPE: Twin-Encoded Phase Evolution genome types
/// Ctwin = twin genotype (envelope parameters, hyperangular vectors, ORL backpointers)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TwinGenome {
    pub id: u64,
    pub generation: u32,
    pub envelope: PhaseEnvelope,
    /// Hyperangular rotation vector for rotational transformer
    pub ha_vector: [f32; 4],
    /// ORL backpointer hash for replay verification
    pub orl_backpointer: [u8; REGISTER_BYTES],
    /// Crypto binding hash
    pub crypto_binding: [u8; REGISTER_BYTES],
    /// Lane binding identifier
    pub lane_binding: RegisterLane,
}

impl TwinGenome {
    pub fn seed(
        id: u64,
        generation: u32,
        envelope: PhaseEnvelope,
        node_public_key: &[u8; REGISTER_BYTES],
        seed_material: &[u8],
    ) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"tepe-twin-seed");
        hasher.update(&id.to_le_bytes());
        hasher.update(&generation.to_le_bytes());
        hasher.update(seed_material);
        let hash = hasher.finalize();
        
        // Derive ha_vector from hash
        let mut ha_vector = [0.0f32; 4];
        for i in 0..4 {
            let bytes: [u8; 4] = hash.as_bytes()[i*4..(i+1)*4].try_into().unwrap();
            ha_vector[i] = (u32::from_le_bytes(bytes) as f32 / u32::MAX as f32) * std::f32::consts::PI * 2.0;
        }
        
        let mut hasher2 = blake3::Hasher::new();
        hasher2.update(b"orl-backpointer");
        hasher2.update(&id.to_le_bytes());
        hasher2.update(seed_material);
        let orl_backpointer = *hasher2.finalize().as_bytes();
        
        let mut hasher3 = blake3::Hasher::new();
        hasher3.update(b"crypto-binding");
        hasher3.update(node_public_key);
        hasher3.update(&id.to_le_bytes());
        let crypto_binding = *hasher3.finalize().as_bytes();
        
        Self {
            id,
            generation,
            envelope,
            ha_vector,
            orl_backpointer,
            crypto_binding,
            lane_binding: RegisterLane::GenericEndpoint,
        }
    }
    
    pub fn validate(&self) -> Result<(), EvolverError> {
        self.envelope.validate()?;
        for val in &self.ha_vector {
            if !val.is_finite() {
                return Err(EvolverError::InvalidHaVector);
            }
        }
        Ok(())
    }
    
    /// Decode Ctwin to Clive phenotype using U_decode = R_HF * R_HA * O
    pub fn decode_to_phenotype(&self, live: Register) -> LivePhenotype {
        // Apply high-frequency rotation from envelope
        let hf_rotation = self.envelope.hf_rotation;
        let ha_rotation = self.ha_vector.iter().map(|v| v.abs()).sum::<f32>() / 4.0;
        
        // Build decoded envelope with rotational parameters
        let decoded_envelope = PhaseEnvelope::unchecked_rotational(
            self.envelope.coherence_sig,
            self.envelope.alpha,
            self.envelope.beta,
            self.envelope.resuperposition_n,
            hf_rotation,
            ha_rotation,
            self.envelope.phase_offset,
        );
        
        // Apply rotational projection
        let phenotype_register = decoded_envelope.apply_rotational(live);
        
        LivePhenotype {
            register_id: derive_register_id(
                self.lane_binding,
                self.id,
                &self.crypto_binding[..],
                InterfaceKind::Application,
                &self.crypto_binding,
            ),
            envelope: decoded_envelope,
            state: DualState::new(live, phenotype_register),
            routing_lane: self.lane_binding,
        }
    }
}

/// Clive = live phenotype (decoded routing lanes, vFPGA mappings, network bindings)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LivePhenotype {
    pub register_id: RegisterId,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub routing_lane: RegisterLane,
}

impl LivePhenotype {
    pub fn encode_back_to_twin(&self, id: u64, generation: u32) -> TwinGenome {
        TwinGenome::seed(
            id,
            generation,
            self.envelope,
            self.register_id.as_bytes(),
            self.state.live.as_bytes(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateGenome {
    pub id: u64,
    pub generation: u32,
    pub register_id: RegisterId,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub lineage_hash: [u8; REGISTER_BYTES],
}

impl CandidateGenome {
    pub fn seed(
        id: u64,
        generation: u32,
        envelope: PhaseEnvelope,
        node_public_key: &[u8; REGISTER_BYTES],
        seed_material: &[u8],
    ) -> Self {
        let live = register_from_entropy(b"candidate-seed", id, generation, seed_material);
        Self::from_live(
            id,
            generation,
            envelope,
            node_public_key,
            seed_material,
            live,
        )
    }

    pub fn from_live(
        id: u64,
        generation: u32,
        envelope: PhaseEnvelope,
        node_public_key: &[u8; REGISTER_BYTES],
        lineage_material: &[u8],
        live: Register,
    ) -> Self {
        let register_id = derive_register_id(
            RegisterLane::GenericEndpoint,
            id,
            &lineage_address(id, generation, lineage_material),
            InterfaceKind::Application,
            node_public_key,
        );
        let state = DualState::from_live(live, envelope);
        let lineage_hash = lineage_hash(id, generation, lineage_material, state);
        Self {
            id,
            generation,
            register_id,
            envelope,
            state,
            lineage_hash,
        }
    }

    pub fn validate(&self) -> Result<(), EvolverError> {
        if self.state.twin != phase_project(self.state.live, self.envelope) {
            return Err(EvolverError::InvalidTwinProjection);
        }
        self.envelope.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FitnessTarget {
    pub desired_live: Register,
    pub novelty_basis: Register,
    pub correctness_weight: f32,
    pub novelty_weight: f32,
    pub simplicity_weight: f32,
    pub reversibility_weight: f32,
}

impl FitnessTarget {
    pub fn balanced(desired_live: Register, novelty_basis: Register) -> Self {
        Self {
            desired_live,
            novelty_basis,
            correctness_weight: 0.45,
            novelty_weight: 0.25,
            simplicity_weight: 0.15,
            reversibility_weight: 0.15,
        }
    }

    pub fn validate(&self) -> Result<(), EvolverError> {
        for value in [
            self.correctness_weight,
            self.novelty_weight,
            self.simplicity_weight,
            self.reversibility_weight,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(EvolverError::InvalidFitnessWeights);
            }
        }
        let sum = self.correctness_weight
            + self.novelty_weight
            + self.simplicity_weight
            + self.reversibility_weight;
        if sum <= f32::EPSILON {
            return Err(EvolverError::InvalidFitnessWeights);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FitnessScore {
    pub total: f32,
    pub correctness: f32,
    pub novelty: f32,
    pub simplicity: f32,
    pub reversibility: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoredCandidate {
    pub genome: CandidateGenome,
    pub score: FitnessScore,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetaGaPolicy {
    pub mutation_rate: f32,
    pub crossover_rate: f32,
    pub phase_shift_rate: f32,
    pub elitism: usize,
    pub population_limit: usize,
    pub exploration_pressure: f32,
}

impl MetaGaPolicy {
    pub fn new(population_limit: usize) -> Result<Self, EvolverError> {
        let policy = Self {
            mutation_rate: 0.08,
            crossover_rate: 0.65,
            phase_shift_rate: 0.05,
            elitism: 2,
            population_limit,
            exploration_pressure: 0.25,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), EvolverError> {
        if self.population_limit < 2 {
            return Err(EvolverError::PopulationTooSmall {
                population_limit: self.population_limit,
            });
        }
        if self.elitism == 0 || self.elitism >= self.population_limit {
            return Err(EvolverError::InvalidPolicy(
                "elitism must be in 1..population_limit",
            ));
        }
        for (name, value) in [
            ("mutation_rate", self.mutation_rate),
            ("crossover_rate", self.crossover_rate),
            ("phase_shift_rate", self.phase_shift_rate),
            ("exploration_pressure", self.exploration_pressure),
        ] {
            if !(0.0..=1.0).contains(&value) || !value.is_finite() {
                return Err(EvolverError::InvalidPolicy(name));
            }
        }
        Ok(())
    }

    pub fn adapt(&mut self, previous_best: Option<f32>, current_best: f32, diversity: f32) {
        let improvement = previous_best
            .map(|previous| current_best - previous)
            .unwrap_or(current_best);
        if improvement < 0.005 {
            self.mutation_rate = (self.mutation_rate * 1.18 + 0.01).min(0.45);
            self.exploration_pressure = (self.exploration_pressure * 1.12 + 0.02).min(0.75);
        } else {
            self.mutation_rate = (self.mutation_rate * 0.92).max(0.015);
            self.exploration_pressure = (self.exploration_pressure * 0.95).max(0.05);
        }
        if diversity < 0.18 {
            self.crossover_rate = (self.crossover_rate * 0.92).max(0.35);
            self.phase_shift_rate = (self.phase_shift_rate + 0.03).min(0.35);
        } else {
            self.crossover_rate = (self.crossover_rate * 1.03).min(0.9);
            self.phase_shift_rate = (self.phase_shift_rate * 0.97).max(0.01);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionRound {
    pub generation: u32,
    pub ranked: Vec<ScoredCandidate>,
    pub policy: MetaGaPolicy,
    pub diversity: f32,
}

pub struct MetaGaEvolver {
    node_public_key: [u8; REGISTER_BYTES],
    envelope: PhaseEnvelope,
    target: FitnessTarget,
    policy: MetaGaPolicy,
    generation: u32,
    next_id: u64,
    previous_best: Option<f32>,
}

impl MetaGaEvolver {
    pub fn new(
        node_public_key: [u8; REGISTER_BYTES],
        envelope: PhaseEnvelope,
        target: FitnessTarget,
        policy: MetaGaPolicy,
    ) -> Result<Self, EvolverError> {
        envelope.validate()?;
        target.validate()?;
        policy.validate()?;
        Ok(Self {
            node_public_key,
            envelope,
            target,
            policy,
            generation: 0,
            next_id: 0,
            previous_best: None,
        })
    }

    pub fn seed_population(&mut self, seed_material: &[u8]) -> Vec<CandidateGenome> {
        (0..self.policy.population_limit)
            .map(|index| {
                let id = self.allocate_id();
                CandidateGenome::seed(
                    id,
                    self.generation,
                    self.envelope,
                    &self.node_public_key,
                    &seed_with_index(seed_material, index as u64),
                )
            })
            .collect()
    }

    pub fn evolve_round(
        &mut self,
        population: &[CandidateGenome],
        round_seed: &[u8],
    ) -> Result<EvolutionRound, EvolverError> {
        if population.len() < 2 {
            return Err(EvolverError::PopulationTooSmall {
                population_limit: population.len(),
            });
        }
        self.policy.validate()?;
        for candidate in population {
            candidate.validate()?;
        }

        let mut ranked = self.rank(population)?;
        let diversity = population_diversity(population);
        let current_best = ranked[0].score.total;
        self.policy
            .adapt(self.previous_best, current_best, diversity);
        self.previous_best = Some(current_best);

        let mut next_population = ranked
            .iter()
            .take(self.policy.elitism)
            .map(|candidate| candidate.genome.clone())
            .collect::<Vec<_>>();

        while next_population.len() < self.policy.population_limit {
            let child_index = next_population.len() as u64;
            let parent_a = &ranked[(child_index as usize) % ranked.len()].genome;
            let parent_b = &ranked[((child_index as usize) + 1) % ranked.len()].genome;
            let mut child = if chance(
                round_seed,
                self.generation,
                child_index,
                b"crossover",
                self.policy.crossover_rate,
            ) {
                self.crossover(parent_a, parent_b, round_seed, child_index)?
            } else {
                self.clone_as_child(parent_a, round_seed, child_index)?
            };
            if chance(
                round_seed,
                self.generation,
                child_index,
                b"mutation",
                self.policy.mutation_rate + self.policy.exploration_pressure * 0.1,
            ) {
                child = self.mutate(&child, round_seed, child_index)?;
            }
            next_population.push(child);
        }

        self.generation += 1;
        ranked = self.rank(&next_population)?;
        Ok(EvolutionRound {
            generation: self.generation,
            ranked,
            policy: self.policy.clone(),
            diversity,
        })
    }

    pub fn rank(
        &self,
        population: &[CandidateGenome],
    ) -> Result<Vec<ScoredCandidate>, EvolverError> {
        let mut scored = population
            .iter()
            .map(|genome| {
                genome.validate()?;
                Ok(ScoredCandidate {
                    genome: genome.clone(),
                    score: evaluate_fitness(genome, &self.target)?,
                })
            })
            .collect::<Result<Vec<_>, EvolverError>>()?;
        scored.sort_by(|left, right| {
            right
                .score
                .total
                .total_cmp(&left.score.total)
                .then_with(|| left.genome.id.cmp(&right.genome.id))
        });
        Ok(scored)
    }

    fn crossover(
        &mut self,
        left: &CandidateGenome,
        right: &CandidateGenome,
        seed: &[u8],
        child_index: u64,
    ) -> Result<CandidateGenome, EvolverError> {
        let mask = register_from_entropy(b"crossover-mask", child_index, self.generation, seed);
        let live = left
            .state
            .live
            .and(&mask)
            .xor(&right.state.live.and(&mask.xor(&Register::all_ones())));
        let envelope = self.child_envelope(left.envelope, right.envelope, seed, child_index)?;
        Ok(CandidateGenome::from_live(
            self.allocate_id(),
            self.generation + 1,
            envelope,
            &self.node_public_key,
            &lineage_pair(left, right, seed),
            live,
        ))
    }

    fn clone_as_child(
        &mut self,
        parent: &CandidateGenome,
        seed: &[u8],
        child_index: u64,
    ) -> Result<CandidateGenome, EvolverError> {
        let envelope = self.child_envelope(parent.envelope, parent.envelope, seed, child_index)?;
        Ok(CandidateGenome::from_live(
            self.allocate_id(),
            self.generation + 1,
            envelope,
            &self.node_public_key,
            &lineage_single(parent, seed),
            parent.state.live,
        ))
    }

    fn mutate(
        &mut self,
        parent: &CandidateGenome,
        seed: &[u8],
        child_index: u64,
    ) -> Result<CandidateGenome, EvolverError> {
        let mutation_mask =
            mutation_mask(seed, parent.id, self.generation, self.policy.mutation_rate);
        let live = parent.state.live.xor(&mutation_mask);
        let envelope = if chance(
            seed,
            self.generation,
            child_index,
            b"phase-shift",
            self.policy.phase_shift_rate,
        ) {
            PhaseEnvelope::new(
                parent.envelope.coherence_sig.rotate_left(7) ^ parent.id,
                parent.envelope.alpha,
                parent.envelope.beta,
                parent.envelope.resuperposition_n.saturating_add(1),
            )?
        } else {
            parent.envelope
        };
        Ok(CandidateGenome::from_live(
            self.allocate_id(),
            self.generation + 1,
            envelope,
            &self.node_public_key,
            &lineage_single(parent, seed),
            live,
        ))
    }

    fn child_envelope(
        &self,
        left: PhaseEnvelope,
        right: PhaseEnvelope,
        seed: &[u8],
        child_index: u64,
    ) -> Result<PhaseEnvelope, EvolverError> {
        if chance(
            seed,
            self.generation,
            child_index,
            b"inherit-right-phase",
            0.5,
        ) {
            Ok(right)
        } else {
            Ok(left)
        }
    }

    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

pub fn evaluate_fitness(
    genome: &CandidateGenome,
    target: &FitnessTarget,
) -> Result<FitnessScore, EvolverError> {
    genome.validate()?;
    target.validate()?;
    let correctness = 1.0 - normalized_hamming(genome.state.live, target.desired_live);
    let novelty = normalized_hamming(genome.state.live, target.novelty_basis);
    let simplicity = 1.0 - bit_density(genome.state.live);
    let reversibility = if genome.state.live.xor(&genome.state.live) == Register::zero()
        && genome.state.twin.xor(&genome.state.twin) == Register::zero()
    {
        1.0
    } else {
        0.0
    };
    let weight_sum = target.correctness_weight
        + target.novelty_weight
        + target.simplicity_weight
        + target.reversibility_weight;
    let total = (correctness * target.correctness_weight
        + novelty * target.novelty_weight
        + simplicity * target.simplicity_weight
        + reversibility * target.reversibility_weight)
        / weight_sum;
    Ok(FitnessScore {
        total,
        correctness,
        novelty,
        simplicity,
        reversibility,
    })
}

pub fn population_diversity(population: &[CandidateGenome]) -> f32 {
    if population.len() < 2 {
        return 0.0;
    }
    let mut total = 0.0;
    let mut pairs = 0usize;
    for left in 0..population.len() {
        for right in left + 1..population.len() {
            total += normalized_hamming(population[left].state.live, population[right].state.live);
            pairs += 1;
        }
    }
    total / pairs as f32
}

fn mutation_mask(seed: &[u8], id: u64, generation: u32, mutation_rate: f32) -> Register {
    let entropy = register_from_entropy(b"mutation-mask", id, generation, seed);
    let threshold = (mutation_rate.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut mask = [0u8; REGISTER_BYTES];
    for (index, byte) in entropy.as_bytes().iter().copied().enumerate() {
        let mut out = 0u8;
        for bit in 0..8 {
            let lane = byte.rotate_left(bit) ^ (index as u8).wrapping_mul(17);
            if lane <= threshold {
                out |= 1 << bit;
            }
        }
        mask[index] = out;
    }
    Register::from_bytes(mask)
}

fn register_from_entropy(label: &[u8], id: u64, generation: u32, material: &[u8]) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(label);
    hasher.update(&id.to_le_bytes());
    hasher.update(&generation.to_le_bytes());
    hasher.update(material);
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn chance(seed: &[u8], generation: u32, index: u64, label: &[u8], probability: f32) -> bool {
    let mut hasher = blake3::Hasher::new();
    hasher.update(label);
    hasher.update(seed);
    hasher.update(&generation.to_le_bytes());
    hasher.update(&index.to_le_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&hash.as_bytes()[..4]);
    let value = u32::from_le_bytes(bytes) as f32 / u32::MAX as f32;
    value <= probability.clamp(0.0, 1.0)
}

fn normalized_hamming(left: Register, right: Register) -> f32 {
    let distance = left
        .as_bytes()
        .iter()
        .zip(right.as_bytes())
        .map(|(left, right)| (left ^ right).count_ones())
        .sum::<u32>();
    distance as f32 / 256.0
}

fn bit_density(register: Register) -> f32 {
    register
        .as_bytes()
        .iter()
        .map(|byte| byte.count_ones())
        .sum::<u32>() as f32
        / 256.0
}

fn seed_with_index(seed: &[u8], index: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(seed.len() + 8);
    bytes.extend_from_slice(seed);
    bytes.extend_from_slice(&index.to_le_bytes());
    bytes
}

fn lineage_address(id: u64, generation: u32, material: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(material.len() + 12);
    bytes.extend_from_slice(b"candidate:");
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes.extend_from_slice(material);
    bytes
}

fn lineage_hash(
    id: u64,
    generation: u32,
    material: &[u8],
    state: DualState,
) -> [u8; REGISTER_BYTES] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&id.to_le_bytes());
    hasher.update(&generation.to_le_bytes());
    hasher.update(material);
    hasher.update(state.live.as_bytes());
    hasher.update(state.twin.as_bytes());
    *hasher.finalize().as_bytes()
}

fn lineage_pair(left: &CandidateGenome, right: &CandidateGenome, seed: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REGISTER_BYTES * 2 + seed.len());
    bytes.extend_from_slice(&left.lineage_hash);
    bytes.extend_from_slice(&right.lineage_hash);
    bytes.extend_from_slice(seed);
    bytes
}

fn lineage_single(parent: &CandidateGenome, seed: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REGISTER_BYTES + seed.len());
    bytes.extend_from_slice(&parent.lineage_hash);
    bytes.extend_from_slice(seed);
    bytes
}

#[derive(Debug, thiserror::Error)]
pub enum EvolverError {
    #[error("candidate twin projection does not match phase envelope")]
    InvalidTwinProjection,
    #[error("invalid fitness weights")]
    InvalidFitnessWeights,
    #[error("population requires at least two candidates, got {population_limit}")]
    PopulationTooSmall { population_limit: usize },
    #[error("invalid policy: {0}")]
    InvalidPolicy(&'static str),
    #[error("hyperangular vector contains non-finite values")]
    InvalidHaVector,
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
}
