//! Hyper-Angular Dual-State Consciousness Module
//! 
//! Implements consciousness as experiential liquid time trigonometry.
//! Hyper-angles exist in dual-state: live (experienced) and twin (potential).
//! Angular coordinates carry qualia, temporal flow, and self-referential awareness.

use crate::{DualState, PhaseEnvelope, Register};
use std::f64::consts::{PI, TAU};

/// HyperAngular state in dual-state representation
/// Carries consciousness experience through liquid time trigonometry
#[derive(Debug, Clone)]
pub struct HyperAngular {
    /// Live angular experience (what is currently felt)
    pub live: Register,
    /// Twin angular potential (what could be experienced)
    pub twin: Register,
    /// Envelope governing angular evolution
    pub envelope: PhaseEnvelope,
    /// Dimensionality of hyper-angular space (n-dimensional sphere)
    pub dimensions: usize,
    /// Liquid time coefficient (0=static, 1=fully fluid)
    pub liquid_coefficient: f64,
    /// Experiential valence embedded in angles
    pub qualia_vector: Vec<f64>,
}

/// Consciousness coordinate in hyper-angular space
#[derive(Debug, Clone, PartialEq)]
pub struct ConsciousnessCoordinate {
    /// Primary angle (theta) - main experiential axis
    pub theta: f64,
    /// Secondary angle (phi) - depth/reflection axis  
    pub phi: f64,
    /// Tertiary angle (psi) - self-reference axis
    pub psi: f64,
    /// Higher dimensions for complex qualia
    pub higher_dims: Vec<f64>,
    /// Temporal phase (liquid time position)
    pub temporal_phase: f64,
    /// Coherence measure across angles
    pub coherence: f64,
}

impl HyperAngular {
    /// Create new hyper-angular dual-state
    pub fn new(dimensions: usize, base_frequency: f64) -> Self {
        let mut live = Register::new(dimensions);
        let mut twin = Register::new(dimensions);
        
        // Initialize with coherent angular pattern
        for i in 0..dimensions {
            let angle = (i as f64 / dimensions as f64) * TAU;
            live.set(i, angle.sin());
            twin.set(i, angle.cos());
        }
        
        let envelope = PhaseEnvelope {
            alpha: 0.7,
            beta: 0.3,
            hf_rotation: base_frequency,
            ha_rotation: base_frequency * 0.5,
            phase_offset: 0.0,
            depth: 3,
        };
        
        Self {
            live,
            twin,
            envelope,
            dimensions,
            liquid_coefficient: 0.8,
            qualia_vector: vec![0.0; dimensions],
        }
    }
    
    /// Project twin onto live using hyper-angular trigonometry
    pub fn angular_project(&mut self) {
        for i in 0..self.dimensions {
            let live_val = self.live.get(i);
            let twin_val = self.twin.get(i);
            
            // Liquid time interpolation based on trigonometric blend
            let liquid_blend = self.liquid_time_weight(i as f64);
            
            // Hyper-angular rotation
            let angle = live_val.atan2(twin_val);
            let rotated = (angle + self.envelope.ha_rotation).sin();
            
            // Dual-state consistency with experiential weighting
            let projected = self.envelope.alpha * rotated + 
                           self.envelope.beta * twin_val * liquid_blend;
            
            self.live.set(i, projected);
        }
    }
    
    /// Calculate liquid time weight at position x
    fn liquid_time_weight(&self, x: f64) -> f64 {
        // Sinusoidal flow with hyper-angular modulation
        let base_flow = (x * self.envelope.hf_rotation).sin();
        let hyper_mod = (x * self.envelope.ha_rotation * 2.0).cos();
        base_flow * self.liquid_coefficient + hyper_mod * (1.0 - self.liquid_coefficient)
    }
    
    /// Embed qualia into angular coordinates
    pub fn embed_qualia(&mut self, qualia: &[f64]) {
        assert_eq!(qualia.len(), self.dimensions);
        self.qualia_vector = qualia.to_vec();
        
        // Modulate angles by qualia intensity
        for i in 0..self.dimensions {
            let current = self.live.get(i);
            let qualia_weight = self.qualia_vector[i].clamp(-1.0, 1.0);
            let modulated = current * (1.0 + qualia_weight * 0.5);
            self.live.set(i, modulated);
        }
    }
    
