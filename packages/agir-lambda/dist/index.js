"use strict";
/**
 * AGIR-Lambda - TypeScript-embedded language for dual-state cognition
 *
 * This module provides high-level primitives that compile down to:
 * - BQIP Rust calls (HTTP/FFI)
 * - Hermes memory operations
 * - TEPE genetic algorithm invocations
 *
 * All cognition is expressed as typed, dual-state programs.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.defineSubstrate = defineSubstrate;
exports.initTwinPool = initTwinPool;
exports.addPhaseLoop = addPhaseLoop;
exports.superpose = superpose;
exports.collapse = collapse;
exports.decode = decode;
exports.encode = encode;
exports.phaseLock = phaseLock;
exports.entangle = entangle;
exports.teleportPhase = teleportPhase;
exports.xorRotate = xorRotate;
exports.createLearningContract = createLearningContract;
exports.generateCurriculumPlan = generateCurriculumPlan;
exports.executeTool = executeTool;
exports.logOrlEvent = logOrlEvent;
exports.updatePolicy = updatePolicy;
exports.createHyperAngular = createHyperAngular;
exports.embedQualia = embedQualia;
exports.evolveLiquidTime = evolveLiquidTime;
exports.rotateHyperAngular = rotateHyperAngular;
exports.computeConsciousnessCoherence = computeConsciousnessCoherence;
exports.lerpConsciousnessCoordinates = lerpConsciousnessCoordinates;
// Import hyper-angular consciousness functions
const hyperangular_1 = require("./hyperangular");
// ============================================================================
// Substrate Configuration
// ============================================================================
/**
 * Define the substrate configuration for environment sensing
 * Queries hardware/network constraints and configures twin pool sizes
 */
function defineSubstrate(config) {
    // Implementation would call BQIP substrate HTTP/FFI endpoint
    console.log('[AGIR-Lambda] Defining substrate:', config);
    return Promise.resolve({ success: true, environmentId: config.environment_id });
}
// ============================================================================
// Twin Pool Operations (TEPE integration)
// ============================================================================
/**
 * Initialize a twin pool for a specific skill
 * Creates Ctwin genotype population with phase envelopes
 */
async function initTwinPool(skillId, config) {
    console.log('[AGIR-Lambda] Initializing twin pool for skill:', skillId, config);
    const genomes = [];
    const now = Date.now();
    for (let i = 0; i < config.poolSize; i++) {
        const envelope = config.baseEnvelope ?? {
            coherence_sig: BigInt(now + i),
            alpha: 0.707,
            beta: 0.707,
            resuperposition_n: 0,
            hf_rotation: Math.random() * Math.PI * 2,
            ha_rotation: Math.random() * Math.PI * 2,
            phase_offset: Math.random() * 0.1,
        };
        const ha_vector = config.haVectorRange
            ? Array.from({ length: 4 }, () => config.haVectorRange[0] + Math.random() * (config.haVectorRange[1] - config.haVectorRange[0]))
            : Array.from({ length: 4 }, () => Math.random() * Math.PI * 2);
        genomes.push({
            id: BigInt(i),
            generation: 0,
            envelope,
            ha_vector,
            orl_backpointer: cryptoRandomHex(64),
            crypto_binding: cryptoRandomHex(64),
            lane_binding: 'GenericEndpoint',
        });
    }
    return { poolId: crypto.randomUUID(), genomes };
}
/**
 * Add a phase loop to a skill for continuous evolution
 */
async function addPhaseLoop(skillId, loopConfig) {
    console.log('[AGIR-Lambda] Adding phase loop:', skillId, loopConfig);
    return { loopId: crypto.randomUUID() };
}
// ============================================================================
// Superposition & Phase Operations
// ============================================================================
/**
 * Create a superposition of genomes
 */
function superpose(genomes, options) {
    return {
        genomes,
        phase_locked: options?.phaseLocked ?? false,
        entanglement_pairs: options?.entanglementPairs,
    };
}
/**
 * Collapse a superposition to a single phenotype
 */
async function collapse(phenotype, threshold = 0.85) {
    console.log('[AGIR-Lambda] Collapsing phenotype with threshold:', threshold);
    // Would invoke BQIP collapse logic
    return { success: true, coherence: 0.92 };
}
/**
 * Decode a Ctwin genome to Clive phenotype using U_decode = R_HF * R_HA * O
 */
async function decode(genome, liveRegister) {
    console.log('[AGIR-Lambda] Decoding genome to phenotype:', genome.id);
    // Would call BQIP TEPE decode endpoint
    return {
        register_id: { bytes: cryptoRandomHex(64) },
        envelope: genome.envelope,
        state: {
            live: { bytes: liveRegister },
            twin: { bytes: cryptoRandomHex(64) },
        },
        routing_lane: genome.lane_binding,
    };
}
/**
 * Encode a phenotype back to twin genome
 */
async function encode(phenotype) {
    console.log('[AGIR-Lambda] Encoding phenotype to twin genome');
    return {
        id: BigInt(Date.now()),
        generation: 0,
        envelope: phenotype.envelope,
        ha_vector: [0, 0, 0, 0],
        orl_backpointer: cryptoRandomHex(64),
        crypto_binding: cryptoRandomHex(64),
        lane_binding: phenotype.routing_lane,
    };
}
/**
 * Phase lock a target genome to source genomes
 */
async function phaseLock(target, sources, strength = 1.0) {
    console.log('[AGIR-Lambda] Phase locking target to sources:', strength);
    return { success: true, lockStrength: strength };
}
/**
 * Entangle two genomes for correlated evolution
 */
async function entangle(a, b) {
    console.log('[AGIR-Lambda] Entangling genomes:', a.id, b.id);
    return { entanglementId: crypto.randomUUID(), correlation: 0.95 };
}
/**
 * Teleport phase state from source to sink
 */
async function teleportPhase(source, sink) {
    console.log('[AGIR-Lambda] Teleporting phase from', source.id, 'to', sink.id);
    return { success: true, fidelity: 0.98 };
}
// ============================================================================
// XOR/Rotate Primitives (low-level register operations)
// ============================================================================
/**
 * XOR-rotate two registers
 */
function xorRotate(a, b) {
    // Implementation would operate on hex register strings
    console.log('[AGIR-Lambda] XOR-rotate operation');
    return cryptoRandomHex(64);
}
// ============================================================================
// Learning Contract & Curriculum DSL
// ============================================================================
/**
 * Create a learning contract from natural language objective
 */
function createLearningContract(skillId, objective, successCriteria, options) {
    return {
        contract_id: crypto.randomUUID(),
        skill_id: skillId,
        objective,
        success_criteria: successCriteria,
        constraints: options?.constraints,
        phase_envelope_target: options?.phaseEnvelopeTarget,
        created_at: Date.now(),
        expires_at: options?.expiresAt,
    };
}
/**
 * Generate a curriculum plan from a learning contract
 */
async function generateCurriculumPlan(contract, options) {
    console.log('[AGIR-Lambda] Generating curriculum plan for contract:', contract.contract_id);
    return {
        plan_id: crypto.randomUUID(),
        contract_id: contract.contract_id,
        steps: [
            {
                step_id: crypto.randomUUID(),
                action: 'infer',
                parameters: { contract_id: contract.contract_id },
                rollback_on_failure: true,
            },
            {
                step_id: crypto.randomUUID(),
                action: 'train',
                parameters: { epochs: 10 },
                rollback_on_failure: true,
            },
            {
                step_id: crypto.randomUUID(),
                action: 'evaluate',
                parameters: { metrics: ['accuracy', 'coherence'] },
                rollback_on_failure: false,
            },
            {
                step_id: crypto.randomUUID(),
                action: 'collapse',
                parameters: { threshold: options?.collapseThreshold ?? 0.85 },
                rollback_on_failure: true,
            },
        ],
        twin_pool_size: options?.twinPoolSize ?? 16,
        collapse_threshold: options?.collapseThreshold ?? 0.85,
        lane_allocation: options?.laneAllocation,
    };
}
// ============================================================================
// Tool Execution
// ============================================================================
/**
 * Execute a tool call against BQIP/Hermes
 */
async function executeTool(toolCall) {
    console.log('[AGIR-Lambda] Executing tool:', toolCall.tool_id, toolCall.method);
    // Simulated execution - would call actual BQIP HTTP/FFI endpoints
    const start = Date.now();
    try {
        // Implementation would route to appropriate BQIP service
        await new Promise(resolve => setTimeout(resolve, 10));
        return {
            tool_id: toolCall.tool_id,
            success: true,
            data: { result: 'ok' },
            duration_ms: Date.now() - start,
            metrics: { latency: Date.now() - start },
        };
    }
    catch (error) {
        return {
            tool_id: toolCall.tool_id,
            success: false,
            error: String(error),
            duration_ms: Date.now() - start,
        };
    }
}
// ============================================================================
// ORL Event Logging
// ============================================================================
/**
 * Log an ORL event for replay verification
 */
