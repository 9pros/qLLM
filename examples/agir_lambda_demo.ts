/**
 * AGIR-Lambda Demonstration
 * 
 * This example shows how to use the AGIR-Lambda cognitive language
 * to orchestrate BQIP substrate operations for dual-state AGI cognition.
 */

import {
  defineSubstrate,
  initTwinPool,
  addPhaseLoop,
  superpose,
  collapse,
  decode,
  encode,
  phaseLock,
  entangle,
  createLearningContract,
  generateCurriculumPlan,
  executeTool,
  logOrlEvent,
  updatePolicy,
} from '../packages/agir-lambda/src/index';
import type { PhaseEnvelope, SubstrateConfig, TwinGenome } from '../packages/agir-lambda/src/index';

async function demonstrateAgirLambda() {
  console.log('='.repeat(80));
  console.log('AGIR-Lambda Dual-State Cognition Demonstration');
  console.log('='.repeat(80));

  // ==========================================================================
  // Step 1: Define Substrate Configuration (Environment Sensing)
  // ==========================================================================
  console.log('\n[Step 1] Configuring substrate with environment sensing...');
  
  const substrateConfig: SubstrateConfig = {
    environment_id: 'env-demo-001',
    hardware: {
      cpu_cores: 8,
      gpu_available: true,
      gpu_memory_gb: 16,
      system_memory_gb: 64,
      network_bandwidth_mbps: 1000,
    },
    twin_pool_budget: 32,
    phase_budget: 0.75,
    latency_budget_ms: 50,
    enable_gpu: true,
    enable_network_routing: false,
  };

  const substrateResult = await defineSubstrate(substrateConfig);
  console.log('✓ Substrate configured:', substrateResult);

  // ==========================================================================
  // Step 2: Initialize Twin Pool for Skill (TEPE Genotype Population)
  // ==========================================================================
  console.log('\n[Step 2] Initializing TEPE twin pool for skill "reasoning"...');
  
  const baseEnvelope: PhaseEnvelope = {
    coherence_sig: BigInt(Date.now()),
    alpha: 0.707,
    beta: 0.707,
    resuperposition_n: 0,
    hf_rotation: Math.PI / 4,  // 45° high-frequency rotation
    ha_rotation: Math.PI / 6,  // 30° hyperangular rotation
    phase_offset: 0.05,         // Liquid time-constant offset
  };

  const twinPool = await initTwinPool('reasoning', {
    poolSize: 16,
    baseEnvelope,
    haVectorRange: [0, Math.PI * 2],
  });

  console.log(`✓ Twin pool created: ${twinPool.poolId}`);
  console.log(`  - Genomes: ${twinPool.genomes.length}`);
  console.log(`  - Base envelope: α=${baseEnvelope.alpha}, β=${baseEnvelope.beta}`);
  console.log(`  - HF rotation: ${baseEnvelope.hf_rotation.toFixed(3)} rad`);
  console.log(`  - HA rotation: ${baseEnvelope.ha_rotation.toFixed(3)} rad`);

  // ==========================================================================
  // Step 3: Add Phase Loop for Continuous Evolution
  // ==========================================================================
  console.log('\n[Step 3] Adding coherence phase loop...');
  
  const phaseLoop = await addPhaseLoop('reasoning', {
    loopType: 'coherence',
    targetCoherence: 0.9,
    updateIntervalMs: 1000,
  });

  console.log(`✓ Phase loop created: ${phaseLoop.loopId}`);

  // ==========================================================================
  // Step 4: Create Superposition of Genomes
  // ==========================================================================
  console.log('\n[Step 4] Creating superposition of top genomes...');
  
  const selectedGenomes = twinPool.genomes.slice(0, 4);
  const superposition = superpose(selectedGenomes, {
    phaseLocked: false,
    entanglementPairs: [[0, 1], [2, 3]],
  });

  console.log(`✓ Superposition created with ${superposition.genomes.length} genomes`);
  console.log(`  - Entanglement pairs: ${superposition.entanglement_pairs?.length ?? 0}`);

  // ==========================================================================
  // Step 5: Decode Genome to Phenotype (U_decode = R_HF × R_HA × O)
  // ==========================================================================
  console.log('\n[Step 5] Decoding Ctwin genome to Clive phenotype...');
  
  const sampleGenome = twinPool.genomes[0];
  const liveRegister = 'a'.repeat(64); // Simulated 32-byte register as hex
  
  const phenotype = await decode(sampleGenome, liveRegister);
  console.log('✓ Phenotype decoded:');
  console.log(`  - Register ID: ${phenotype.register_id.bytes.substring(0, 16)}...`);
  console.log(`  - Routing lane: ${phenotype.routing_lane}`);
  console.log(`  - Live state: ${phenotype.state.live.bytes.substring(0, 16)}...`);
  console.log(`  - Twin state: ${phenotype.state.twin.bytes.substring(0, 16)}...`);

  // ==========================================================================
  // Step 6: Phase Lock Genomes for Stable Evolution
  // ==========================================================================
  console.log('\n[Step 6] Phase locking genomes for stability...');
  
  const lockResult = await phaseLock(
    twinPool.genomes[0],
    twinPool.genomes.slice(1, 3),
    0.95
  );
  console.log(`✓ Phase lock applied: strength=${lockResult.lockStrength}`);

  // ==========================================================================
  // Step 7: Entangle Two Genomes for Correlated Evolution
  // ==========================================================================
  console.log('\n[Step 7] Entangling genomes for correlated evolution...');
  
  const entanglement = await entangle(twinPool.genomes[0], twinPool.genomes[1]);
  console.log(`✓ Entanglement created: ${entanglement.entanglementId}`);
  console.log(`  - Correlation: ${entanglement.correlation.toFixed(3)}`);

  // ==========================================================================
  // Step 8: Create Learning Contract (Natural Language → JSON)
  // ==========================================================================
  console.log('\n[Step 8] Creating learning contract from objective...');
  
  const learningContract = createLearningContract(
    'reasoning',
    'Improve logical reasoning accuracy on mathematical problems',
    [
      'Achieve >90% accuracy on test set',
      'Maintain phase coherence >0.85',
      'Reduce inference latency <20ms',
    ],
    {
      constraints: ['No GPU memory overflow', 'Preserve prior knowledge'],
      phaseEnvelopeTarget: baseEnvelope,
      expiresAt: Date.now() + 3600000, // 1 hour
    }
  );

  console.log('✓ Learning contract created:');
  console.log(`  - Contract ID: ${learningContract.contract_id}`);
  console.log(`  - Objective: ${learningContract.objective}`);
  console.log(`  - Success criteria: ${learningContract.success_criteria.length}`);

  // ==========================================================================
  // Step 9: Generate Curriculum Plan (DSVM/CESC Planning)
  // ==========================================================================
  console.log('\n[Step 9] Generating curriculum plan from contract...');
  
  const curriculumPlan = await generateCurriculumPlan(learningContract, {
    twinPoolSize: 16,
    collapseThreshold: 0.85,
    laneAllocation: { 'GenericEndpoint': 0.6, 'PublicApi': 0.4 },
  });

  console.log('✓ Curriculum plan generated:');
  console.log(`  - Plan ID: ${curriculumPlan.plan_id}`);
  console.log(`  - Steps: ${curriculumPlan.steps.length}`);
  curriculumPlan.steps.forEach((step, i) => {
    console.log(`    ${i + 1}. ${step.action} (${step.rollback_on_failure ? 'rollback' : 'no rollback'})`);
  });
  console.log(`  - Twin pool size: ${curriculumPlan.twin_pool_size}`);
  console.log(`  - Collapse threshold: ${curriculumPlan.collapse_threshold}`);

  // ==========================================================================
  // Step 10: Execute Tool Call (BQIP Inference)
  // ==========================================================================
  console.log('\n[Step 10] Executing BQIP inference tool...');
  
  const toolCall = {
    tool_id: 'bqip-infer',
    method: 'run_inference',
    arguments: {
      model_path: '/workspace/bqip_model.bin',
      input: 'What is 2 + 2?',
      twin_genome_id: sampleGenome.id.toString(),
    },
    timeout_ms: 5000,
  };

  const executionResult = await executeTool(toolCall);
  console.log('✓ Tool execution completed:');
  console.log(`  - Success: ${executionResult.success}`);
  console.log(`  - Duration: ${executionResult.duration_ms}ms`);
  if (executionResult.metrics) {
    console.log(`  - Latency: ${executionResult.metrics.latency}ms`);
  }

  // ==========================================================================
  // Step 11: Log ORL Event (Replay Verification)
  // ==========================================================================
  console.log('\n[Step 11] Logging ORL event for verification...');
  
  const orlEvent = await logOrlEvent(
    'fitness_evaluation',
    {
      genome_id: sampleGenome.id.toString(),
      fitness_score: 0.92,
      metrics: { coherence: 0.89, novelty: 0.76, simplicity: 0.84 },
    },
    '0'.repeat(64) // Previous hash (genesis)
  );

  console.log('✓ ORL event logged:');
  console.log(`  - Event ID: ${orlEvent.event_id}`);
  console.log(`  - Type: ${orlEvent.event_type}`);
  console.log(`  - Hash: ${orlEvent.current_hash.substring(0, 16)}...`);

  // ==========================================================================
  // Step 12: Update Self-Mod Policy (Meta-Learning)
  // ==========================================================================
  console.log('\n[Step 12] Updating self-mod policy based on performance...');
  
  const policyUpdate = {
    policy_id: 'policy-tool-selection-001',
    update_type: 'tool_selection' as const,
    old_value: { default_tool: 'bqip-infer-v1' },
    new_value: { default_tool: 'bqip-infer-v2' },
    rationale: 'v2 shows 15% better coherence preservation',
    confidence: 0.87,
    timestamp: Date.now(),
  };

  const policyResult = await updatePolicy(policyUpdate);
  console.log(`✓ Policy updated: ${policyResult.success}`);

  // ==========================================================================
  // Summary
  // ==========================================================================
  console.log('\n' + '='.repeat(80));
  console.log('Demonstration Complete!');
  console.log('='.repeat(80));
  console.log('\nKey achievements:');
  console.log('  ✓ Substrate configured with environment sensing');
  console.log('  ✓ TEPE twin pool initialized (Ctwin genotypes)');
  console.log('  ✓ Phase loops added for continuous evolution');
  console.log('  ✓ Superposition and entanglement operations performed');
  console.log('  ✓ Ctwin → Clive decoding via U_decode = R_HF × R_HA × O');
  console.log('  ✓ Learning contract created from natural language');
  console.log('  ✓ Curriculum plan generated (DSVM/CESC planning)');
  console.log('  ✓ Tool execution against BQIP substrate');
  console.log('  ✓ ORL event logged for replay verification');
  console.log('  ✓ Self-mod policy updated (meta-learning)');
  console.log('\nThe AGIR-Lambda cognitive layer successfully orchestrated');
  console.log('the BQIP dual-state substrate for AGI cognition!');
  console.log('='.repeat(80));
}

// Run demonstration
demonstrateAgirLambda().catch(console.error);
