# Revolutionary BQIP Training Guide

This guide covers the **new revolutionary training features** added to BQIP.

## 🌟 New Training Features

### 1. Twin-State Consistency Regularization

The model now enforces that `twin = phase_project(live, envelope)` through a **decoherence loss**.

**How it works:**
- During training, the live state is perturbed with small noise
- The model is penalized if the twin projection drifts from the expected relationship
- This creates a smooth, differentiable regularizer
- Encourages the model to maintain quantum-like unitary evolution

**Configuration:**
```rust
TrainConfig {
    twin_loss_weight: 0.1,          // Strength (0.0 = disabled)
    twin_perturbation_scale: 0.05,  // Noise scale (0.0-0.1 typical)
    ..
}
```

**Monitoring:**
- `EpochMetrics.mean_decoherence`: Lower = better twin consistency
- Typical values: 0.001-0.05 (lower is better)

---

### 2. Learnable Phase Envelopes (α, β)

The phase envelope parameters α and β are now **trainable**!

**How it works:**
- Added `envelope_alpha` and `envelope_beta` as 1×d_model matrices
- These are linear heads that predict α, β from hidden states
- Output is normalized to satisfy α² + β² = 1
- Model learns **optimal quantum basis** for each concept

**Why this matters:**
- Different concepts may need different phase relationships
- Some concepts are "classical" (α≈1, β≈0)
- Others are "quantum" (α≈β≈0.707)
- The model discovers this automatically!

**Configuration:**
```rust
// No extra config needed! Automatically added to ModelWeights
// Trained via MSE loss against target envelope
```

**Monitoring:**
- `EpochMetrics.mean_envelope_alpha_error`: How well α is predicted
- `EpochMetrics.mean_envelope_beta_error`: How well β is predicted
- Typical values: 0.01-0.1 (lower is better)

---

### 3. Coherence-Signature Curriculum

Gradually relaxes the phase compatibility constraint during training.

**How it works:**
- `phase_delta` controls how strictly coherence signatures must match
- Start with small delta (strict matching)
- Increment each epoch: `phase_delta += phase_delta_increment`
- Allows more cross-signature combination over time
- Encourages hierarchical abstraction

**Configuration:**
```rust
TrainConfig {
    phase_delta_increment: 1,  // Increase per epoch (0 = disabled)
    ..
}
```

**Why this matters:**
- Early training: Focus on within-signature patterns
- Later training: Allow cross-signature generalization
- Similar to curriculum learning in NLP

---

### 4. Knowledge Graph Contrastive Pre-training

Pulls related concepts closer in representation space.

**How it works:**
- Groups examples by concept label
- Encodes each example's hidden state
- Computes InfoNCE loss within each group
- Similar concepts → similar representations

**Configuration:**
```rust
TrainConfig {
    graph_contrastive_weight: 0.15,  // Strength (0.0 = disabled)
    ..
}
```

**Why this matters:**
- Learns semantic relationships without explicit supervision
- Improves few-shot generalization
- Complements standard language modeling loss

---

## 🚀 Quick Start

### Basic Training

```rust
use bqip_training::{ConceptTokenCompiler, ConceptTokenizerConfig, 
                     HybridTransformer, TrainConfig, TrainingCorpus, 
                     HybridTrainer};
use bqip_core::REGISTER_BYTES;

// 1. Setup tokenizer
let compiler = ConceptTokenCompiler::new(ConceptTokenizerConfig {
    vocab_size: 512,
    max_tokens: 96,
    ngram_min: 1,
    ngram_max: 3,
    include_byte_tokens: true,
})?;

// 2. Create training data
let docs = vec![
    compiler.compile("quantum", b"quantum superposition")?,
    compiler.compile("neural", b"neural networks")?,
];
let corpus = TrainingCorpus::from_tokenized_documents(
    compiler.config().vocab_size, 50, &docs
)?;

// 3. Configure revolutionary training
let train_config = TrainConfig {
    epochs: 50,
    batch_size: 4,
    learning_rate: 0.08,
    max_grad_norm: 1.5,
    trainable_scope: TrainableScope::LmHead,
    
    // 🌟 NEW FEATURES
    twin_loss_weight: 0.1,          // Twin consistency
    twin_perturbation_scale: 0.05,  // Perturbation noise
    phase_delta_increment: 1,       // Coherence curriculum
    graph_contrastive_weight: 0.15, // Graph contrastive
    
    ..TrainConfig::default()
};

// 4. Initialize model
let model = HybridTransformer::new(
    train_config.model.clone(),
    [42u8; REGISTER_BYTES],
)?;

// 5. Train!
let trainer = HybridTrainer::new(model, train_config)?;
let trained = trainer.train(&corpus)?;

// 6. Save
trained.model.save_weights("bqip_model.bin")?;
```

### Run Example

```bash
cargo run --example train_revolutionary
```

