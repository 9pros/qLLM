// Quick Start: Training BQIP with Revolutionary Features
//
// Run with: cargo run --example train_revolutionary --package bqip-training
//
// This demonstrates the NEW training capabilities:
// 1. Twin-state consistency regularization
// 2. Learnable phase envelopes (alpha, beta)
// 3. Coherence-signature curriculum
// 4. Knowledge graph contrastive pre-training

use bqip_training::{ConceptTokenCompiler, ConceptTokenizerConfig, HybridTrainer, 
                     TrainConfig, TrainingCorpus, TrainableScope};
use bqip_core::REGISTER_BYTES;
use bqip_transformer::{HybridConfig, HybridTransformer, MixerKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Revolutionary BQIP Training - Solid Model\n");

    // 1. Setup Tokenizer - vocab must match model
    let compiler = ConceptTokenCompiler::new(ConceptTokenizerConfig {
        vocab_size: 512,      // Must match model.vocab_size
        max_tokens: 64,       // Reasonable
        ngram_min: 1,
        ngram_max: 3,
        include_byte_tokens: true,
    })?;

    // 2. Model Configuration - Lean but capable
    let model_config = HybridConfig {
        vocab_size: 512,      // Smaller vocab
        d_model: 128,         // Smaller hidden dim
        max_context: 32,      // Much shorter context = faster
        mixer: MixerKind::GatedDeltaNet,
        num_heads: 4,         // Fewer heads
        phase_bucket_size: 32,
        structural_feedback: 0.03,
        ..HybridConfig::default()
    };

    // 3. Create Substantial Training Data - Multiple concepts
    let docs = vec![
        // Quantum physics concept cluster
        compiler.compile("quantum", b"quantum superposition")?,
        compiler.compile("quantum", b"quantum entanglement")?,
        compiler.compile("quantum", b"quantum tunneling")?,
        
        // Neural network concept cluster
        compiler.compile("neural", b"neural networks")?,
        compiler.compile("neural", b"attention mechanisms")?,
        compiler.compile("neural", b"transformer architectures")?,
        
        // Learning theory concept cluster
        compiler.compile("learning", b"supervised learning")?,
        compiler.compile("learning", b"reinforcement learning")?,
        compiler.compile("learning", b"transfer learning")?,
    ];

    println!("📚 Training corpus: {} documents, {} unique concepts", 
        docs.len(),
        docs.iter().map(|d| d.concept_label.as_str()).collect::<std::collections::HashSet<_>>().len()
    );

    let corpus = TrainingCorpus::from_tokenized_documents(
        compiler.config().vocab_size, 32, &docs
    )?;

    // 4. Fast & Solid Revolutionary Training Configuration
    let train_config = TrainConfig {
        epochs: 10,                     // Faster - still substantial
        batch_size: 4,                  // Small batch for stability
        learning_rate: 0.08,            // Standard BQIP LR
        weight_decay: 0.01,             // Prevent overfitting
        max_grad_norm: 1.5,             // Gradient clipping
        trainable_scope: TrainableScope::LmHead,
        
        // 🌟 Twin-State Consistency (active)
        twin_loss_weight: 0.1,          // Enforces quantum-like structure
        twin_perturbation_scale: 0.05,  // Noise magnitude
        
        // 🌟 Coherence Curriculum (gradual relaxation)
        phase_delta_increment: 1,       // Allow cross-concept mixing over time
        
        // 🌟 Graph Contrastive - DISABLED for speed (set >0 to enable)
        graph_contrastive_weight: 0.0,  // Set 0.1-0.15 for semantic learning (slow)
        
        // Finite difference settings
        finite_difference_epsilon: 1.0e-3,
    };

    // 5. Initialize Model
    println!("🏗️  Initializing model (d_model={}, layers=1, heads={})", 
        model_config.d_model, model_config.num_heads);
    let model = HybridTransformer::new(
        model_config,
        [42u8; REGISTER_BYTES],
    )?;

    // 6. Train!
    println!("🎯 Starting training ({} epochs)...\n", train_config.epochs);
    let trainer = HybridTrainer::new(model, train_config)?;
    let trained = trainer.train(&corpus)?;

    // 7. Results & Analysis
    println!("\n📊 Training Results:");
    println!("   Loss: {:.4} → {:.4} (Δ {:.4})", 
        trained.report.initial_loss, 
        trained.report.final_loss,
        trained.report.initial_loss - trained.report.final_loss);
    println!("   Accuracy: {:.2}% → {:.2}%", 
        trained.report.initial_accuracy * 100.0,
        trained.report.final_accuracy * 100.0);
    
    println!("\n📈 Epoch-by-Epoch:");
    println!("   {:>6} {:>12} {:>10} {:>12} {:>12} {:>12}", 
        "Epoch", "Loss", "Acc %", "Decoherence", "α_error", "β_error");
    for (i, epoch) in trained.report.epochs.iter().enumerate() {
        println!("   {:>5}/{:>5} {:>12.4} {:>10.2}% {:>12.4} {:>12.4} {:>12.4}",
            i + 1,
            trained.report.epochs.len(),
            epoch.mean_loss,
            epoch.accuracy * 100.0,
            epoch.mean_decoherence,
            epoch.mean_envelope_alpha_error,
            epoch.mean_envelope_beta_error);
    }

    // 8. Save Model
    let model_path = "bqip_solid_model.bin";
    trained.model.save_weights(model_path)?;
    println!("\n✅ Model saved to {}", model_path);

    // 9. Quick validation
    println!("\n🔍 Validating saved model...");
    let loaded = HybridTransformer::load_weights(model_path)?;
    println!("   ✅ Model loads successfully");
    println!("   📐 Config: d_model={}, vocab={}, max_context={}", 
        loaded.config().d_model,
        loaded.config().vocab_size,
        loaded.config().max_context);

    Ok(())
}