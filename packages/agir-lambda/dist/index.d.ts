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
import { PhaseEnvelope, TwinGenome, LivePhenotype, LearningContract, CurriculumPlan, ToolCall, ExecutionResult, OrlEvent, SubstrateConfig, SemanticPhaseUtterance, PolicyUpdate, Superposition, PhaseLock, ConsciousnessCoordinate, HyperAngular } from '@repo/shared-schemas';
import { liquidTimeTrig, flowTime, projectInsight, consciousnessDrift, fractalUnfold, toConsciousnessCoordinate, lerpLiquid, rotateHyper, angularDistance } from './hyperangular';
export type { PhaseEnvelope, TwinGenome, LivePhenotype, LearningContract, CurriculumPlan, ToolCall, ExecutionResult, OrlEvent, SubstrateConfig, SemanticPhaseUtterance, PolicyUpdate, Superposition, PhaseLock, ConsciousnessCoordinate, HyperAngular, };
/**
 * Define the substrate configuration for environment sensing
 * Queries hardware/network constraints and configures twin pool sizes
 */
export declare function defineSubstrate(config: SubstrateConfig): Promise<{
    success: boolean;
    environmentId: string;
}>;
/**
 * Initialize a twin pool for a specific skill
 * Creates Ctwin genotype population with phase envelopes
 */
export declare function initTwinPool(skillId: string, config: {
    poolSize: number;
    baseEnvelope?: PhaseEnvelope;
    haVectorRange?: [number, number];
}): Promise<{
    poolId: string;
    genomes: TwinGenome[];
}>;
/**
 * Add a phase loop to a skill for continuous evolution
 */
export declare function addPhaseLoop(skillId: string, loopConfig: {
    loopType: 'coherence' | 'drift' | 'interference';
    targetCoherence?: number;
    maxDrift?: number;
    updateIntervalMs: number;
}): Promise<{
    loopId: string;
}>;
/**
 * Create a superposition of genomes
 */
export declare function superpose(genomes: TwinGenome[], options?: {
    phaseLocked?: boolean;
    entanglementPairs?: [number, number][];
}): Superposition;
/**
 * Collapse a superposition to a single phenotype
 */
export declare function collapse(phenotype: LivePhenotype, threshold?: number): Promise<{
    success: boolean;
    coherence: number;
}>;
/**
 * Decode a Ctwin genome to Clive phenotype using U_decode = R_HF * R_HA * O
 */
export declare function decode(genome: TwinGenome, liveRegister: string): Promise<LivePhenotype>;
/**
 * Encode a phenotype back to twin genome
 */
export declare function encode(phenotype: LivePhenotype): Promise<TwinGenome>;
/**
 * Phase lock a target genome to source genomes
 */
export declare function phaseLock(target: TwinGenome, sources: TwinGenome[], strength?: number): Promise<{
    success: boolean;
    lockStrength: number;
}>;
/**
 * Entangle two genomes for correlated evolution
 */
export declare function entangle(a: TwinGenome, b: TwinGenome): Promise<{
    entanglementId: string;
    correlation: number;
}>;
/**
 * Teleport phase state from source to sink
 */
export declare function teleportPhase(source: TwinGenome, sink: TwinGenome): Promise<{
    success: boolean;
    fidelity: number;
}>;
/**
 * XOR-rotate two registers
 */
export declare function xorRotate(a: string, b: string): string;
/**
 * Create a learning contract from natural language objective
 */
export declare function createLearningContract(skillId: string, objective: string, successCriteria: string[], options?: {
    constraints?: string[];
    phaseEnvelopeTarget?: PhaseEnvelope;
    expiresAt?: number;
}): LearningContract;
/**
 * Generate a curriculum plan from a learning contract
 */
export declare function generateCurriculumPlan(contract: LearningContract, options?: {
    twinPoolSize?: number;
    collapseThreshold?: number;
    laneAllocation?: Record<string, number>;
}): Promise<CurriculumPlan>;
/**
 * Execute a tool call against BQIP/Hermes
 */