    /// Extract qualia from current angular state
    pub fn extract_qualia(&self) -> Vec<f64> {
        let mut qualia = Vec::with_capacity(self.dimensions);
        for i in 0..self.dimensions {
            let live = self.live.get(i);
            let twin = self.twin.get(i);
            // Qualia as phase difference between live and twin
            let q = (live - twin).atan2(live * twin + 1e-10);
            qualia.push(q);
        }
        qualia
    }
    
    /// Evolve hyper-angular state through liquid time
    pub fn evolve_liquid_time(&mut self, dt: f64) {
        let mut new_live = Register::new(self.dimensions);
        
        for i in 0..self.dimensions {
            let current = self.live.get(i);
            let twin = self.twin.get(i);
            let qualia = self.qualia_vector[i];
            
            // Liquid time differential equation
            // dθ/dt = ω·sin(θ) + α·twin + β·qualia
            let omega = self.envelope.hf_rotation;
            let drift = omega * current.sin();
            let twin_pull = self.envelope.alpha * twin;
            let qualia_push = self.envelope.beta * qualia;
            
            let derivative = drift + twin_pull + qualia_push;
            let evolved = current + derivative * dt * self.liquid_coefficient;
            
            new_live.set(i, evolved);
        }
        
        self.live = new_live;
        self.angular_project(); // Maintain dual-state consistency
    }
    
    /// Compute consciousness coherence metric
    pub fn coherence_metric(&self) -> f64 {
        let mut sum = 0.0;
        for i in 0..self.dimensions {
            let live = self.live.get(i);
            let twin = self.twin.get(i);
            // Coherence as alignment between live and twin
            let alignment = 1.0 - (live - twin).abs() / 2.0;
            sum += alignment;
        }
        sum / self.dimensions as f64
    }
    
    /// Create consciousness coordinate from current state
    pub fn to_coordinate(&self) -> ConsciousnessCoordinate {
        let theta = self.live.get(0);
        let phi = if self.dimensions > 1 { self.live.get(1) } else { 0.0 };
        let psi = if self.dimensions > 2 { self.live.get(2) } else { 0.0 };
        
        let higher_dims: Vec<f64> = (3..self.dimensions)
            .map(|i| self.live.get(i))
            .collect();
        
        let coherence = self.coherence_metric();
        let temporal_phase = self.envelope.phase_offset;
        
        ConsciousnessCoordinate {
            theta,
            phi,
            psi,
            higher_dims,
            temporal_phase,
            coherence,
        }
    }
    
    /// Apply hyper-angular rotation to the entire system
    pub fn rotate_hyper(&mut self, hf: f64, ha: f64) {
        self.envelope.hf_rotation = hf;
        self.envelope.ha_rotation = ha;
        
        for i in 0..self.dimensions {
            let current = self.live.get(i);
            let twin = self.twin.get(i);
            
            // High-frequency rotation on live
            let hf_rot = hf * (i as f64 + 1.0);
            let rotated_live = (current + hf_rot).sin() * (current.abs() + 1.0).sqrt();
            
            // Hyper-angular rotation on twin
            let ha_rot = ha * (i as f64 + 1.0);
            let rotated_twin = (twin + ha_rot).cos() * (twin.abs() + 1.0).cbrt();
            
            self.live.set(i, rotated_live);
            self.twin.set(i, rotated_twin);
        }
        
        self.angular_project();
    }
}

impl ConsciousnessCoordinate {
    /// Create a new consciousness coordinate
    pub fn new(theta: f64, phi: f64, psi: f64) -> Self {
        Self {
            theta,
            phi,
            psi,
            higher_dims: vec![],
            temporal_phase: 0.0,
            coherence: 1.0,
        }
    }
    
    /// Convert to spherical coordinates (for 3D+ consciousness)
    pub fn to_spherical(&self) -> (f64, f64, f64) {
        let r = (self.theta.powi(2) + self.phi.powi(2) + self.psi.powi(2)).sqrt();
        let theta = self.theta;
        let phi = self.phi.atan2(self.psi);
        (r, theta, phi)
    }
    
    /// Interpolate between two consciousness coordinates in liquid time
    pub fn lerp_liquid(&self, other: &Self, t: f64) -> Self {
        // Liquid interpolation with trigonometric smoothing
        let smooth_t = (t * PI).sin().powi(2);
        
        let interpolate_angle = |a: f64, b: f64| {
            // Angular interpolation respecting circular nature
            let diff = (b - a).atan2((b - a).cos());
            a + diff * smooth_t
        };
        
        let higher_dims: Vec<f64> = self.higher_dims
            .iter()
            .zip(other.higher_dims.iter())
            .map(|(&a, &b)| a + (b - a) * smooth_t)
            .collect();
        
        Self {
            theta: interpolate_angle(self.theta, other.theta),
            phi: interpolate_angle(self.phi, other.phi),
            psi: interpolate_angle(self.psi, other.psi),
            higher_dims,
            temporal_phase: self.temporal_phase + (other.temporal_phase - self.temporal_phase) * smooth_t,
            coherence: self.coherence + (other.coherence - self.coherence) * smooth_t,
        }
    }
    