---

## 📊 Monitoring Revolutionary Metrics

The new `EpochMetrics` tracks revolutionary training progress:

```rust
pub struct EpochMetrics {
    pub epoch_index: usize,
    pub examples_seen: usize,
    pub mean_loss: f32,                    // Standard LM loss
    pub accuracy: f32,                     // Token accuracy
    pub mean_target_probability: f32,      // Target token prob
    pub gradient_norm: f32,                // Gradient magnitude
    
    // 🌟 NEW: Revolutionary Metrics
    pub mean_decoherence: f32,             // Twin consistency (lower=better)
    pub mean_envelope_alpha_error: f32,    // α prediction error
    pub mean_envelope_beta_error: f32,     // β prediction error
}
```

**Interpretation:**

| Metric | Good Value | Meaning |
|--------|-----------|---------|
| `mean_decoherence` | < 0.05 | Twin states well-aligned |
| `mean_envelope_alpha_error` | < 0.1 | α predictions accurate |
| `mean_envelope_beta_error` | < 0.1 | β predictions accurate |
| `mean_loss` | Decreasing | Learning progress |

---

## 🔧 Hyperparameter Tuning

### Twin-State Regularization

```rust
twin_loss_weight: 0.05-0.2    // Start low, increase if unstable
twin_perturbation_scale: 0.02-0.1  // Small noise for gradient
```

**Too high?** → Model focuses too much on consistency, LM loss stalls  
**Too low?** → Twin states drift, lose quantum structure  
**Just right?** → Both losses decrease together

### Coherence Curriculum

```rust
phase_delta_increment: 0-2  // 0 = disabled, 1 = gradual, 2 = fast
```

**0** → Strict signature matching throughout  
**1** → Gradual relaxation (recommended)  
**2+** → Fast relaxation, more cross-signature mixing

### Graph Contrastive

```rust
graph_contrastive_weight: 0.0-0.3
```

**0.0** → Disabled (standard LM only)  
**0.1-0.2** → Balanced with LM loss (recommended)  
**0.3+** → Strong semantic pull, may hurt perplexity

### Learnable Envelopes

No tuning needed! Automatically trained via MSE loss.

---

## 🎯 When to Use Each Feature

| Feature | Best For |
|---------|----------|
| **Twin Consistency** | All tasks - improves stability |
| **Learnable Envelopes** | Domain-specific concepts, transfer learning |
| **Coherence Curriculum** | Large datasets, hierarchical concepts |
| **Graph Contrastive** | Semantic tasks, few-shot learning |

**Recommended Starting Point:**
```rust
TrainConfig {
    twin_loss_weight: 0.1,
    twin_perturbation_scale: 0.05,
    phase_delta_increment: 1,
    graph_contrastive_weight: 0.15,
    ..
}
```

---

## 📈 Expected Results

With revolutionary features enabled:

- **Similar or better** perplexity vs. baseline
- **Improved stability** during training
- **Better generalization** to unseen concepts
- **Interpretable representations** (via learned α, β)
- **Faster convergence** on semantic tasks

**Trade-off:** Slightly slower per-epoch (extra forward passes for envelope prediction)

---

## 🔍 Debugging

### High Decoherence

If `mean_decoherence` > 0.1:
- Reduce `twin_loss_weight` (try 0.05)
- Reduce `twin_perturbation_scale` (try 0.02)
- Check learning rate (may be too high)

### High Envelope Error

If envelope errors > 0.2:
- Increase `graph_contrastive_weight` (helps concept separation)
- Reduce learning rate for envelope heads (via weight decay)
- More training data for concept

### Loss Not Decreasing

- Reduce `twin_loss_weight` (may be dominating)
- Set `phase_delta_increment = 0` (disable curriculum)
- Set `graph_contrastive_weight = 0.0` (disable contrastive)
- Gradually reintroduce features

---

## 🎓 Advanced: Custom Envelope Prediction

Override envelope prediction by subclassing:

```rust
impl HybridTransformer {
    pub fn predict_envelope(&self, hidden: &[f32]) -> (f32, f32) {
        // Default: linear projection + sigmoid + normalize
        // Override for custom behavior
    }
}
```

---

## 📚 References

- **Twin-State Consistency**: Inspired by quantum measurement theory
- **Learnable Envelopes**: Neural quantum state approaches
- **Coherence Curriculum**: Curriculum learning (Bengio et al.)
- **Graph Contrastive**: Contrastive learning (Chen et al.)

---

## 💡 Summary

The revolutionary training features transform BQIP from a standard transformer into a **quantum-inspired learning system** that:

1. Maintains quantum-like consistency (twin regularization)
2. Learns optimal representations (learnable envelopes)
3. Progressively generalizes (coherence curriculum)
4. Understands semantics (graph contrastive)

All features are **optional** and can be mixed/matched. Start with recommended settings and tune for your task!