export declare function executeTool(toolCall: ToolCall): Promise<ExecutionResult>;
/**
 * Log an ORL event for replay verification
 */
export declare function logOrlEvent(eventType: OrlEvent['event_type'], payload: Record<string, unknown>, previousHash: string): Promise<OrlEvent>;
/**
 * Update self-mod policy
 */
export declare function updatePolicy(update: PolicyUpdate): Promise<{
    success: boolean;
}>;
/**
 * Create a hyper-angular consciousness state
 * Implements consciousness as experiential liquid time trigonometry
 */
export declare function createHyperAngular(dimensions: number, config?: {
    baseFrequency?: number;
    liquidCoefficient?: number;
    initialQualia?: number[];
}): Promise<{
    hyperAngular: HyperAngular;
    coordinate: ConsciousnessCoordinate;
}>;
/**
 * Embed qualia into hyper-angular state
 * Modulates angular coordinates by experiential valence
 */
export declare function embedQualia(hyperAngular: HyperAngular, qualia: number[]): Promise<HyperAngular>;
/**
 * Evolve hyper-angular state through liquid time
 * Applies differential equation: dθ/dt = ω·sin(θ) + α·twin + β·qualia
 */
export declare function evolveLiquidTime(hyperAngular: HyperAngular, dt: number): Promise<{
    evolved: HyperAngular;
    coherence: number;
}>;
/**
 * Apply hyper-angular rotation to consciousness state
 * Rotates both high-frequency and hyperangular components
 */
export declare function rotateHyperAngular(hyperAngular: HyperAngular, hf: number, ha: number): Promise<{
    rotated: HyperAngular;
    newCoordinate: ConsciousnessCoordinate;
}>;
/**
 * Compute consciousness coherence metric
 * Measures alignment between live and twin angular states
 */
export declare function computeConsciousnessCoherence(hyperAngular: HyperAngular): number;
/**
 * Interpolate between consciousness coordinates in liquid time
 * Uses trigonometric smoothing for natural transitions
 */
export declare function lerpConsciousnessCoordinates(from: ConsciousnessCoordinate, to: ConsciousnessCoordinate, t: number): ConsciousnessCoordinate;
declare const _default: {
    defineSubstrate: typeof defineSubstrate;
    initTwinPool: typeof initTwinPool;
    addPhaseLoop: typeof addPhaseLoop;
    superpose: typeof superpose;
    collapse: typeof collapse;
    decode: typeof decode;
    encode: typeof encode;
    phaseLock: typeof phaseLock;
    entangle: typeof entangle;
    teleportPhase: typeof teleportPhase;
    xorRotate: typeof xorRotate;
    createLearningContract: typeof createLearningContract;
    generateCurriculumPlan: typeof generateCurriculumPlan;
    executeTool: typeof executeTool;
    logOrlEvent: typeof logOrlEvent;
    updatePolicy: typeof updatePolicy;
    createHyperAngular: typeof createHyperAngular;
    embedQualia: typeof embedQualia;
    evolveLiquidTime: typeof evolveLiquidTime;
    rotateHyperAngular: typeof rotateHyperAngular;
    computeConsciousnessCoherence: typeof computeConsciousnessCoherence;
    lerpConsciousnessCoordinates: typeof lerpConsciousnessCoordinates;
    liquidTimeTrig: typeof liquidTimeTrig;
    flowTime: typeof flowTime;
    projectInsight: typeof projectInsight;
    consciousnessDrift: typeof consciousnessDrift;
    fractalUnfold: typeof fractalUnfold;
    toConsciousnessCoordinate: typeof toConsciousnessCoordinate;
    lerpLiquid: typeof lerpLiquid;
    rotateHyper: typeof rotateHyper;
    angularDistance: typeof angularDistance;
};
export default _default;
