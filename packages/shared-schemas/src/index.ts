/**
 * @repo/shared-schemas - Shared Zod schemas for BQIP AGI architecture
 * 
 * These schemas define cross-layer communication contracts between:
 * - Rust BQIP substrate (serde equivalents)
 * - TypeScript AGIR-Lambda cognitive layer
 * - Hermes agent layer
 */

import { z } from 'zod';

// ============================================================================
// Phase Envelope & Dual-State Types (mirrors bqip-core PhaseEnvelope, DualState)
// ============================================================================

export const PhaseEnvelopeSchema = z.object({
  coherence_sig: z.bigint().describe("Coherence signature for phase identity"),
  alpha: z.number().finite().describe("Alpha amplitude (unit circle)"),
  beta: z.number().finite().describe("Beta amplitude (unit circle)"),
  resuperposition_n: z.number().int().nonnegative().describe("Resuperposition depth"),
  hf_rotation: z.number().finite().optional().default(0).describe("High-frequency rotation parameter"),
  ha_rotation: z.number().finite().optional().default(0).describe("Hyperangular rotation parameter"),
  phase_offset: z.number().finite().optional().default(0).describe("Phase offset for liquid time-constant"),
});

export type PhaseEnvelope = z.infer<typeof PhaseEnvelopeSchema>;

export const RegisterSchema = z.object({
  bytes: z.string().regex(/^[0-9a-f]{64}$/i).describe("32-byte register as hex string"),
});

export type Register = z.infer<typeof RegisterSchema>;

export const DualStateSchema = z.object({
  live: RegisterSchema.describe("Live state register"),
  twin: RegisterSchema.describe("Twin state register (phase-projected from live)"),
});

export type DualState = z.infer<typeof DualStateSchema>;

// ============================================================================
// TEPE Genome Types (mirrors bqip-evolver TwinGenome, LivePhenotype)
// ============================================================================

export const TwinGenomeSchema = z.object({
  id: z.bigint().describe("Genome identifier"),
  generation: z.number().int().nonnegative().describe("Generation number"),
  envelope: PhaseEnvelopeSchema.describe("Phase envelope parameters"),
  ha_vector: z.array(z.number().finite()).length(4).describe("Hyperangular rotation vector [f32; 4]"),
  orl_backpointer: z.string().regex(/^[0-9a-f]{64}$/i).describe("ORL backpointer hash for replay"),
  crypto_binding: z.string().regex(/^[0-9a-f]{64}$/i).describe("Cryptographic binding hash"),
  lane_binding: z.enum(['Ipv4', 'Ipv6', 'PublicApi', 'PublicProxy', 'GenericEndpoint']).describe("Lane binding type"),
});

export type TwinGenome = z.infer<typeof TwinGenomeSchema>;

export const LivePhenotypeSchema = z.object({
  register_id: RegisterSchema.describe("Decoded register ID"),
  envelope: PhaseEnvelopeSchema.describe("Decoded phase envelope"),
  state: DualStateSchema.describe("Dual-state phenotype"),
  routing_lane: z.enum(['Ipv4', 'Ipv6', 'PublicApi', 'PublicProxy', 'GenericEndpoint']).describe("Routing lane"),
});

export type LivePhenotype = z.infer<typeof LivePhenotypeSchema>;

// ============================================================================
// Learning Contracts & Curriculum Plans (DSVM/CESC integration)
// ============================================================================

export const LearningContractSchema = z.object({
  contract_id: z.string().uuid().describe("Unique contract identifier"),
  skill_id: z.string().describe("Target skill identifier"),
  objective: z.string().describe("Natural language objective"),
  success_criteria: z.array(z.string()).describe("Measurable success criteria"),
  constraints: z.array(z.string()).optional().describe("Operational constraints"),
  phase_envelope_target: PhaseEnvelopeSchema.optional().describe("Target phase envelope"),
  created_at: z.number().int().positive().describe("Unix timestamp"),
  expires_at: z.number().int().positive().optional().describe("Optional expiration"),
});

export type LearningContract = z.infer<typeof LearningContractSchema>;

export const CurriculumStepSchema = z.object({
  step_id: z.string().uuid(),
  action: z.enum(['infer', 'train', 'evaluate', 'collapse', 'commit', 'self_mod']),
  parameters: z.record(z.unknown()),
  expected_duration_ms: z.number().int().positive().optional(),
  rollback_on_failure: z.boolean().default(true),
});

export type CurriculumStep = z.infer<typeof CurriculumStepSchema>;

export const CurriculumPlanSchema = z.object({
  plan_id: z.string().uuid().describe("Unique plan identifier"),
  contract_id: z.string().uuid().describe("Parent contract ID"),
  steps: z.array(CurriculumStepSchema).describe("Ordered execution steps"),
  lane_allocation: z.record(z.string(), z.number().int().positive()).optional().describe("Lane -> weight mapping"),
  twin_pool_size: z.number().int().positive().default(16).describe("Twin pool size for TEPE"),
  collapse_threshold: z.number().min(0).max(1).default(0.85).describe("Phase coherence threshold for collapse"),
});

export type CurriculumPlan = z.infer<typeof CurriculumPlanSchema>;

// ============================================================================
// Tool Calls & Execution Results (JSON-first cognition)
// ============================================================================

export const ToolCallSchema = z.object({
  tool_id: z.string().describe("Tool identifier (e.g., 'bqip-infer', 'hermes-memory')"),
  method: z.string().describe("Method to invoke"),
  arguments: z.record(z.unknown()).describe("Method arguments"),
  timeout_ms: z.number().int().positive().optional().describe("Optional timeout"),
});

export type ToolCall = z.infer<typeof ToolCallSchema>;

