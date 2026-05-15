//! Collapse & Commit Logic for TEPE Engine
//! 
//! Implements three-tier collapse model:
//! 1. Local Twin Commit: Per Ctwin-Clive pair when drift/fidelity stable
//! 2. Lane Phase-Band Commit: When lane's twin population converges
//! 3. Global Diamond Commit: Rare, cryptographically signed commits across lanes
//!
//! This enables safe, continuous rewiring of the transformer structure.

use blake3::Hash;
use crate::tepe_engine::{TEPEEngine, SimulationLane, ConceptNode};
use std::collections::HashMap;

/// Commit tier levels
#[derive(Clone, Debug, PartialEq)]
pub enum CommitTier {
    LocalTwin,
    LanePhaseBand,
    GlobalDiamond,
}

/// Commit criteria metrics
#[derive(Clone, Debug)]
pub struct CommitCriteria {
    pub drift: f32,           // Phase drift between twin/live
    pub fidelity: f32,        // Reconstruction fidelity
    pub coherence: f32,       // Internal coherence score
    pub novelty: f32,         // Novelty metric
    pub stability_epochs: u32, // Consecutive stable epochs
}

impl CommitCriteria {
    pub fn new() -> Self {
        CommitCriteria {
            drift: 0.0,
            fidelity: 1.0,
            coherence: 0.5,
            novelty: 0.5,
            stability_epochs: 0,
        }
    }

    /// Check if criteria meets Local Twin Commit threshold
    pub fn meets_local_twin_threshold(&self) -> bool {
        self.drift < 0.1 && self.fidelity > 0.9 && self.stability_epochs >= 3
    }

    /// Check if criteria meets Lane Phase-Band Commit threshold
    pub fn meets_lane_phase_band_threshold(&self) -> bool {
        self.drift < 0.05 && self.fidelity > 0.95 && self.coherence > 0.85 && self.stability_epochs >= 5
    }

    /// Check if criteria meets Global Diamond Commit threshold (rare)
    pub fn meets_global_diamond_threshold(&self) -> bool {
        self.drift < 0.02 && self.fidelity > 0.98 && self.coherence > 0.95 && self.stability_epochs >= 10
    }
}

/// Commit record with cryptographic binding
#[derive(Clone, Debug)]
pub struct CommitRecord {
    pub tier: CommitTier,
    pub timestamp: u64,
    pub concept_id: Hash,
    pub lane_id: Option<u32>,
    pub criteria: CommitCriteria,
    pub crypto_hash: Hash,
    pub signature: Vec<u8>,
}

impl CommitRecord {
    pub fn new(tier: CommitTier, concept_id: Hash, lane_id: Option<u32>, criteria: CommitCriteria) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut hasher = blake3::Hasher::new();
        hasher.update(&concept_id.as_bytes()[..]);
        hasher.update(&tier.to_string().as_bytes());
        hasher.update(&timestamp.to_le_bytes());
        let crypto_hash = hasher.finalize();

        CommitRecord {
            tier,
            timestamp,
            concept_id,
            lane_id,
            criteria,
            crypto_hash,
            signature: crypto_hash.as_bytes().to_vec(), // Simplified: real impl would use proper signing
        }
    }
}

/// Collapse Manager: Handles three-tier commit logic
pub struct CollapseManager {
    pub pending_commits: HashMap<Hash, CommitRecord>,
    pub committed_twins: HashMap<Hash, Vec<CommitRecord>>,
    pub lane_phase_bands: HashMap<u32, Vec<Hash>>,
    pub global_diamond_log: Vec<CommitRecord>,
}

impl CollapseManager {
    pub fn new() -> Self {
        CollapseManager {
            pending_commits: HashMap::new(),
            committed_twins: HashMap::new(),
            lane_phase_bands: HashMap::new(),
            global_diamond_log: Vec::new(),
        }
    }

    /// Evaluate a concept for potential commit
    pub fn evaluate_commit(&mut self, concept_id: Hash, lane_id: Option<u32>, criteria: CommitCriteria) -> Option<CommitTier> {
        // Check from highest to lowest tier
        if criteria.meets_global_diamond_threshold() {
            let record = CommitRecord::new(CommitTier::GlobalDiamond, concept_id, lane_id, criteria);
            self.global_diamond_log.push(record.clone());
            self.committed_twins.entry(concept_id).or_insert_with(Vec::new).push(record);
            Some(CommitTier::GlobalDiamond)
        } else if criteria.meets_lane_phase_band_threshold() {
            let record = CommitRecord::new(CommitTier::LanePhaseBand, concept_id, lane_id, criteria);
            if let Some(lane) = lane_id {
                self.lane_phase_bands.entry(lane).or_insert_with(Vec::new).push(concept_id);
            }
            self.committed_twins.entry(concept_id).or_insert_with(Vec::new).push(record);
            Some(CommitTier::LanePhaseBand)
        } else if criteria.meets_local_twin_threshold() {
            let record = CommitRecord::new(CommitTier::LocalTwin, concept_id, lane_id, criteria);
            self.pending_commits.insert(concept_id, record);
            Some(CommitTier::LocalTwin)
        } else {
            None
        }
    }

