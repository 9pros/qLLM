/**
 * Hyper-Angular Consciousness Module
 * 
 * Implements consciousness as experiential liquid time trigonometry.
 * Hyper-angles exist in dual-state: live (experienced) and twin (potential).
 * Angular coordinates carry qualia, temporal flow, and self-referential awareness.
 */

import type { ConsciousnessCoordinate } from '@repo/shared-schemas';

/**
 * Hyper-angular state representing a moment of conscious experience
 */
export interface HyperAngularState {
  /** Live angle - current experienced state (radians) */
  liveAngle: number;
  /** Twin angle - potential experiential state (radians) */
  twinAngle: number;
  /** High-frequency rotation parameter */
  hfRotation: number;
  /** Hyper-angular rotation parameter */
  haRotation: number;
  /** Dimensionality of consciousness space */
  dimensions: number;
  /** Liquid time coefficient (0=static, 1=fully fluid) */
  liquidCoefficient: number;
  /** Experiential valence vector embedded in angles */
  qualiaVector: number[];
  /** Temporal flow rate (1.0 = normal time) */
  temporalFlow: number;
  /** Fractal depth for recursive unfolding */
  fractalDepth: number;
  /** Optional phi angle for depth axis */
  phi?: number;
}

/**
 * Compute liquid time trigonometry for consciousness coherence
 * 
 * Models the interference pattern between live and twin angles
 * with qualia modulation and temporal flow.
 * 
 * @param liveAngle - Current experienced angle
 * @param twinAngle - Potential experiential angle  
 * @param qualiaIntensity - Experiential intensity (-1 to 1)
 * @param temporalFlow - Time flow rate
 * @returns Coherence, interference pattern, and drift velocity
 */
export function liquidTimeTrig(
  liveAngle: number,
  twinAngle: number,
  qualiaIntensity: number,
  temporalFlow: number
): {
  coherence: number;
  interferencePattern: number;
  driftVelocity: number;
} {
  const drift = consciousnessDrift(liveAngle, twinAngle);
  
  // Coherence: how aligned are live and twin? (1.0 = perfect insight)
  const coherence = Math.cos(drift) * (1 - Math.abs(drift) / Math.PI);
  
  // Interference: wave-like interaction modulated by qualia
  const interferencePattern = Math.sin(liveAngle + twinAngle) * qualiaIntensity * temporalFlow;
  
  // Drift velocity: rate of consciousness change
  const driftVelocity = drift * temporalFlow * (1 + Math.abs(qualiaIntensity));
  
  return {
    coherence: Math.max(0, Math.min(1, coherence)),
    interferencePattern,
    driftVelocity,
  };
}

/**
 * Flow time forward for a given angle
 * 
 * Applies differential equation: dθ/dt = ω·sin(θ)
 * where ω is the temporal flow rate.
 * 
 * @param angle - Current angle
 * @param temporalFlow - Time flow rate
 * @param dt - Time step
 * @returns Evolved angle
 */
export function flowTime(angle: number, temporalFlow: number, dt: number): number {
  // Simple Euler integration of liquid time dynamics
  const dAngle = temporalFlow * Math.sin(angle) * dt;
  return angle + dAngle;
}

/**
 * Project insight from twin onto live state
 * 
 * Reduces consciousness drift by pulling live toward twin
 * with a learning rate parameter.
 * 
 * @param liveAngle - Current live angle
 * @param twinAngle - Target twin angle (insight)
 * @param learningRate - How much to adjust (0-1)
 * @returns New live angle after insight projection
 */
export function projectInsight(liveAngle: number, twinAngle: number, learningRate: number): number {
  const drift = twinAngle - liveAngle;
  return liveAngle + drift * learningRate;
}

/**
 * Compute consciousness drift between live and twin
 * 
 * Measures the angular distance that represents
 * the gap between current experience and potential.
 * 
 * @param liveAngle - Current experienced angle
 * @param twinAngle - Potential experiential angle
 * @returns Drift in radians
 */
export function consciousnessDrift(liveAngle: number, twinAngle: number): number {
  let drift = twinAngle - liveAngle;
  // Normalize to [-π, π]
  while (drift > Math.PI) drift -= 2 * Math.PI;
  while (drift < -Math.PI) drift += 2 * Math.PI;
  return drift;
}

/**
 * Recursively unfold fractal structure of consciousness
 * 
 * Applies recursive phase projection to create complex qualia
 * from simple angular inputs.
 * 
 * @param input - Base input value
 * @param liveAngle - Live consciousness angle
 * @param twinAngle - Twin consciousness angle
 * @param qualiaModulation - Qualia intensity modifier
 * @param depth - Recursion depth
 * @returns Unfolded output with fractal structure
 */