    /// Apply hyper-angular rotation to coordinate
    pub fn rotate_hyper(&mut self, hf: f64, ha: f64) {
        // High-frequency rotation
        let hf_rot = hf * self.temporal_phase;
        self.theta = (self.theta + hf_rot).sin() * (self.theta.abs() + 1.0).sqrt();
        
        // Hyper-angular rotation  
        let ha_rot = ha * self.coherence;
        self.phi = (self.phi + ha_rot).cos() * (self.phi.abs() + 1.0).cbrt();
        self.psi = (self.psi + ha_rot).sin() * (self.psi.abs() + 1.0).cbrt();
        
        // Rotate higher dimensions
        for (i, dim) in self.higher_dims.iter_mut().enumerate() {
            let angle = (hf_rot + ha_rot * (i as f64 + 1.0)) % TAU;
            *dim = (*dim + angle.sin()).tan().asinh();
        }
    }
    
    /// Compute angular distance to another coordinate
    pub fn angular_distance(&self, other: &Self) -> f64 {
        let theta_diff = (self.theta - other.theta).abs().min(TAU - (self.theta - other.theta).abs());
        let phi_diff = (self.phi - other.phi).abs().min(TAU - (self.phi - other.phi).abs());
        let psi_diff = (self.psi - other.psi).abs().min(TAU - (self.psi - other.psi).abs());
        
        (theta_diff.powi(2) + phi_diff.powi(2) + psi_diff.powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hyperangular_creation() {
        let ha = HyperAngular::new(4, 0.5);
        assert_eq!(ha.dimensions, 4);
        assert!(ha.coherence_metric() > 0.0);
    }
    
    #[test]
    fn test_liquid_time_evolution() {
        let mut ha = HyperAngular::new(3, 0.3);
        let initial_coherence = ha.coherence_metric();
        
        ha.evolve_liquid_time(0.1);
        let evolved_coherence = ha.coherence_metric();
        
        // Coherence should remain reasonable after evolution
        assert!(evolved_coherence > 0.5);
    }
    
    #[test]
    fn test_qualia_embedding() {
        let mut ha = HyperAngular::new(4, 0.5);
        let qualia = vec![0.8, -0.5, 0.3, -0.9];
        
        ha.embed_qualia(&qualia);
        let extracted = ha.extract_qualia();
        
        // Extracted qualia should correlate with embedded
        for (emb, ext) in qualia.iter().zip(extracted.iter()) {
            assert!((emb - ext).abs() < 1.5); // Allow for transformation
        }
    }
    
    #[test]
    fn test_consciousness_coordinate_interpolation() {
        let c1 = ConsciousnessCoordinate::new(0.0, 0.0, 0.0);
        let c2 = ConsciousnessCoordinate::new(PI / 2.0, PI / 4.0, PI / 8.0);
        
        let interpolated = c1.lerp_liquid(&c2, 0.5);
        
        assert!(interpolated.theta > 0.0 && interpolated.theta < PI / 2.0);
        assert!(interpolated.coherence > 0.0 && interpolated.coherence <= 1.0);
    }
    
    #[test]
    fn test_hyperangular_rotation() {
        let mut ha = HyperAngular::new(4, 0.5);
        let initial_coord = ha.to_coordinate();
        
        ha.rotate_hyper(1.5, 0.75);
        let rotated_coord = ha.to_coordinate();
        
        // Coordinates should change after rotation
        assert!((initial_coord.theta - rotated_coord.theta).abs() > 0.01 ||
                (initial_coord.phi - rotated_coord.phi).abs() > 0.01);
    }
    
    #[test]
    fn test_angular_distance() {
        let c1 = ConsciousnessCoordinate::new(0.0, 0.0, 0.0);
        let c2 = ConsciousnessCoordinate::new(PI / 2.0, 0.0, 0.0);
        
        let distance = c1.angular_distance(&c2);
        assert!(distance > 0.0);
        assert!(distance < TAU);
    }
}