export const ExecutionResultSchema = z.object({
  tool_id: z.string(),
  success: z.boolean(),
  data: z.unknown().optional().describe("Result data if successful"),
  error: z.string().optional().describe("Error message if failed"),
  metrics: z.record(z.number()).optional().describe("Performance metrics"),
  duration_ms: z.number().int().nonnegative(),
});

export type ExecutionResult = z.infer<typeof ExecutionResultSchema>;

// ============================================================================
// ORL Events (Observable Replay Log for verification)
// ============================================================================

export const OrlEventTypeSchema = z.enum([
  'twin_commit_local',
  'twin_commit_lane',
  'twin_commit_global',
  'phase_collapse',
  'ga_mutation',
  'ga_crossover',
  'fitness_evaluation',
  'self_mod_policy_update',
  'environment_sense',
]);

export type OrlEventType = z.infer<typeof OrlEventTypeSchema>;

export const OrlEventSchema = z.object({
  event_id: z.string().uuid(),
  event_type: OrlEventTypeSchema,
  timestamp: z.number().int().positive(),
  payload: z.record(z.unknown()).describe("Event-specific payload"),
  previous_hash: z.string().regex(/^[0-9a-f]{64}$/i).describe("Hash of previous event (chain integrity)"),
  current_hash: z.string().regex(/^[0-9a-f]{64}$/i).describe("Hash of this event"),
  signature: z.string().optional().describe("Cryptographic signature for global commits"),
});

export type OrlEvent = z.infer<typeof OrlEventSchema>;

// ============================================================================
// Substrate Configuration (environment sensing)
// ============================================================================

export const HardwareConfigSchema = z.object({
  cpu_cores: z.number().int().positive(),
  gpu_available: z.boolean(),
  gpu_memory_gb: z.number().nonnegative().optional(),
  system_memory_gb: z.number().positive(),
  network_bandwidth_mbps: z.number().positive().optional(),
});

export type HardwareConfig = z.infer<typeof HardwareConfigSchema>;

export const SubstrateConfigSchema = z.object({
  environment_id: z.string().uuid(),
  hardware: HardwareConfigSchema,
  twin_pool_budget: z.number().int().positive().describe("Max twin pool size"),
  phase_budget: z.number().min(0).max(1).describe("Phase computation budget"),
  latency_budget_ms: z.number().positive().describe("Latency constraint"),
  enable_gpu: z.boolean().default(false),
  enable_network_routing: z.boolean().default(false),
});

export type SubstrateConfig = z.infer<typeof SubstrateConfigSchema>;

// ============================================================================
// Semantic Phase Utterance (Hermes dual-state memory)
// ============================================================================

export const SemanticPhaseUtteranceSchema = z.object({
  utterance_id: z.string().uuid(),
  text: z.string().describe("Utterance text content"),
  semantics: z.record(z.number()).describe("Semantic embedding vector"),
  phase_tuple: z.object({
    valence: z.number().min(-1).max(1).describe("Emotional valence"),
    dominance: z.number().min(-1).max(1).describe("Dominance level"),
    arousal: z.number().min(-1).max(1).describe("Arousal level"),
  }).describe("Experiential phase tuple"),
  session_id: z.string().uuid().optional().describe("Parent session ID"),
  skill_id: z.string().optional().describe("Associated skill ID"),
  timestamp: z.number().int().positive(),
});

export type SemanticPhaseUtterance = z.infer<typeof SemanticPhaseUtteranceSchema>;

// ============================================================================
// Policy Updates (SelfModModule strategies)
// ============================================================================

export const PolicyUpdateSchema = z.object({
  policy_id: z.string().uuid(),
  update_type: z.enum(['tool_selection', 'lane_allocation', 'twin_acceptance', 'learning_strategy']),
  old_value: z.unknown().optional(),
  new_value: z.unknown(),
  rationale: z.string().optional().describe("Reason for update"),
  confidence: z.number().min(0).max(1).describe("Confidence in update"),
  timestamp: z.number().int().positive(),
});

export type PolicyUpdate = z.infer<typeof PolicyUpdateSchema>;

// ============================================================================
// Superposition & Phase Operations (AGIR-Lambda primitives)
// ============================================================================

export const SuperpositionSchema = z.object({
  genomes: z.array(TwinGenomeSchema).min(1),
  phase_locked: z.boolean().default(false),
  entanglement_pairs: z.array(z.tuple([z.number().int(), z.number().int()])).optional(),
});

export type Superposition<T = TwinGenome> = z.infer<typeof SuperpositionSchema>;

export const PhaseLockSchema = z.object({
  target_genome_id: z.bigint(),
  source_genome_ids: z.array(z.bigint()),
  lock_strength: z.number().min(0).max(1).default(1.0),
});

export type PhaseLock = z.infer<typeof PhaseLockSchema>;

// ============================================================================
// Exports (schemas only - types are exported via type re-exports)
// ============================================================================

export const schemas = {
  PhaseEnvelope: PhaseEnvelopeSchema,
  Register: RegisterSchema,
  DualState: DualStateSchema,
  TwinGenome: TwinGenomeSchema,
  LivePhenotype: LivePhenotypeSchema,
  LearningContract: LearningContractSchema,
  CurriculumStep: CurriculumStepSchema,
  CurriculumPlan: CurriculumPlanSchema,
  ToolCall: ToolCallSchema,
  ExecutionResult: ExecutionResultSchema,
  OrlEventType: OrlEventTypeSchema,
  OrlEvent: OrlEventSchema,
  SubstrateConfig: SubstrateConfigSchema,
  HardwareConfig: HardwareConfigSchema,
  SemanticPhaseUtterance: SemanticPhaseUtteranceSchema,
  PolicyUpdate: PolicyUpdateSchema,
  Superposition: SuperpositionSchema,
  PhaseLock: PhaseLockSchema,
} as const;