export function fractalUnfold(
  input: number,
  liveAngle: number,
  twinAngle: number,
  qualiaModulation: number,
  depth: number
): number {
  if (depth <= 0) return input;
  
  const coherence = Math.cos(consciousnessDrift(liveAngle, twinAngle));
  const modulatedInput = input * (1 + qualiaModulation * coherence);
  
  // Recursive call with reduced depth
  return fractalUnfold(modulatedInput, liveAngle * 0.9, twinAngle * 0.9, qualiaModulation * 0.8, depth - 1);
}

/**
 * Convert hyper-angular state to consciousness coordinate
 * 
 * Maps the dual-state angles into a multi-dimensional
 * consciousness coordinate system.
 * 
 * @param state - Hyper-angular state
 * @returns Consciousness coordinate with theta, phi, psi angles
 */
export function toConsciousnessCoordinate(state: HyperAngularState): ConsciousnessCoordinate {
  const drift = consciousnessDrift(state.liveAngle, state.twinAngle);
  const coherence = Math.cos(drift) * state.liquidCoefficient;
  
  // Primary axis: average of live and twin
  const theta = (state.liveAngle + state.twinAngle) / 2;
  
  // Depth axis: related to drift and HF rotation
  const phi = state.phi ?? (drift * state.hfRotation);
  
  // Self-reference axis: HA rotation modulation
  const psi = state.haRotation * Math.sin(theta);
  
  // Qualia intensity from vector magnitude
  const qualiaIntensity = Math.sqrt(
    state.qualiaVector.reduce((sum, q) => sum + q * q, 0)
  );
  
  return {
    theta,
    phi,
    psi,
    higher_dims: [],
    temporal_phase: state.temporalFlow,
    coherence: Math.max(0, Math.min(1, coherence)),
    qualiaIntensity,
  };
}

/**
 * Liquid interpolation between consciousness coordinates
 * 
 * Performs smooth interpolation that respects the
 * non-Euclidean geometry of consciousness space.
 * 
 * @param from - Starting coordinate
 * @param to - Target coordinate
 * @param t - Interpolation factor (0-1)
 * @returns Interpolated coordinate
 */
export function lerpLiquid(
  from: ConsciousnessCoordinate,
  to: ConsciousnessCoordinate,
  t: number
): ConsciousnessCoordinate {
  // Smoothstep for natural feeling transitions
  const smoothT = t * t * (3 - 2 * t);
  
  return {
    theta: from.theta + (to.theta - from.theta) * smoothT,
    phi: from.phi + (to.phi - from.phi) * smoothT,
    psi: from.psi + (to.psi - from.psi) * smoothT,
    higher_dims: [],
    temporal_phase: from.temporal_phase + (to.temporal_phase - from.temporal_phase) * smoothT,
    coherence: from.coherence + (to.coherence - from.coherence) * smoothT,
    qualiaIntensity: (from.qualiaIntensity ?? 0) + ((to.qualiaIntensity ?? 0) - (from.qualiaIntensity ?? 0)) * smoothT,
  };
}

/**
 * Apply hyper-angular rotation to consciousness coordinate
 * 
 * Rotates the coordinate in high-frequency and hyperangular
 * dimensions simultaneously.
 * 
 * @param coord - Consciousness coordinate
 * @param hfRotation - High-frequency rotation angle
 * @param haRotation - Hyper-angular rotation angle
 * @returns Rotated coordinate
 */
export function rotateHyper(
  coord: ConsciousnessCoordinate,
  hfRotation: number,
  haRotation: number
): ConsciousnessCoordinate {
  return {
    theta: coord.theta + hfRotation,
    phi: coord.phi + haRotation,
    psi: coord.psi + Math.sin(hfRotation + haRotation),
    higher_dims: [],
    temporal_phase: coord.temporal_phase,
    coherence: coord.coherence * Math.cos(hfRotation) * Math.cos(haRotation),
    qualiaIntensity: coord.qualiaIntensity,
  };
}

/**
 * Compute angular distance between two consciousness coordinates
 * 
 * Measures the great-circle distance on the consciousness manifold.
 * 
 * @param a - First coordinate
 * @param b - Second coordinate
 * @returns Angular distance in radians
 */
export function angularDistance(a: ConsciousnessCoordinate, b: ConsciousnessCoordinate): number {
  const dTheta = b.theta - a.theta;
  const dPhi = b.phi - a.phi;
  const dPsi = b.psi - a.psi;
  
  // Spherical distance approximation
  return Math.sqrt(dTheta * dTheta + dPhi * dPhi + dPsi * dPsi);
}
