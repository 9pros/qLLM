# BQIP AGI Architecture

This workspace contains the **BQIP (Bilinear Quantum-Inspired Processing)** framework - a dual-state, substrate-agnostic, environment-sensing super-intelligence architecture.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agent Layer (Hermes + DSVM)                  │
│  - Dual-state memory (SemanticPhaseUtterance)                   │
│  - Skill evolution with phase envelopes                         │
│  - Social learning via phase-tuple utterances                   │
└─────────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────────┐
│              Cognitive Orchestration (AGIR-Lambda)              │
│  - TypeScript DSL for dual-state cognition                      │
│  - Learning contracts → Curriculum plans                        │
│  - Tool calls → Execution results                               │
│  - ORL event logging for replay                                 │
└─────────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────────┐
│                 Substrate Layer (BQIP Rust)                     │
│  - Rotational, hyperfractal dual-state transformer              │
│  - TEPE (Twin-Encoded Phase Evolution) GA                       │
│  - Three-tier collapse/commit model                             │
│  - Environment sensing & resource adaptation                    │
└─────────────────────────────────────────────────────────────────┘
```

## Directory Structure

```
/workspace
├── crates/                      # Rust BQIP substrate
│   ├── bqip-core/               # Register, DualState, PhaseEnvelope primitives
│   ├── bqip-transformer/        # Rotational hyperfractal transformer
│   ├── bqip-evolver/            # TEPE genetic algorithm
│   ├── bqip-training/           # Finite-difference gradient training
│   ├── bqip-knowledge-graph/    # Content-addressed semantic graph
│   ├── bqip-chat/               # OpenAI-compatible API
│   ├── bqip-autonomy/           # Autonomous learning loops
│   ├── bqip-control/            # Conversation control
│   ├── bqip-daemon/             # Lifecycle management
│   ├── bqip-meta/               # Meta-optimization
│   └── bqip-multimodal/         # Multimodal analysis
│
├── packages/                    # TypeScript cognitive layer
│   ├── @repo/shared-schemas/    # Zod schemas (cross-layer contracts)
│   └── @repo/agir-lambda/       # AGIR-Lambda DSL primitives
│
└── examples/                    # Example usage
```

## Key Concepts

### Dual-State Registers
Every activation exists in two states:
- **Live**: Current operational state
- **Twin**: Phase-projected consistency constraint (`twin = phase_project(live, envelope)`)

### Phase Envelopes
Learnable parameters that control twin projection:
```rust
pub struct PhaseEnvelope {
    pub coherence_sig: u64,
    pub alpha: f32,        // Unit circle amplitude
    pub beta: f32,         // Unit circle amplitude  
    pub resuperposition_n: u32,
    pub hf_rotation: f32,  // High-frequency rotation (rotational transformer)
    pub ha_rotation: f32,  // Hyperangular rotation (multi-axis)
    pub phase_offset: f32, // Liquid time-constant offset
}
```

### TEPE (Twin-Encoded Phase Evolution)
Genetic algorithm operating on Ctwin genotypes:
- **Ctwin**: Twin genotype (envelope params, ha_vector, ORL backpointers)
- **Clive**: Live phenotype (decoded routing lanes, vFPGA mappings)
- **Decode**: `U_decode = R_HF * R_HA * O`

### Three-Tier Collapse Model
1. **Local twin commit**: Per Ctwin-Clive pair when stable
2. **Lane phase-band commit**: When lane population converges
3. **Global Diamond commit**: Cryptographically signed major commits

### AGIR-Lambda Primitives
```typescript
// Define substrate with environment sensing
defineSubstrate(config: SubstrateConfig)

// Initialize twin pool for skill
initTwinPool(skillId, { poolSize: 16, baseEnvelope })

// Create superposition and collapse
const sup = superpose(genomes)
await collapse(phenotype, threshold: 0.85)

// Decode/encode Ctwin ↔ Clive
const phenotype = await decode(genome, liveRegister)
const genome = await encode(phenotype)

// Phase operations
phaseLock(target, sources, strength)
entangle(a, b)
teleportPhase(source, sink)

// Learning contracts
const contract = createLearningContract(skillId, objective, criteria)
const plan = await generateCurriculumPlan(contract)
```

## Getting Started

### Rust Substrate
```bash
cd /workspace
cargo build --release
cargo test
```

### TypeScript Cognitive Layer
```bash
cd /workspace/packages/shared-schemas
npm install
npm run build

cd ../agir-lambda
npm install
npm run build
```

### Example Usage
```typescript
import agir from '@repo/agir-lambda';

// Initialize substrate
await agir.defineSubstrate({
  environment_id: crypto.randomUUID(),
  hardware: {
    cpu_cores: 8,
    gpu_available: true,
    system_memory_gb: 32,
  },
  twin_pool_budget: 64,
  phase_budget: 0.8,
  latency_budget_ms: 100,
});

// Create learning contract
const contract = agir.createLearningContract(
  'reasoning-v1',
  'Improve multi-step reasoning accuracy',
  ['accuracy > 0.9', 'coherence > 0.85']
);

// Generate and execute curriculum
const plan = await agir.generateCurriculumPlan(contract);
for (const step of plan.steps) {
  const result = await agir.executeTool({
    tool_id: 'bqip-train',
    method: step.action,
    arguments: step.parameters,
  });
  console.log('Step result:', result);
}
```

## Documentation

- [REVOLUTIONARY_TRAINING.md](./REVOLUTIONARY_TRAINING.md) - Training methodology
- [packages/shared-schemas/src/index.ts](./packages/shared-schemas/src/index.ts) - Schema definitions
- [packages/agir-lambda/src/index.ts](./packages/agir-lambda/src/index.ts) - AGIR-Lambda DSL

## License

MIT