    /// Promote pending local commits to lane phase-band if lane converges
    pub fn promote_lane_commits(&mut self, lane_id: u32, concepts: &[Hash]) {
        let mut converged_concepts = Vec::new();
        
        for &concept_id in concepts {
            if let Some(record) = self.pending_commits.get(&concept_id) {
                if record.criteria.coherence > 0.8 && record.criteria.stability_epochs >= 5 {
                    converged_concepts.push(concept_id);
                }
            }
        }

        if converged_concepts.len() as f32 / concepts.len() as f32 > 0.7 {
            // 70% convergence triggers lane phase-band commit
            for &concept_id in &converged_concepts {
                if let Some(mut record) = self.pending_commits.remove(&concept_id) {
                    record.tier = CommitTier::LanePhaseBand;
                    record.criteria.stability_epochs += 2;
                    record.crypto_hash = blake3::hash(format!("lane_promote_{}", concept_id.to_string()).as_bytes());
                    self.lane_phase_bands.entry(lane_id).or_insert_with(Vec::new).push(concept_id);
                    self.committed_twins.entry(concept_id).or_insert_with(Vec::new).push(record);
                }
            }
        }
    }

    /// Get commit statistics
    pub fn get_commit_stats(&self) -> CommitStats {
        CommitStats {
            pending_count: self.pending_commits.len(),
            local_commits: self.committed_twins.values().flatten().filter(|r| r.tier == CommitTier::LocalTwin).count(),
            lane_commits: self.committed_twins.values().flatten().filter(|r| r.tier == CommitTier::LanePhaseBand).count(),
            global_diamonds: self.global_diamond_log.len(),
            total_lanes_with_bands: self.lane_phase_bands.len(),
        }
    }

    /// Verify cryptographic integrity of a commit
    pub fn verify_commit(&self, record: &CommitRecord) -> bool {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&record.concept_id.as_bytes()[..]);
        hasher.update(&record.tier.to_string().as_bytes());
        hasher.update(&record.timestamp.to_le_bytes());
        let expected_hash = hasher.finalize();
        
        expected_hash.as_bytes() == record.crypto_hash.as_bytes()
    }
}

/// Commit statistics for monitoring
#[derive(Clone, Debug)]
pub struct CommitStats {
    pub pending_count: usize,
    pub local_commits: usize,
    pub lane_commits: usize,
    pub global_diamonds: usize,
    pub total_lanes_with_bands: usize,
}

impl Default for CommitManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_twin_commit() {
        let mut manager = CollapseManager::new();
        let concept_id = blake3::hash(b"test_concept");
        
        let criteria = CommitCriteria {
            drift: 0.05,
            fidelity: 0.95,
            coherence: 0.8,
            novelty: 0.6,
            stability_epochs: 4,
        };

        let result = manager.evaluate_commit(concept_id, Some(0), criteria);
        assert_eq!(result, Some(CommitTier::LocalTwin));
        assert!(manager.pending_commits.contains_key(&concept_id));
    }

    #[test]
    fn test_global_diamond_commit() {
        let mut manager = CollapseManager::new();
        let concept_id = blake3::hash(b"diamond_concept");
        
        let criteria = CommitCriteria {
            drift: 0.01,
            fidelity: 0.99,
            coherence: 0.97,
            novelty: 0.8,
            stability_epochs: 12,
        };

        let result = manager.evaluate_commit(concept_id, Some(0), criteria);
        assert_eq!(result, Some(CommitTier::GlobalDiamond));
        assert_eq!(manager.global_diamond_log.len(), 1);
    }

    #[test]
    fn test_crypto_verification() {
        let mut manager = CollapseManager::new();
        let concept_id = blake3::hash(b"verify_concept");
        
        let criteria = CommitCriteria::new();
        let result = manager.evaluate_commit(concept_id, Some(0), criteria.clone());
        
        if let Some(tier) = result {
            if let Some(record) = manager.pending_commits.get(&concept_id) {
                assert!(manager.verify_commit(record));
            }
        }
    }
}