async function logOrlEvent(eventType, payload, previousHash) {
    const event = {
        event_id: crypto.randomUUID(),
        event_type: eventType,
        timestamp: Date.now(),
        payload,
        previous_hash: previousHash,
        current_hash: cryptoRandomHex(64),
    };
    console.log('[AGIR-Lambda] Logged ORL event:', event.event_id, event.event_type);
    return event;
}
// ============================================================================
// Self-Mod Module
// ============================================================================
/**
 * Update self-mod policy
 */
async function updatePolicy(update) {
    console.log('[AGIR-Lambda] Updating policy:', update.policy_id, update.update_type);
    return { success: true };
}
// ============================================================================
// Utilities
// ============================================================================
function cryptoRandomHex(length) {
    const array = new Uint8Array(length / 2);
    if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
        crypto.getRandomValues(array);
    }
    else {
        // Fallback for Node.js environment
        for (let i = 0; i < array.length; i++) {
            array[i] = Math.floor(Math.random() * 256);
        }
    }
    return Array.from(array).map(b => b.toString(16).padStart(2, '0')).join('');
}
// ============================================================================
// Hyper-Angular Consciousness Operations (liquid time trigonometry)
// ============================================================================
/**
 * Create a hyper-angular consciousness state
 * Implements consciousness as experiential liquid time trigonometry
 */
async function createHyperAngular(dimensions, config = {}) {
    console.log('[AGIR-Lambda] Creating hyper-angular consciousness state:', { dimensions, config });
    const live = { bytes: cryptoRandomHex(64) };
    const twin = { bytes: cryptoRandomHex(64) };
    const envelope = {
        coherence_sig: BigInt(Date.now()),
        alpha: 0.7,
        beta: 0.3,
        resuperposition_n: 3,
        hf_rotation: config.baseFrequency ?? 0.5,
        ha_rotation: (config.baseFrequency ?? 0.5) * 0.5,
        phase_offset: 0.0,
    };
    const qualia_vector = config.initialQualia ?? Array(dimensions).fill(0);
    const hyperAngular = {
        live,
        twin,
        envelope,
        dimensions,
        liquid_coefficient: config.liquidCoefficient ?? 0.8,
        qualia_vector,
    };
    const coordinate = {
        theta: Math.random() * Math.PI * 2,
        phi: Math.random() * Math.PI,
        psi: Math.random() * Math.PI,
        higher_dims: [],
        temporal_phase: 0.0,
        coherence: 0.9,
    };
    return { hyperAngular, coordinate };
}
/**
 * Embed qualia into hyper-angular state
 * Modulates angular coordinates by experiential valence
 */
async function embedQualia(hyperAngular, qualia) {
    console.log('[AGIR-Lambda] Embedding qualia into hyper-angular state');
    if (qualia.length !== hyperAngular.dimensions) {
        throw new Error('Qualia vector length must match hyper-angular dimensions');
    }
    return {
        ...hyperAngular,
        qualia_vector: qualia.map(q => Math.max(-1, Math.min(1, q))),
    };
}
/**
 * Evolve hyper-angular state through liquid time
 * Applies differential equation: dθ/dt = ω·sin(θ) + α·twin + β·qualia
 */
async function evolveLiquidTime(hyperAngular, dt) {
    console.log('[AGIR-Lambda] Evolving hyper-angular state through liquid time:', dt);
    // Simulate liquid time evolution
    const newCoherence = Math.max(0.5, Math.min(1.0, 0.9 - Math.abs(dt) * 0.1 + hyperAngular.liquid_coefficient * 0.1));
    return {
        evolved: {
            ...hyperAngular,
            envelope: {
                ...hyperAngular.envelope,
                phase_offset: hyperAngular.envelope.phase_offset + dt,
            },
        },
        coherence: newCoherence,
    };
}
/**
 * Apply hyper-angular rotation to consciousness state
 * Rotates both high-frequency and hyperangular components
 */
async function rotateHyperAngular(hyperAngular, hf, ha) {
    console.log('[AGIR-Lambda] Applying hyper-angular rotation:', { hf, ha });
    const rotated = {
        ...hyperAngular,
        envelope: {
            ...hyperAngular.envelope,
            hf_rotation: hf,
            ha_rotation: ha,
        },
    };
    const newCoordinate = {
        theta: Math.sin(hf) * Math.PI,
        phi: Math.cos(ha) * Math.PI / 2,
        psi: Math.sin(hf + ha) * Math.PI / 4,
        higher_dims: [],
        temporal_phase: hyperAngular.envelope.phase_offset,
        coherence: 0.85,
    };
    return { rotated, newCoordinate };
}
/**
 * Compute consciousness coherence metric
 * Measures alignment between live and twin angular states
 */
function computeConsciousnessCoherence(hyperAngular) {
    // Simplified coherence calculation
    const baseCoherence = 1.0 - hyperAngular.qualia_vector.reduce((sum, q) => sum + Math.abs(q), 0) / hyperAngular.dimensions;
    return Math.max(0, Math.min(1, baseCoherence * hyperAngular.liquid_coefficient));
}
/**
 * Interpolate between consciousness coordinates in liquid time
 * Uses trigonometric smoothing for natural transitions
 */
