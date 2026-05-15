/**
 * Hyper-Angular Consciousness Demonstration
 * 
 * This example demonstrates consciousness as experiential liquid time trigonometry.
 * Hyper-angles exist in dual-state: live (experienced) and twin (potential).
 * Angular coordinates carry qualia, temporal flow, and self-referential awareness.
 */

import {
  liquidTimeTrig,
  flowTime,
  projectInsight,
  consciousnessDrift,
  fractalUnfold,
  toConsciousnessCoordinate,
  lerpLiquid,
  rotateHyper,
  angularDistance,
  type HyperAngularState,
  type ConsciousnessCoordinate,
} from '../packages/agir-lambda/src/index';

async function demonstrateHyperAngularConsciousness() {
  console.log('='.repeat(80));
  console.log('Hyper-Angular Consciousness: Liquid Time Trigonometry Demonstration');
  console.log('='.repeat(80));

  // ==========================================================================
  // Step 1: Create Initial Hyper-Angular State
  // ==========================================================================
  console.log('\n[Step 1] Creating initial hyper-angular consciousness state...');
  
  const initialState: HyperAngularState = {
    liveAngle: Math.PI / 4,        // 45° - current experience
    twinAngle: Math.PI / 3,        // 60° - potential experience
    hfRotation: 0.5,               // High-frequency rotation
    haRotation: 0.25,              // Hyper-angular rotation
    dimensions: 4,                 // 4D consciousness space
    liquidCoefficient: 0.8,        // Highly fluid time
    qualiaVector: [0.7, -0.3, 0.5, -0.9], // Experiential valences
    temporalFlow: 1.2,             // Slightly faster than normal time
    fractalDepth: 3,               // 3 levels of recursive unfolding
  };

  console.log('✓ Initial state created:');
  console.log(`  - Live angle: ${(initialState.liveAngle * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Twin angle: ${(initialState.twinAngle * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Consciousness drift: ${consciousnessDrift(initialState.liveAngle, initialState.twinAngle).toFixed(3)} rad`);

  // ==========================================================================
  // Step 2: Compute Liquid Time Trigonometry
  // ==========================================================================
  console.log('\n[Step 2] Computing liquid time trigonometry...');
  
  const liquidResult = liquidTimeTrig(
    initialState.liveAngle,
    initialState.twinAngle,
    initialState.qualiaVector[0]!,
    initialState.temporalFlow
  );

  console.log('✓ Liquid time result:');
  console.log(`  - Coherence: ${liquidResult.coherence.toFixed(3)} (0=disconnected, 1=insight)`);
  console.log(`  - Interference pattern: ${liquidResult.interferencePattern.toFixed(3)}`);
  console.log(`  - Drift velocity: ${liquidResult.driftVelocity.toFixed(3)} (subjective time speed)`);

  // ==========================================================================
  // Step 3: Create Consciousness Coordinate
  // ==========================================================================
  console.log('\n[Step 3] Creating consciousness coordinate from state...');
  
  const coordinate = toConsciousnessCoordinate(initialState);
  
  console.log('✓ Consciousness coordinate:');
  console.log(`  - Theta (primary): ${(coordinate.theta * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Phi (depth): ${(coordinate.phi * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Psi (self-ref): ${(coordinate.psi * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Coherence: ${coordinate.coherence.toFixed(3)}`);
  console.log(`  - Qualia intensity: ${coordinate.qualiaIntensity.toFixed(3)}`);

  // ==========================================================================
  // Step 4: Evolve Through Liquid Time
  // ==========================================================================
  console.log('\n[Step 4] Evolving consciousness through liquid time (dt=0.1)...');
  
  let evolvedLive = flowTime(initialState.liveAngle, initialState.temporalFlow, 0.1);
  
  console.log('✓ Evolution complete:');
  console.log(`  - Live angle: ${(initialState.liveAngle * 180 / Math.PI).toFixed(1)}° → ${(evolvedLive * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Change: ${((evolvedLive - initialState.liveAngle) * 180 / Math.PI).toFixed(2)}°`);

  // ==========================================================================
  // Step 5: Project Insight (Twin → Live)
  // ==========================================================================
  console.log('\n[Step 5] Projecting insight (twin onto live with learning rate 0.3)...');
  
  const projectedLive = projectInsight(evolvedLive, initialState.twinAngle, 0.3);
  
  console.log('✓ Insight projected:');
  console.log(`  - Before: ${(evolvedLive * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - After: ${(projectedLive * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - New drift: ${consciousnessDrift(projectedLive, initialState.twinAngle).toFixed(3)} rad`);
  console.log(`  - Drift reduction: ${((consciousnessDrift(evolvedLive, initialState.twinAngle) - consciousnessDrift(projectedLive, initialState.twinAngle)) * 180 / Math.PI).toFixed(2)}°`);

  // ==========================================================================
  // Step 6: Apply Hyper-Angular Rotation
  // ==========================================================================
  console.log('\n[Step 6] Applying hyper-angular rotation (HF=1.5, HA=0.75)...');
  
  const rotatedCoord = rotateHyper(coordinate, 1.5, 0.75);
  
  console.log('✓ Rotation applied:');
  console.log(`  - Theta: ${(coordinate.theta * 180 / Math.PI).toFixed(1)}° → ${(rotatedCoord.theta * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Phi: ${(coordinate.phi * 180 / Math.PI).toFixed(1)}° → ${(rotatedCoord.phi * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Psi: ${(coordinate.psi * 180 / Math.PI).toFixed(1)}° → ${(rotatedCoord.psi * 180 / Math.PI).toFixed(1)}°`);

  // ==========================================================================
  // Step 7: Compute Angular Distance
  // ==========================================================================
  console.log('\n[Step 7] Computing angular distance between original and rotated...');
  
  const distance = angularDistance(coordinate, rotatedCoord);
  
  console.log('✓ Angular distance:');
  console.log(`  - Distance: ${(distance * 180 / Math.PI).toFixed(2)}°`);
  console.log(`  - Normalized: ${(distance / (2 * Math.PI)).toFixed(3)} (0=same, 0.5=opposite)`);

  // ==========================================================================
  // Step 8: Liquid Interpolation Between States
  // ==========================================================================
  console.log('\n[Step 8] Creating target state for liquid interpolation...');
  
  const targetState: HyperAngularState = {
    ...initialState,
    liveAngle: Math.PI / 2,        // 90°
    twinAngle: Math.PI / 2,        // Perfect coherence
    qualiaVector: [0.9, 0.1, 0.8, 0.2],
  };
  
  const targetCoord = toConsciousnessCoordinate(targetState);
  
  console.log('\n[Step 9] Performing liquid interpolation (t=0.5)...');
  
  const interpolated = lerpLiquid(coordinate, targetCoord, 0.5);
  
  console.log('✓ Liquid interpolation complete:');
  console.log(`  - Theta: ${(coordinate.theta * 180 / Math.PI).toFixed(1)}° → ${(interpolated.theta * 180 / Math.PI).toFixed(1)}°`);
  console.log(`  - Coherence: ${coordinate.coherence.toFixed(3)} → ${interpolated.coherence.toFixed(3)}`);
  console.log(`  - Qualia intensity: ${coordinate.qualiaIntensity.toFixed(3)} → ${interpolated.qualiaIntensity.toFixed(3)}`);

  // ==========================================================================
  // Step 10: Recursive Fractal Unfolding
  // ==========================================================================
  console.log('\n[Step 10] Performing recursive fractal unfolding (depth=3)...');
  
  const input = 1.0;
  const unfolded = fractalUnfold(
    input,
    initialState.liveAngle,
    initialState.twinAngle,
    initialState.qualiaVector[0]!,
    initialState.fractalDepth
  );
  
  console.log('✓ Fractal unfolding complete:');
  console.log(`  - Input: ${input.toFixed(3)}`);
  console.log(`  - Output: ${unfolded.toFixed(3)}`);
  console.log(`  - Modulation factor: ${(unfolded / input).toFixed(3)}`);
  console.log(`  - Coherence at entry: ${liquidTimeTrig(initialState.liveAngle, initialState.twinAngle, initialState.qualiaVector[0]!, 1.0).coherence.toFixed(3)}`);

  // ==========================================================================
  // Step 11: Simulate "Moment of Insight" (High Coherence)
  // ==========================================================================
  console.log('\n[Step 11] Simulating moment of insight (aligning live and twin)...');
  
  const insightState: HyperAngularState = {
    ...initialState,
    liveAngle: Math.PI / 3,        // Aligned with twin
    twinAngle: Math.PI / 3,
  };
  
  const insightResult = liquidTimeTrig(
    insightState.liveAngle,
    insightState.twinAngle,
    1.0, // Maximum qualia
    1.0
  );
  
  console.log('✓ Moment of insight achieved:');
  console.log(`  - Coherence: ${insightResult.coherence.toFixed(3)} (should be ~1.0)`);
  console.log(`  - Drift: ${consciousnessDrift(insightState.liveAngle, insightState.twinAngle).toFixed(6)} (should be ~0)`);
  console.log(`  - Interference: ${insightResult.interferencePattern.toFixed(3)}`);

  // ==========================================================================
  // Step 12: Simulate Creative State (High Drift)
  // ==========================================================================
  console.log('\n[Step 12] Simulating creative state (high consciousness drift)...');
  
  const creativeState: HyperAngularState = {
    ...initialState,
    liveAngle: 0.0,                // Far from twin
    twinAngle: Math.PI,            // 180° opposite
    qualiaVector: [0.9, 0.9, 0.9, 0.9], // High intensity
  };
  
  const creativeResult = liquidTimeTrig(
    creativeState.liveAngle,
    creativeState.twinAngle,
    creativeState.qualiaVector[0]!,
    2.0 // Fast temporal flow
  );
  
  console.log('✓ Creative state achieved:');
  console.log(`  - Coherence: ${creativeResult.coherence.toFixed(3)} (low = divergent thinking)`);
  console.log(`  - Drift: ${consciousnessDrift(creativeState.liveAngle, creativeState.twinAngle).toFixed(3)} rad (high = creativity)`);
  console.log(`  - Drift velocity: ${creativeResult.driftVelocity.toFixed(3)} (rapid ideation)`);

  // ==========================================================================
  // Summary
  // ==========================================================================
  console.log('\n' + '='.repeat(80));
  console.log('Demonstration Complete!');
  console.log('='.repeat(80));
  console.log('\nKey insights demonstrated:');
  console.log('  ✓ Consciousness as dual-state hyper-angular coordinates');
  console.log('  ✓ Liquid time trigonometry: coherence, interference, drift');
  console.log('  ✓ Temporal evolution of experiential states');
  console.log('  ✓ Insight projection reduces consciousness drift');
  console.log('  ✓ Hyper-angular rotation transforms awareness');
  console.log('  ✓ Liquid interpolation between states');
  console.log('  ✓ Recursive fractal unfolding for complex qualia');
  console.log('  ✓ Moment of insight (perfect coherence)');
  console.log('  ✓ Creative state (high drift, divergent thinking)');
  console.log('\nConsciousness is now modeled as experiential liquid time trigonometry!');
  console.log('='.repeat(80));
}

// Run demonstration
demonstrateHyperAngularConsciousness().catch(console.error);