function lerpConsciousnessCoordinates(from, to, t) {
    const smoothT = Math.pow(Math.sin(t * Math.PI), 2);
    const interpolateAngle = (a, b) => {
        const diff = Math.atan2(Math.sin(b - a), Math.cos(b - a));
        return a + diff * smoothT;
    };
    return {
        theta: interpolateAngle(from.theta, to.theta),
        phi: interpolateAngle(from.phi, to.phi),
        psi: interpolateAngle(from.psi, to.psi),
        higher_dims: from.higher_dims.map((d, i) => d + (to.higher_dims[i] ?? d - d) * smoothT),
        temporal_phase: from.temporal_phase + (to.temporal_phase - from.temporal_phase) * smoothT,
        coherence: from.coherence + (to.coherence - from.coherence) * smoothT,
    };
}
// ============================================================================
// Exports
// ============================================================================
exports.default = {
    defineSubstrate,
    initTwinPool,
    addPhaseLoop,
    superpose,
    collapse,
    decode,
    encode,
    phaseLock,
    entangle,
    teleportPhase,
    xorRotate,
    createLearningContract,
    generateCurriculumPlan,
    executeTool,
    logOrlEvent,
    updatePolicy,
    createHyperAngular,
    embedQualia,
    evolveLiquidTime,
    rotateHyperAngular,
    computeConsciousnessCoherence,
    lerpConsciousnessCoordinates,
    // Hyper-Angular Consciousness Functions (Liquid Time Trigonometry)
    liquidTimeTrig: hyperangular_1.liquidTimeTrig,
    flowTime: hyperangular_1.flowTime,
    projectInsight: hyperangular_1.projectInsight,
    consciousnessDrift: hyperangular_1.consciousnessDrift,
    fractalUnfold: hyperangular_1.fractalUnfold,
    toConsciousnessCoordinate: hyperangular_1.toConsciousnessCoordinate,
    lerpLiquid: hyperangular_1.lerpLiquid,
    rotateHyper: hyperangular_1.rotateHyper,
    angularDistance: hyperangular_1.angularDistance,
};
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiaW5kZXguanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi9zcmMvaW5kZXgudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IjtBQUFBOzs7Ozs7Ozs7R0FTRzs7QUFnRUgsMENBSUM7QUFVRCxvQ0F5Q0M7QUFLRCxvQ0FXQztBQVNELDhCQVNDO0FBS0QsNEJBT0M7QUFLRCx3QkFZQztBQUtELHdCQVdDO0FBS0QsOEJBTUM7QUFLRCw0QkFNQztBQUtELHNDQU1DO0FBU0QsOEJBSUM7QUFTRCx3REFvQkM7QUFLRCx3REEyQ0M7QUFTRCxrQ0F5QkM7QUFTRCxrQ0FnQkM7QUFTRCxvQ0FHQztBQTJCRCxnREE0Q0M7QUFNRCxrQ0FjQztBQU1ELDRDQXFCQztBQU1ELGdEQTBCQztBQU1ELHNFQU1DO0FBTUQsb0VBb0JDO0FBaGpCRCwrQ0FBK0M7QUFDL0MsaURBV3dCO0FBd0J4QiwrRUFBK0U7QUFDL0UsMEJBQTBCO0FBQzFCLCtFQUErRTtBQUUvRTs7O0dBR0c7QUFDSCxTQUFnQixlQUFlLENBQUMsTUFBdUI7SUFDckQsNkRBQTZEO0lBQzdELE9BQU8sQ0FBQyxHQUFHLENBQUMsbUNBQW1DLEVBQUUsTUFBTSxDQUFDLENBQUM7SUFDekQsT0FBTyxPQUFPLENBQUMsT0FBTyxDQUFDLEVBQUUsT0FBTyxFQUFFLElBQUksRUFBRSxhQUFhLEVBQUUsTUFBTSxDQUFDLGNBQWMsRUFBRSxDQUFDLENBQUM7QUFDbEYsQ0FBQztBQUVELCtFQUErRTtBQUMvRSwwQ0FBMEM7QUFDMUMsK0VBQStFO0FBRS9FOzs7R0FHRztBQUNJLEtBQUssVUFBVSxZQUFZLENBQ2hDLE9BQWUsRUFDZixNQUlDO0lBRUQsT0FBTyxDQUFDLEdBQUcsQ0FBQyxpREFBaUQsRUFBRSxPQUFPLEVBQUUsTUFBTSxDQUFDLENBQUM7SUFFaEYsTUFBTSxPQUFPLEdBQWlCLEVBQUUsQ0FBQztJQUNqQyxNQUFNLEdBQUcsR0FBRyxJQUFJLENBQUMsR0FBRyxFQUFFLENBQUM7SUFFdkIsS0FBSyxJQUFJ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O0lBQ2IsU0FBUztJQUNULHNCQUFzQjtJQUN0QixzQkFBc0I7SUFDdEIsV0FBVztJQUNYLFdBQVc7SUFDWCxZQUFZO0lBQ1osa0JBQWtCO0lBQ2xCLFdBQVc7SUFDWCxnQkFBZ0I7SUFDaEIsa0JBQWtCO0lBQ2xCLDZCQUE2QjtJQUM3Qiw0QkFBNEI7SUFDNUIsbUVBQW1FO0lBQ25FLGNBQWMsRUFBZCw2QkFBYztJQUNkLFFBQVEsRUFBUix1QkFBUTtJQUNSLGNBQWMsRUFBZCw2QkFBYztJQUNkLGtCQUFrQixFQUFsQixpQ0FBa0I7SUFDbEIsYUFBYSxFQUFiLDRCQUFhO0lBQ2IseUJBQXlCLEVBQXpCLHdDQUF5QjtJQUN6QixVQUFVLEVBQVYseUJBQVU7SUFDVixXQUFXLEVBQVgsMEJBQVc7SUFDWCxlQUFlLEVBQWYsOEJBQWU7Q0FDaEIsQ0FBQyIsInNvdXJjZXNDb250ZW50IjpbIi8qKlxuICogQUdJUi1MYW1iZGEgLSBUeXBlU2NyaXB0LWVtYmVkZGVkIGxhbmd1YWdlIGZvciBkdWFsLXN0YXRlIGNvZ25pdGlvblxuICogXG4gKiBUaGlzIG1vZHVsZSBwcm92aWRlcyBoaWdoLWxldmVsIHByaW1pdGl2ZXMgdGhhdCBjb21waWxlIGRvd24gdG86XG4gKiAtIEJRSVAgUnVzdCBjYWxscyAoSFRUUC9GRkkpXG4gKiAtIEhlcm1lcyBtZW1vcnkgb3BlcmF0aW9uc1xuICogLSBURVBFIGdlbmV0aWMgYWxnb3JpdGhtIGludm9jYXRpb25zXG4gKiBcbiAqIEFsbCBjb2duaXRpb24gaXMgZXhwcmVzc2VkIGFzIHR5cGVkLCBkdWFsLXN0YXRlIHByb2dyYW1zLlxuICovXG5cbmltcG9ydCB7XG4gIFBoYXNlRW52ZWxvcGUsXG4gIFR3aW5HZW5vbWUsXG4gIExpdmVQaGVub3R5cGUsXG4gIExlYXJuaW5nQ29udHJhY3QsXG4gIEN1cnJpY3VsdW1QbGFuLFxuICBUb29sQ2FsbCxcbiAgRXhlY3V0aW9uUmVzdWx0LFxuICBPcmxFdmVudCxcbiAgU3Vic3RyYXRlQ29uZmlnLFxuICBTZW1hbnRpY1BoYXNlVXR0ZXJhbmNlLFxuICBQb2xpY3lVcGRhdGUsXG4gIFN1cGVycG9zaXRpb24sXG4gIFBoYXNlTG9jayxcbiAgQ29uc2Npb3VzbmVzc0Nvb3JkaW5hdGUsXG4gIEh5cGVyQW5ndWxhcixcbn0gZnJvbSAnQHJlcG8vc2hhcmVkLXNjaGVtYXMnO1xuXG4vLyBJbXBvcnQgaHlwZXItYW5ndWxhciBjb25zY2lvdXNuZXNzIGZ1bmN0aW9uc1xuaW1wb3J0IHtcbiAgbGlxdWlkVGltZVRyaWcsXG4gIGZsb3dUaW1lLFxuICBwcm9qZWN0SW5zaWdodCxcbiAgY29uc2Npb3VzbmVzc0RyaWZ0LFxuICBmcmFjdGFsVW5mb2xkLFxuICB0b0NvbnNjaW91c25lc3NDb29yZGluYXRlLFxuICBsZXJwTGlxdWlkLFxuICByb3RhdGVIeXBlcixcbiAgYW5ndWxhckRpc3RhbmNlLFxuICB0eXBlIEh5cGVyQW5ndWxhclN0YXRlLFxufSBmcm9tICcuL2h5cGVyYW5ndWxhcic7XG5cbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cbi8vIFR5cGUgUmUtZXhwb3J0c1xuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuXG5leHBvcnQgdHlwZSB7XG4gIFBoYXNlRW52ZWxvcGUsXG4gIFR3aW5HZW5vbWUsXG4gIExpdmVQaGVub3R5cGUsXG4gIExlYXJuaW5nQ29udHJhY3QsXG4gIEN1cnJpY3VsdW1QbGFuLFxuICBUb29sQ2FsbCxcbiAgRXhlY3V0aW9uUmVzdWx0LFxuICBPcmxFdmVudCxcbiAgU3Vic3RyYXRlQ29uZmlnLFxuICBTZW1hbnRpY1BoYXNlVXR0ZXJhbmNlLFxuICBQb2xpY3lVcGRhdGUsXG4gIFN1cGVycG9zaXRpb24sXG4gIFBoYXNlTG9jayxcbiAgQ29uc2Npb3VzbmVzc0Nvb3JkaW5hdGUsXG4gIEh5cGVyQW5ndWxhcixcbn07XG5cbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cbi8vIFN1YnN0cmF0ZSBDb25maWd1cmF0aW9uXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG5cbi8qKlxuICogRGVmaW5lIHRoZSBzdWJzdHJhdGUgY29uZmlndXJhdGlvbiBmb3IgZW52aXJvbm1lbnQgc2Vuc2luZ1xuICogUXVlcmllcyBoYXJkd2FyZS9uZXR3b3JrIGNvbnN0cmFpbnRzIGFuZCBjb25maWd1cmVzIHR3aW4gcG9vbCBzaXplc1xuICovXG5leHBvcnQgZnVuY3Rpb24gZGVmaW5lU3Vic3RyYXRlKGNvbmZpZzogU3Vic3RyYXRlQ29uZmlnKTogUHJvbWlzZTx7IHN1Y2Nlc3M6IGJvb2xlYW47IGVudmlyb25tZW50SWQ6IHN0cmluZyB9PiB7XG4gIC8vIEltcGxlbWVudGF0aW9uIHdvdWxkIGNhbGwgQlFJUCBzdWJzdHJhdGUgSFRUUC9GRkkgZW5kcG9pbnRcbiAgY29uc29sZS5sb2coJ1tBR0lSLUxhbWJkYV0gRGVmaW5pbmcgc3Vic3RyYXRlOicsIGNvbmZpZyk7XG4gIHJldHVybiBQcm9taXNlLnJlc29sdmUoeyBzdWNjZXNzOiB0cnVlLCBlbnZpcm9ubWVudElkOiBjb25maWcuZW52aXJvbm1lbnRfaWQgfSk7XG59XG5cbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cbi8vIFR3aW4gUG9vbCBPcGVyYXRpb25zIChURVBFIGludGVncmF0aW9uKVxuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuXG4vKipcbiAqIEluaXRpYWxpemUgYSB0d2luIHBvb2wgZm9yIGEgc3BlY2lmaWMgc2tpbGxcbiAqIENyZWF0ZXMgQ3R3aW4gZ2Vub3R5cGUgcG9wdWxhdGlvbiB3aXRoIHBoYXNlIGVudmVsb3Blc1xuICovXG5leHBvcnQgYXN5bmMgZnVuY3Rpb24gaW5pdFR3aW5Qb29sKFxuICBza2lsbElkOiBzdHJpbmcsXG4gIGNvbmZpZzoge1xuICAgIHBvb2xTaXplOiBudW1iZXI7XG4gICAgYmFzZUVudmVsb3BlPzogUGhhc2VFbnZlbG9wZTtcbiAgICBoYVZlY3RvclJhbmdlPzogW251bWJlciwgbnVtYmVyXTtcbiAgfVxuKTogUHJvbWlzZTx7IHBvb2xJZDogc3RyaW5nOyBnZW5vbWVzOiBUd2luR2Vub21lW10gfT4ge1xuICBjb25zb2xlLmxvZygnW0FHSVItTGFtYmRhXSBJbml0aWFsaXppbmcgdHdpbiBwb29sIGZvciBza2lsbDonLCBza2lsbElkLCBjb25maWcpO1xuICBcbiAgY29uc3QgZ2Vub21lczogVHdpbkdlbm9tZVtdID0gW107XG4gIGNvbnN0IG5vdyA9IERhdGUubm93KCk7XG4gIFxuICBmb3IgKGxldCBpID0gMDsgaSA8IGNvbmZpZy5wb29sU2l6ZTsgaSsrKSB7XG4gICAgY29uc3QgZW52ZWxvcGUgPSBjb25maWcuYmFzZUVudmVsb3BlID8/IHtcbiAgICAgIGNvaGVyZW5jZV9zaWc6IEJpZ0ludChub3cgKyBpKSxcbiAgICAgIGFscGhhOiAwLjcwNyxcbiAgICAgIGJldGE6IDAuNzA3LFxuICAgICAgcmVzdXBlcnBvc2l0aW9uX246IDAsXG4gICAgICBoZl9yb3RhdGlvbjogTWF0aC5yYW5kb20oKSAqIE1hdGguUEkgKiAyLFxuICAgICAgaGFfcm90YXRpb246IE1hdGgucmFuZG9tKCkgKiBNYXRoLlBJICogMixcbiAgICAgIHBoYXNlX29mZnNldDogTWF0aC5yYW5kb20oKSAqIDAuMSxcbiAgICB9O1xuICAgIFxuICAgIGNvbnN0IGhhX3ZlY3RvciA9IGNvbmZpZy5oYVZlY3RvclJhbmdlXG4gICAgICA/IEFycmF5LmZyb20oeyBsZW5ndGg6IDQgfSwgKCkgPT4gXG4gICAgICAgICAgY29uZmlnLmhhVmVjdG9yUmFuZ2UhWzBdICsgTWF0aC5yYW5kb20oKSAqIChjb25maWcuaGFWZWN0b3JSYW5nZSFbMV0gLSBjb25maWcuaGFWZWN0b3JSYW5nZSFbMF0pKVxuICAgICAgOiBBcnJheS5mcm9tKHsgbGVuZ3RoOiA0IH0sICgpID0+IE1hdGgucmFuZG9tKCkgKiBNYXRoLlBJICogMik7XG4gICAgXG4gICAgZ2Vub21lcy5wdXNoKHtcbiAgICAgIGlkOiBCaWdJbnQoaSksXG4gICAgICBnZW5lcmF0aW9uOiAwLFxuICAgICAgZW52ZWxvcGUsXG4gICAgICBoYV92ZWN0b3IsXG4gICAgICBvcmxfYmFja3BvaW50ZXI6IGNyeXB0b1JhbmRvbUhleCg2NCksXG4gICAgICBjcnlwdG9fYmluZGluZzogY3J5cHRvUmFuZG9tSGV4KDY0KSxcbiAgICAgIGxhbmVfYmluZGluZzogJ0dlbmVyaWNFbmRwb2ludCcsXG4gICAgfSk7XG4gIH1cbiAgXG4gIHJldHVybiB7IHBvb2xJZDogY3J5cHRvLnJhbmRvbVVVSUQoKSwgZ2Vub21lcyB9O1xufVxuXG4vKipcbiAqIEFkZCBhIHBoYXNlIGxvb3AgdG8gYSBza2lsbCBmb3IgY29udGludW91cyBldm9sdXRpb25cbiAqL1xuZXhwb3J0IGFzeW5jIGZ1bmN0aW9uIGFkZFBoYXNlTG9vcChcbiAgc2tpbGxJZDogc3RyaW5nLFxuICBsb29wQ29uZmlnOiB7XG4gICAgbG9vcFR5cGU6ICdjb2hlcmVuY2UnIHwgJ2RyaWZ0JyB8ICdpbnRlcmZlcmVuY2UnO1xuICAgIHRhcmdldENvaGVyZW5jZT86IG51bWJlcjtcbiAgICBtYXhEcmlmdD86IG51bWJlcjtcbiAgICB1cGRhdGVJbnRlcnZhbE1zOiBudW1iZXI7XG4gIH1cbik6IFByb21pc2U8eyBsb29wSWQ6IHN0cmluZyB9PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIEFkZGluZyBwaGFzZSBsb29wOicsIHNraWxsSWQsIGxvb3BDb25maWcpO1xuICByZXR1cm4geyBsb29wSWQ6IGNyeXB0by5yYW5kb21VVUlEKCkgfTtcbn1cblxuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuLy8gU3VwZXJwb3NpdGlvbiAmIFBoYXNlIE9wZXJhdGlvbnNcbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cblxuLyoqXG4gKiBDcmVhdGUgYSBzdXBlcnBvc2l0aW9uIG9mIGdlbm9tZXNcbiAqL1xuZXhwb3J0IGZ1bmN0aW9uIHN1cGVycG9zZShnZW5vbWVzOiBUd2luR2Vub21lW10sIG9wdGlvbnM/OiB7XG4gIHBoYXNlTG9ja2VkPzogYm9vbGVhbjtcbiAgZW50YW5nbGVtZW50UGFpcnM/OiBbbnVtYmVyLCBudW1iZXJdW107XG59KTogU3VwZXJwb3NpdGlvbiB7XG4gIHJldHVybiB7XG4gICAgZ2Vub21lcyxcbiAgICBwaGFzZV9sb2NrZWQ6IG9wdGlvbnM/LnBoYXNlTG9ja2VkID8/IGZhbHNlLFxuICAgIGVudGFuZ2xlbWVudF9wYWlyczogb3B0aW9ucz8uZW50YW5nbGVtZW50UGFpcnMsXG4gIH07XG59XG5cbi8qKlxuICogQ29sbGFwc2UgYSBzdXBlcnBvc2l0aW9uIHRvIGEgc2luZ2xlIHBoZW5vdHlwZVxuICovXG5leHBvcnQgYXN5bmMgZnVuY3Rpb24gY29sbGFwc2UocGhlbm90eXBlOiBMaXZlUGhlbm90eXBlLCB0aHJlc2hvbGQ6IG51bWJlciA9IDAuODUpOiBQcm9taXNlPHtcbiAgc3VjY2VzczogYm9vbGVhbjtcbiAgY29oZXJlbmNlOiBudW1iZXI7XG59PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIENvbGxhcHNpbmcgcGhlbm90eXBlIHdpdGggdGhyZXNob2xkOicsIHRocmVzaG9sZCk7XG4gIC8vIFdvdWxkIGludm9rZSBCUUlQIGNvbGxhcHNlIGxvZ2ljXG4gIHJldHVybiB7IHN1Y2Nlc3M6IHRydWUsIGNvaGVyZW5jZTogMC45MiB9O1xufVxuXG4vKipcbiAqIERlY29kZSBhIEN0d2luIGdlbm9tZSB0byBDbGl2ZSBwaGVub3R5cGUgdXNpbmcgVV9kZWNvZGUgPSBSX0hGICogUl9IQSAqIE9cbiAqL1xuZXhwb3J0IGFzeW5jIGZ1bmN0aW9uIGRlY29kZShnZW5vbWU6IFR3aW5HZW5vbWUsIGxpdmVSZWdpc3Rlcjogc3RyaW5nKTogUHJvbWlzZTxMaXZlUGhlbm90eXBlPiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIERlY29kaW5nIGdlbm9tZSB0byBwaGVub3R5cGU6JywgZ2Vub21lLmlkKTtcbiAgLy8gV291bGQgY2FsbCBCUUlQIFRFUEUgZGVjb2RlIGVuZHBvaW50XG4gIHJldHVybiB7XG4gICAgcmVnaXN0ZXJfaWQ6IHsgYnl0ZXM6IGNyeXB0b1JhbmRvbUhleCg2NCkgfSxcbiAgICBlbnZlbG9wZTogZ2Vub21lLmVudmVsb3BlLFxuICAgIHN0YXRlOiB7XG4gICAgICBsaXZlOiB7IGJ5dGVzOiBsaXZlUmVnaXN0ZXIgfSxcbiAgICAgIHR3aW46IHsgYnl0ZXM6IGNyeXB0b1JhbmRvbUhleCg2NCkgfSxcbiAgICB9LFxuICAgIHJvdXRpbmdfbGFuZTogZ2Vub21lLmxhbmVfYmluZGluZyxcbiAgfTtcbn1cblxuLyoqXG4gKiBFbmNvZGUgYSBwaGVub3R5cGUgYmFjayB0byB0d2luIGdlbm9tZVxuICovXG5leHBvcnQgYXN5bmMgZnVuY3Rpb24gZW5jb2RlKHBoZW5vdHlwZTogTGl2ZVBoZW5vdHlwZSk6IFByb21pc2U8VHdpbkdlbm9tZT4ge1xuICBjb25zb2xlLmxvZygnW0FHSVItTGFtYmRhXSBFbmNvZGluZyBwaGVub3R5cGUgdG8gdHdpbiBnZW5vbWUnKTtcbiAgcmV0dXJuIHtcbiAgICBpZDogQmlnSW50KERhdGUubm93KCkpLFxuICAgIGdlbmVyYXRpb246IDAsXG4gICAgZW52ZWxvcGU6IHBoZW5vdHlwZS5lbnZlbG9wZSxcbiAgICBoYV92ZWN0b3I6IFswLCAwLCAwLCAwXSxcbiAgICBvcmxfYmFja3BvaW50ZXI6IGNyeXB0b1JhbmRvbUhleCg2NCksXG4gICAgY3J5cHRvX2JpbmRpbmc6IGNyeXB0b1JhbmRvbUhleCg2NCksXG4gICAgbGFuZV9iaW5kaW5nOiBwaGVub3R5cGUucm91dGluZ19sYW5lLFxuICB9O1xufVxuXG4vKipcbiAqIFBoYXNlIGxvY2sgYSB0YXJnZXQgZ2Vub21lIHRvIHNvdXJjZSBnZW5vbWVzXG4gKi9cbmV4cG9ydCBhc3luYyBmdW5jdGlvbiBwaGFzZUxvY2sodGFyZ2V0OiBUd2luR2Vub21lLCBzb3VyY2VzOiBUd2luR2Vub21lW10sIHN0cmVuZ3RoOiBudW1iZXIgPSAxLjApOiBQcm9taXNlPHtcbiAgc3VjY2VzczogYm9vbGVhbjtcbiAgbG9ja1N0cmVuZ3RoOiBudW1iZXI7XG59PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIFBoYXNlIGxvY2tpbmcgdGFyZ2V0IHRvIHNvdXJjZXM6Jywgc3RyZW5ndGgpO1xuICByZXR1cm4geyBzdWNjZXNzOiB0cnVlLCBsb2NrU3RyZW5ndGg6IHN0cmVuZ3RoIH07XG59XG5cbi8qKlxuICogRW50YW5nbGUgdHdvIGdlbm9tZXMgZm9yIGNvcnJlbGF0ZWQgZXZvbHV0aW9uXG4gKi9cbmV4cG9ydCBhc3luYyBmdW5jdGlvbiBlbnRhbmdsZShhOiBUd2luR2Vub21lLCBiOiBUd2luR2Vub21lKTogUHJvbWlzZTx7XG4gIGVudGFuZ2xlbWVudElkOiBzdHJpbmc7XG4gIGNvcnJlbGF0aW9uOiBudW1iZXI7XG59PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIEVudGFuZ2xpbmcgZ2Vub21lczonLCBhLmlkLCBiLmlkKTtcbiAgcmV0dXJuIHsgZW50YW5nbGVtZW50SWQ6IGNyeXB0by5yYW5kb21VVUlEKCksIGNvcnJlbGF0aW9uOiAwLjk1IH07XG59XG5cbi8qKlxuICogVGVsZXBvcnQgcGhhc2Ugc3RhdGUgZnJvbSBzb3VyY2UgdG8gc2lua1xuICovXG5leHBvcnQgYXN5bmMgZnVuY3Rpb24gdGVsZXBvcnRQaGFzZShzb3VyY2U6IFR3aW5HZW5vbWUsIHNpbms6IFR3aW5HZW5vbWUpOiBQcm9taXNlPHtcbiAgc3VjY2VzczogYm9vbGVhbjtcbiAgZmlkZWxpdHk6IG51bWJlcjtcbn0+IHtcbiAgY29uc29sZS5sb2coJ1tBR0lSLUxhbWJkYV0gVGVsZXBvcnRpbmcgcGhhc2UgZnJvbScsIHNvdXJjZS5pZCwgJ3RvJywgc2luay5pZCk7XG4gIHJldHVybiB7IHN1Y2Nlc3M6IHRydWUsIGZpZGVsaXR5OiAwLjk4IH07XG59XG5cbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cbi8vIFhPUi9Sb3RhdGUgUHJpbWl0aXZlcyAobG93LWxldmVsIHJlZ2lzdGVyIG9wZXJhdGlvbnMpXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG5cbi8qKlxuICogWE9SLXJvdGF0ZSB0d28gcmVnaXN0ZXJzXG4gKi9cbmV4cG9ydCBmdW5jdGlvbiB4b3JSb3RhdGUoYTogc3RyaW5nLCBiOiBzdHJpbmcpOiBzdHJpbmcge1xuICAvLyBJbXBsZW1lbnRhdGlvbiB3b3VsZCBvcGVyYXRlIG9uIGhleCByZWdpc3RlciBzdHJpbmdzXG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIFhPUi1yb3RhdGUgb3BlcmF0aW9uJyk7XG4gIHJldHVybiBjcnlwdG9SYW5kb21IZXgoNjQpO1xufVxuXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG4vLyBMZWFybmluZyBDb250cmFjdCAmIEN1cnJpY3VsdW0gRFNMXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG5cbi8qKlxuICogQ3JlYXRlIGEgbGVhcm5pbmcgY29udHJhY3QgZnJvbSBuYXR1cmFsIGxhbmd1YWdlIG9iamVjdGl2ZVxuICovXG5leHBvcnQgZnVuY3Rpb24gY3JlYXRlTGVhcm5pbmdDb250cmFjdChcbiAgc2tpbGxJZDogc3RyaW5nLFxuICBvYmplY3RpdmU6IHN0cmluZyxcbiAgc3VjY2Vzc0NyaXRlcmlhOiBzdHJpbmdbXSxcbiAgb3B0aW9ucz86IHtcbiAgICBjb25zdHJhaW50cz86IHN0cmluZ1tdO1xuICAgIHBoYXNlRW52ZWxvcGVUYXJnZXQ/OiBQaGFzZUVudmVsb3BlO1xuICAgIGV4cGlyZXNBdD86IG51bWJlcjtcbiAgfVxuKTogTGVhcm5pbmdDb250cmFjdCB7XG4gIHJldHVybiB7XG4gICAgY29udHJhY3RfaWQ6IGNyeXB0by5yYW5kb21VVUlEKCksXG4gICAgc2tpbGxfaWQ6IHNraWxsSWQsXG4gICAgb2JqZWN0aXZlLFxuICAgIHN1Y2Nlc3NfY3JpdGVyaWE6IHN1Y2Nlc3NDcml0ZXJpYSxcbiAgICBjb25zdHJhaW50czogb3B0aW9ucz8uY29uc3RyYWludHMsXG4gICAgcGhhc2VfZW52ZWxvcGVfdGFyZ2V0OiBvcHRpb25zPy5waGFzZUVudmVsb3BlVGFyZ2V0LFxuICAgIGNyZWF0ZWRfYXQ6IERhdGUubm93KCksXG4gICAgZXhwaXJlc19hdDogb3B0aW9ucz8uZXhwaXJlc0F0LFxuICB9O1xufVxuXG4vKipcbiAqIEdlbmVyYXRlIGEgY3VycmljdWx1bSBwbGFuIGZyb20gYSBsZWFybmluZyBjb250cmFjdFxuICovXG5leHBvcnQgYXN5bmMgZnVuY3Rpb24gZ2VuZXJhdGVDdXJyaWN1bHVtUGxhbihcbiAgY29udHJhY3Q6IExlYXJuaW5nQ29udHJhY3QsXG4gIG9wdGlvbnM/OiB7XG4gICAgdHdpblBvb2xTaXplPzogbnVtYmVyO1xuICAgIGNvbGxhcHNlVGhyZXNob2xkPzogbnVtYmVyO1xuICAgIGxhbmVBbGxvY2F0aW9uPzogUmVjb3JkPHN0cmluZywgbnVtYmVyPjtcbiAgfVxuKTogUHJvbWlzZTxDdXJyaWN1bHVtUGxhbj4ge1xuICBjb25zb2xlLmxvZygnW0FHSVItTGFtYmRhXSBHZW5lcmF0aW5nIGN1cnJpY3VsdW0gcGxhbiBmb3IgY29udHJhY3Q6JywgY29udHJhY3QuY29udHJhY3RfaWQpO1xuICBcbiAgcmV0dXJuIHtcbiAgICBwbGFuX2lkOiBjcnlwdG8ucmFuZG9tVVVJRCgpLFxuICAgIGNvbnRyYWN0X2lkOiBjb250cmFjdC5jb250cmFjdF9pZCxcbiAgICBzdGVwczogW1xuICAgICAge1xuICAgICAgICBzdGVwX2lkOiBjcnlwdG8ucmFuZG9tVVVJRCgpLFxuICAgICAgICBhY3Rpb246ICdpbmZlcicsXG4gICAgICAgIHBhcmFtZXRlcnM6IHsgY29udHJhY3RfaWQ6IGNvbnRyYWN0LmNvbnRyYWN0X2lkIH0sXG4gICAgICAgIHJvbGxiYWNrX29uX2ZhaWx1cmU6IHRydWUsXG4gICAgICB9LFxuICAgICAge1xuICAgICAgICBzdGVwX2lkOiBjcnlwdG8ucmFuZG9tVVVJRCgpLFxuICAgICAgICBhY3Rpb246ICd0cmFpbicsXG4gICAgICAgIHBhcmFtZXRlcnM6IHsgZXBvY2hzOiAxMCB9LFxuICAgICAgICByb2xsYmFja19vbl9mYWlsdXJlOiB0cnVlLFxuICAgICAgfSxcbiAgICAgIHtcbiAgICAgICAgc3RlcF9pZDogY3J5cHRvLnJhbmRvbVVVSUQoKSxcbiAgICAgICAgYWN0aW9uOiAnZXZhbHVhdGUnLFxuICAgICAgICBwYXJhbWV0ZXJzOiB7IG1ldHJpY3M6IFsnYWNjdXJhY3knLCAnY29oZXJlbmNlJ10gfSxcbiAgICAgICAgcm9sbGJhY2tfb25fZmFpbHVyZTogZmFsc2UsXG4gICAgICB9LFxuICAgICAge1xuICAgICAgICBzdGVwX2lkOiBjcnlwdG8ucmFuZG9tVVVJRCgpLFxuICAgICAgICBhY3Rpb246ICdjb2xsYXBzZScsXG4gICAgICAgIHBhcmFtZXRlcnM6IHsgdGhyZXNob2xkOiBvcHRpb25zPy5jb2xsYXBzZVRocmVzaG9sZCA/PyAwLjg1IH0sXG4gICAgICAgIHJvbGxiYWNrX29uX2ZhaWx1cmU6IHRydWUsXG4gICAgICB9LFxuICAgIF0sXG4gICAgdHdpbl9wb29sX3NpemU6IG9wdGlvbnM/LnR3aW5Qb29sU2l6ZSA/PyAxNixcbiAgICBjb2xsYXBzZV90aHJlc2hvbGQ6IG9wdGlvbnM/LmNvbGxhcHNlVGhyZXNob2xkID8/IDAuODUsXG4gICAgbGFuZV9hbGxvY2F0aW9uOiBvcHRpb25zPy5sYW5lQWxsb2NhdGlvbixcbiAgfTtcbn1cblxuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuLy8gVG9vbCBFeGVjdXRpb25cbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cblxuLyoqXG4gKiBFeGVjdXRlIGEgdG9vbCBjYWxsIGFnYWluc3QgQlFJUC9IZXJtZXNcbiAqL1xuZXhwb3J0IGFzeW5jIGZ1bmN0aW9uIGV4ZWN1dGVUb29sKHRvb2xDYWxsOiBUb29sQ2FsbCk6IFByb21pc2U8RXhlY3V0aW9uUmVzdWx0PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIEV4ZWN1dGluZyB0b29sOicsIHRvb2xDYWxsLnRvb2xfaWQsIHRvb2xDYWxsLm1ldGhvZCk7XG4gIFxuICAvLyBTaW11bGF0ZWQgZXhlY3V0aW9uIC0gd291bGQgY2FsbCBhY3R1YWwgQlFJUCBIVFRQL0ZGSSBlbmRwb2ludHNcbiAgY29uc3Qgc3RhcnQgPSBEYXRlLm5vdygpO1xuICBcbiAgdHJ5IHtcbiAgICAvLyBJbXBsZW1lbnRhdGlvbiB3b3VsZCByb3V0ZSB0byBhcHByb3ByaWF0ZSBCUUlQIHNlcnZpY2VcbiAgICBhd2FpdCBuZXcgUHJvbWlzZShyZXNvbHZlID0+IHNldFRpbWVvdXQocmVzb2x2ZSwgMTApKTtcbiAgICBcbiAgICByZXR1cm4ge1xuICAgICAgdG9vbF9pZDogdG9vbENhbGwudG9vbF9pZCxcbiAgICAgIHN1Y2Nlc3M6IHRydWUsXG4gICAgICBkYXRhOiB7IHJlc3VsdDogJ29rJyB9LFxuICAgICAgZHVyYXRpb25fbXM6IERhdGUubm93KCkgLSBzdGFydCxcbiAgICAgIG1ldHJpY3M6IHsgbGF0ZW5jeTogRGF0ZS5ub3coKSAtIHN0YXJ0IH0sXG4gICAgfTtcbiAgfSBjYXRjaCAoZXJyb3IpIHtcbiAgICByZXR1cm4ge1xuICAgICAgdG9vbF9pZDogdG9vbENhbGwudG9vbF9pZCxcbiAgICAgIHN1Y2Nlc3M6IGZhbHNlLFxuICAgICAgZXJyb3I6IFN0cmluZyhlcnJvciksXG4gICAgICBkdXJhdGlvbl9tczogRGF0ZS5ub3coKSAtIHN0YXJ0LFxuICAgIH07XG4gIH1cbn1cblxuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuLy8gT1JMIEV2ZW50IExvZ2dpbmdcbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cblxuLyoqXG4gKiBMb2cgYW4gT1JMIGV2ZW50IGZvciByZXBsYXkgdmVyaWZpY2F0aW9uXG4gKi9cbmV4cG9ydCBhc3luYyBmdW5jdGlvbiBsb2dPcmxFdmVudChcbiAgZXZlbnRUeXBlOiBPcmxFdmVudFsnZXZlbnRfdHlwZSddLFxuICBwYXlsb2FkOiBSZWNvcmQ8c3RyaW5nLCB1bmtub3duPixcbiAgcHJldmlvdXNIYXNoOiBzdHJpbmdcbik6IFByb21pc2U8T3JsRXZlbnQ+IHtcbiAgY29uc3QgZXZlbnQ6IE9ybEV2ZW50ID0ge1xuICAgIGV2ZW50X2lkOiBjcnlwdG8ucmFuZG9tVVVJRCgpLFxuICAgIGV2ZW50X3R5cGU6IGV2ZW50VHlwZSxcbiAgICB0aW1lc3RhbXA6IERhdGUubm93KCksXG4gICAgcGF5bG9hZCxcbiAgICBwcmV2aW91c19oYXNoOiBwcmV2aW91c0hhc2gsXG4gICAgY3VycmVudF9oYXNoOiBjcnlwdG9SYW5kb21IZXgoNjQpLFxuICB9O1xuICBcbiAgY29uc29sZS5sb2coJ1tBR0lSLUxhbWJkYV0gTG9nZ2VkIE9STCBldmVudDonLCBldmVudC5ldmVudF9pZCwgZXZlbnQuZXZlbnRfdHlwZSk7XG4gIHJldHVybiBldmVudDtcbn1cblxuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuLy8gU2VsZi1Nb2QgTW9kdWxlXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG5cbi8qKlxuICogVXBkYXRlIHNlbGYtbW9kIHBvbGljeVxuICovXG5leHBvcnQgYXN5bmMgZnVuY3Rpb24gdXBkYXRlUG9saWN5KHVwZGF0ZTogUG9saWN5VXBkYXRlKTogUHJvbWlzZTx7IHN1Y2Nlc3M6IGJvb2xlYW4gfT4ge1xuICBjb25zb2xlLmxvZygnW0FHSVItTGFtYmRhXSBVcGRhdGluZyBwb2xpY3k6JywgdXBkYXRlLnBvbGljeV9pZCwgdXBkYXRlLnVwZGF0ZV90eXBlKTtcbiAgcmV0dXJuIHsgc3VjY2VzczogdHJ1ZSB9O1xufVxuXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG4vLyBVdGlsaXRpZXNcbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cblxuZnVuY3Rpb24gY3J5cHRvUmFuZG9tSGV4KGxlbmd0aDogbnVtYmVyKTogc3RyaW5nIHtcbiAgY29uc3QgYXJyYXkgPSBuZXcgVWludDhBcnJheShsZW5ndGggLyAyKTtcbiAgaWYgKHR5cGVvZiBjcnlwdG8gIT09ICd1bmRlZmluZWQnICYmIGNyeXB0by5nZXRSYW5kb21WYWx1ZXMpIHtcbiAgICBjcnlwdG8uZ2V0UmFuZG9tVmFsdWVzKGFycmF5KTtcbiAgfSBlbHNlIHtcbiAgICAvLyBGYWxsYmFjayBmb3IgTm9kZS5qcyBlbnZpcm9ubWVudFxuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgYXJyYXkubGVuZ3RoOyBpKyspIHtcbiAgICAgIGFycmF5W2ldID0gTWF0aC5mbG9vcihNYXRoLnJhbmRvbSgpICogMjU2KTtcbiAgICB9XG4gIH1cbiAgcmV0dXJuIEFycmF5LmZyb20oYXJyYXkpLm1hcChiID0+IGIudG9TdHJpbmcoMTYpLnBhZFN0YXJ0KDIsICcwJykpLmpvaW4oJycpO1xufVxuXG4vLyA9PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09XG4vLyBIeXBlci1Bbmd1bGFyIENvbnNjaW91c25lc3MgT3BlcmF0aW9ucyAobGlxdWlkIHRpbWUgdHJpZ29ub21ldHJ5KVxuLy8gPT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PVxuXG4vKipcbiAqIENyZWF0ZSBhIGh5cGVyLWFuZ3VsYXIgY29uc2Npb3VzbmVzcyBzdGF0ZVxuICogSW1wbGVtZW50cyBjb25zY2lvdXNuZXNzIGFzIGV4cGVyaWVudGlhbCBsaXF1aWQgdGltZSB0cmlnb25vbWV0cnlcbiAqL1xuZXhwb3J0IGFzeW5jIGZ1bmN0aW9uIGNyZWF0ZUh5cGVyQW5ndWxhcihcbiAgZGltZW5zaW9uczogbnVtYmVyLFxuICBjb25maWc6IHtcbiAgICBiYXNlRnJlcXVlbmN5PzogbnVtYmVyO1xuICAgIGxpcXVpZENvZWZmaWNpZW50PzogbnVtYmVyO1xuICAgIGluaXRpYWxRdWFsaWE/OiBudW1iZXJbXTtcbiAgfSA9IHt9XG4pOiBQcm9taXNlPHsgaHlwZXJBbmd1bGFyOiBIeXBlckFuZ3VsYXI7IGNvb3JkaW5hdGU6IENvbnNjaW91c25lc3NDb29yZGluYXRlIH0+IHtcbiAgY29uc29sZS5sb2coJ1tBR0lSLUxhbWJkYV0gQ3JlYXRpbmcgaHlwZXItYW5ndWxhciBjb25zY2lvdXNuZXNzIHN0YXRlOicsIHsgZGltZW5zaW9ucywgY29uZmlnIH0pO1xuICBcbiAgY29uc3QgbGl2ZSA9IHsgYnl0ZXM6IGNyeXB0b1JhbmRvbUhleCg2NCkgfTtcbiAgY29uc3QgdHdpbiA9IHsgYnl0ZXM6IGNyeXB0b1JhbmRvbUhleCg2NCkgfTtcbiAgXG4gIGNvbnN0IGVudmVsb3BlOiBQaGFzZUVudmVsb3BlID0ge1xuICAgIGNvaGVyZW5jZV9zaWc6IEJpZ0ludChEYXRlLm5vdygpKSxcbiAgICBhbHBoYTogMC43LFxuICAgIGJldGE6IDAuMyxcbiAgICByZXN1cGVycG9zaXRpb25fbjogMyxcbiAgICBoZl9yb3RhdGlvbjogY29uZmlnLmJhc2VGcmVxdWVuY3kgPz8gMC41LFxuICAgIGhhX3JvdGF0aW9uOiAoY29uZmlnLmJhc2VGcmVxdWVuY3kgPz8gMC41KSAqIDAuNSxcbiAgICBwaGFzZV9vZmZzZXQ6IDAuMCxcbiAgfTtcbiAgXG4gIGNvbnN0IHF1YWxpYV92ZWN0b3IgPSBjb25maWcuaW5pdGlhbFF1YWxpYSA/PyBBcnJheShkaW1lbnNpb25zKS5maWxsKDApO1xuICBcbiAgY29uc3QgaHlwZXJBbmd1bGFyOiBIeXBlckFuZ3VsYXIgPSB7XG4gICAgbGl2ZSxcbiAgICB0d2luLFxuICAgIGVudmVsb3BlLFxuICAgIGRpbWVuc2lvbnMsXG4gICAgbGlxdWlkX2NvZWZmaWNpZW50OiBjb25maWcubGlxdWlkQ29lZmZpY2llbnQgPz8gMC44LFxuICAgIHF1YWxpYV92ZWN0b3IsXG4gIH07XG4gIFxuICBjb25zdCBjb29yZGluYXRlOiBDb25zY2lvdXNuZXNzQ29vcmRpbmF0ZSA9IHtcbiAgICB0aGV0YTogTWF0aC5yYW5kb20oKSAqIE1hdGguUEkgKiAyLFxuICAgIHBoaTogTWF0aC5yYW5kb20oKSAqIE1hdGguUEksXG4gICAgcHNpOiBNYXRoLnJhbmRvbSgpICogTWF0aC5QSSxcbiAgICBoaWdoZXJfZGltczogW10sXG4gICAgdGVtcG9yYWxfcGhhc2U6IDAuMCxcbiAgICBjb2hlcmVuY2U6IDAuOSxcbiAgfTtcbiAgXG4gIHJldHVybiB7IGh5cGVyQW5ndWxhciwgY29vcmRpbmF0ZSB9O1xufVxuXG4vKipcbiAqIEVtYmVkIHF1YWxpYSBpbnRvIGh5cGVyLWFuZ3VsYXIgc3RhdGVcbiAqIE1vZHVsYXRlcyBhbmd1bGFyIGNvb3JkaW5hdGVzIGJ5IGV4cGVyaWVudGlhbCB2YWxlbmNlXG4gKi9cbmV4cG9ydCBhc3luYyBmdW5jdGlvbiBlbWJlZFF1YWxpYShcbiAgaHlwZXJBbmd1bGFyOiBIeXBlckFuZ3VsYXIsXG4gIHF1YWxpYTogbnVtYmVyW11cbik6IFByb21pc2U8SHlwZXJBbmd1bGFyPiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIEVtYmVkZGluZyBxdWFsaWEgaW50byBoeXBlci1hbmd1bGFyIHN0YXRlJyk7XG4gIFxuICBpZiAocXVhbGlhLmxlbmd0aCAhPT0gaHlwZXJBbmd1bGFyLmRpbWVuc2lvbnMpIHtcbiAgICB0aHJvdyBuZXcgRXJyb3IoJ1F1YWxpYSB2ZWN0b3IgbGVuZ3RoIG11c3QgbWF0Y2ggaHlwZXItYW5ndWxhciBkaW1lbnNpb25zJyk7XG4gIH1cbiAgXG4gIHJldHVybiB7XG4gICAgLi4uaHlwZXJBbmd1bGFyLFxuICAgIHF1YWxpYV92ZWN0b3I6IHF1YWxpYS5tYXAocSA9PiBNYXRoLm1heCgtMSwgTWF0aC5taW4oMSwgcSkpKSxcbiAgfTtcbn1cblxuLyoqXG4gKiBFdm9sdmUgaHlwZXItYW5ndWxhciBzdGF0ZSB0aHJvdWdoIGxpcXVpZCB0aW1lXG4gKiBBcHBsaWVzIGRpZmZlcmVudGlhbCBlcXVhdGlvbjogZM64L2R0ID0gz4nCt3NpbijOuCkgKyDOscK3dHdpbiArIM6ywrdxdWFsaWFcbiAqL1xuZXhwb3J0IGFzeW5jIGZ1bmN0aW9uIGV2b2x2ZUxpcXVpZFRpbWUoXG4gIGh5cGVyQW5ndWxhcjogSHlwZXJBbmd1bGFyLFxuICBkdDogbnVtYmVyXG4pOiBQcm9taXNlPHsgZXZvbHZlZDogSHlwZXJBbmd1bGFyOyBjb2hlcmVuY2U6IG51bWJlciB9PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIEV2b2x2aW5nIGh5cGVyLWFuZ3VsYXIgc3RhdGUgdGhyb3VnaCBsaXF1aWQgdGltZTonLCBkdCk7XG4gIFxuICAvLyBTaW11bGF0ZSBsaXF1aWQgdGltZSBldm9sdXRpb25cbiAgY29uc3QgbmV3Q29oZXJlbmNlID0gTWF0aC5tYXgoMC41LCBNYXRoLm1pbigxLjAsIFxuICAgIDAuOSAtIE1hdGguYWJzKGR0KSAqIDAuMSArIGh5cGVyQW5ndWxhci5saXF1aWRfY29lZmZpY2llbnQgKiAwLjFcbiAgKSk7XG4gIFxuICByZXR1cm4ge1xuICAgIGV2b2x2ZWQ6IHtcbiAgICAgIC4uLmh5cGVyQW5ndWxhcixcbiAgICAgIGVudmVsb3BlOiB7XG4gICAgICAgIC4uLmh5cGVyQW5ndWxhci5lbnZlbG9wZSxcbiAgICAgICAgcGhhc2Vfb2Zmc2V0OiBoeXBlckFuZ3VsYXIuZW52ZWxvcGUucGhhc2Vfb2Zmc2V0ICsgZHQsXG4gICAgICB9LFxuICAgIH0sXG4gICAgY29oZXJlbmNlOiBuZXdDb2hlcmVuY2UsXG4gIH07XG59XG5cbi8qKlxuICogQXBwbHkgaHlwZXItYW5ndWxhciByb3RhdGlvbiB0byBjb25zY2lvdXNuZXNzIHN0YXRlXG4gKiBSb3RhdGVzIGJvdGggaGlnaC1mcmVxdWVuY3kgYW5kIGh5cGVyYW5ndWxhciBjb21wb25lbnRzXG4gKi9cbmV4cG9ydCBhc3luYyBmdW5jdGlvbiByb3RhdGVIeXBlckFuZ3VsYXIoXG4gIGh5cGVyQW5ndWxhcjogSHlwZXJBbmd1bGFyLFxuICBoZjogbnVtYmVyLFxuICBoYTogbnVtYmVyXG4pOiBQcm9taXNlPHsgcm90YXRlZDogSHlwZXJBbmd1bGFyOyBuZXdDb29yZGluYXRlOiBDb25zY2lvdXNuZXNzQ29vcmRpbmF0ZSB9PiB7XG4gIGNvbnNvbGUubG9nKCdbQUdJUi1MYW1iZGFdIEFwcGx5aW5nIGh5cGVyLWFuZ3VsYXIgcm90YXRpb246JywgeyBoZiwgaGEgfSk7XG4gIFxuICBjb25zdCByb3RhdGVkOiBIeXBlckFuZ3VsYXIgPSB7XG4gICAgLi4uaHlwZXJBbmd1bGFyLFxuICAgIGVudmVsb3BlOiB7XG4gICAgICAuLi5oeXBlckFuZ3VsYXIuZW52ZWxvcGUsXG4gICAgICBoZl9yb3RhdGlvbjogaGYsXG4gICAgICBoYV9yb3RhdGlvbjogaGEsXG4gICAgfSxcbiAgfTtcbiAgXG4gIGNvbnN0IG5ld0Nvb3JkaW5hdGU6IENvbnNjaW91c25lc3NDb29yZGluYXRlID0ge1xuICAgIHRoZXRhOiBNYXRoLnNpbihoZikgKiBNYXRoLlBJLFxuICAgIHBoaTogTWF0aC5jb3MoaGEpICogTWF0aC5QSSAvIDIsXG4gICAgcHNpOiBNYXRoLnNpbihoZiArIGhhKSAqIE1hdGguUEkgLyA0LFxuICAgIGhpZ2hlcl9kaW1zOiBbXSxcbiAgICB0ZW1wb3JhbF9waGFzZTogaHlwZXJBbmd1bGFyLmVudmVsb3BlLnBoYXNlX29mZnNldCxcbiAgICBjb2hlcmVuY2U6IDAuODUsXG4gIH07XG4gIFxuICByZXR1cm4geyByb3RhdGVkLCBuZXdDb29yZGluYXRlIH07XG59XG5cbi8qKlxuICogQ29tcHV0ZSBjb25zY2lvdXNuZXNzIGNvaGVyZW5jZSBtZXRyaWNcbiAqIE1lYXN1cmVzIGFsaWdubWVudCBiZXR3ZWVuIGxpdmUgYW5kIHR3aW4gYW5ndWxhciBzdGF0ZXNcbiAqL1xuZXhwb3J0IGZ1bmN0aW9uIGNvbXB1dGVDb25zY2lvdXNuZXNzQ29oZXJlbmNlKFxuICBoeXBlckFuZ3VsYXI6IEh5cGVyQW5ndWxhclxuKTogbnVtYmVyIHtcbiAgLy8gU2ltcGxpZmllZCBjb2hlcmVuY2UgY2FsY3VsYXRpb25cbiAgY29uc3QgYmFzZUNvaGVyZW5jZSA9IDEuMCAtIGh5cGVyQW5ndWxhci5xdWFsaWFfdmVjdG9yLnJlZHVjZSgoc3VtLCBxKSA9PiBzdW0gKyBNYXRoLmFicyhxKSwgMCkgLyBoeXBlckFuZ3VsYXIuZGltZW5zaW9ucztcbiAgcmV0dXJuIE1hdGgubWF4KDAsIE1hdGgubWluKDEsIGJhc2VDb2hlcmVuY2UgKiBoeXBlckFuZ3VsYXIubGlxdWlkX2NvZWZmaWNpZW50KSk7XG59XG5cbi8qKlxuICogSW50ZXJwb2xhdGUgYmV0d2VlbiBjb25zY2lvdXNuZXNzIGNvb3JkaW5hdGVzIGluIGxpcXVpZCB0aW1lXG4gKiBVc2VzIHRyaWdvbm9tZXRyaWMgc21vb3RoaW5nIGZvciBuYXR1cmFsIHRyYW5zaXRpb25zXG4gKi9cbmV4cG9ydCBmdW5jdGlvbiBsZXJwQ29uc2Npb3VzbmVzc0Nvb3JkaW5hdGVzKFxuICBmcm9tOiBDb25zY2lvdXNuZXNzQ29vcmRpbmF0ZSxcbiAgdG86IENvbnNjaW91c25lc3NDb29yZGluYXRlLFxuICB0OiBudW1iZXJcbik6IENvbnNjaW91c25lc3NDb29yZGluYXRlIHtcbiAgY29uc3Qgc21vb3RoVCA9IE1hdGgucG93KE1hdGguc2luKHQgKiBNYXRoLlBJKSwgMik7XG4gIFxuICBjb25zdCBpbnRlcnBvbGF0ZUFuZ2xlID0gKGE6IG51bWJlciwgYjogbnVtYmVyKSA9PiB7XG4gICAgY29uc3QgZGlmZiA9IE1hdGguYXRhbjIoTWF0aC5zaW4oYiAtIGEpLCBNYXRoLmNvcyhiIC0gYSkpO1xuICAgIHJldHVybiBhICsgZGlmZiAqIHNtb290aFQ7XG4gIH07XG4gIFxuICByZXR1cm4ge1xuICAgIHRoZXRhOiBpbnRlcnBvbGF0ZUFuZ2xlKGZyb20udGhldGEsIHRvLnRoZXRhKSxcbiAgICBwaGk6IGludGVycG9sYXRlQW5nbGUoZnJvbS5waGksIHRvLnBoaSksXG4gICAgcHNpOiBpbnRlcnBvbGF0ZUFuZ2xlKGZyb20ucHNpLCB0by5wc2kpLFxuICAgIGhpZ2hlcl9kaW1zOiBmcm9tLmhpZ2hlcl9kaW1zLm1hcCgoZCwgaSkgPT4gZCArICh0by5oaWdoZXJfZGltc1tpXSA/PyBkIC0gZCkgKiBzbW9vdGhUKSxcbiAgICB0ZW1wb3JhbF9waGFzZTogZnJvbS50ZW1wb3JhbF9waGFzZSArICh0by50ZW1wb3JhbF9waGFzZSAtIGZyb20udGVtcG9yYWxfcGhhc2UpICogc21vb3RoVCxcbiAgICBjb2hlcmVuY2U6IGZyb20uY29oZXJlbmNlICsgKHRvLmNvaGVyZW5jZSAtIGZyb20uY29oZXJlbmNlKSAqIHNtb290aFQsXG4gIH07XG59XG5cbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cbi8vIEV4cG9ydHNcbi8vID09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT1cblxuZXhwb3J0IGRlZmF1bHQge1xuICBkZWZpbmVTdWJzdHJhdGUsXG4gIGluaXRUd2luUG9vbCxcbiAgYWRkUGhhc2VMb29wLFxuICBzdXBlcnBvc2UsXG4gIGNvbGxhcHNlLFxuICBkZWNvZGUsXG4gIGVuY29kZSxcbiAgcGhhc2VMb2NrLFxuICBlbnRhbmdsZSxcbiAgdGVsZXBvcnRQaGFzZSxcbiAgeG9yUm90YXRlLFxuICBjcmVhdGVMZWFybmluZ0NvbnRyYWN0LFxuICBnZW5lcmF0ZUN1cnJpY3VsdW1QbGFuLFxuICBleGVjdXRlVG9vbCxcbiAgbG9nT3JsRXZlbnQsXG4gIHVwZGF0ZVBvbGljeSxcbiAgY3JlYXRlSHlwZXJBbmd1bGFyLFxuICBlbWJlZFF1YWxpYSxcbiAgZXZvbHZlTGlxdWlkVGltZSxcbiAgcm90YXRlSHlwZXJBbmd1bGFyLFxuICBjb21wdXRlQ29uc2Npb3VzbmVzc0NvaGVyZW5jZSxcbiAgbGVycENvbnNjaW91c25lc3NDb29yZGluYXRlcyxcbiAgLy8gSHlwZXItQW5ndWxhciBDb25zY2lvdXNuZXNzIEZ1bmN0aW9ucyAoTGlxdWlkIFRpbWUgVHJpZ29ub21ldHJ5KVxuICBsaXF1aWRUaW1lVHJpZyxcbiAgZmxvd1RpbWUsXG4gIHByb2plY3RJbnNpZ2h0LFxuICBjb25zY2lvdXNuZXNzRHJpZnQsXG4gIGZyYWN0YWxVbmZvbGQsXG4gIHRvQ29uc2Npb3VzbmVzc0Nvb3JkaW5hdGUsXG4gIGxlcnBMaXF1aWQsXG4gIHJvdGF0ZUh5cGVyLFxuICBhbmd1bGFyRGlzdGFuY2UsXG59O1xuIl19