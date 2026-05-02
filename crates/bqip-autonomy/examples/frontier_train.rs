// Frontier BQIP Training - Continual, Self-Improving Mode
//
// Run: cargo run --example frontier_train --package bqip-autonomy
//
// This demonstrates frontier training capabilities:
// - Online continual learning (no fixed epochs)
// - Experience replay buffer with priority sampling
// - Curriculum stage progression (phase_delta increases)
// - Self-synthesis (model generates training data)
// - Live metrics every N steps

use bqip_autonomy::{FrontierTrainer, FrontierTrainConfig, LearningStepResult};
use bqip_training::ConceptTokenCompiler;
use bqip_transformer::{HybridConfig, HybridTransformer, MixerKind};
use bqip_core::{phase_project, REGISTER_BYTES};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌟 Frontier BQIP - Continual Learning Mode\n");

    // 1. Tokenizer (simple hash-based, no learned vocab)
    let compiler = ConceptTokenCompiler::new(Default::default())?;

    // 2. Model - moderate size for quick iteration
    let model_config = HybridConfig {
        vocab_size: 1024,
        d_model: 192,
        max_context: 64,
        mixer: MixerKind::GatedDeltaNet,
        num_heads: 6,
        phase_bucket_size: 64,
        structural_feedback: 0.03,
        ..HybridConfig::default()
    };
    let model = HybridTransformer::new(model_config, [99u8; REGISTER_BYTES])?;
    println!("🤖 Model initialized: d_model={}, heads={}", 
        model.config.d_model, model.config.num_heads);

    // 3. Frontier trainer config
    let mut fconfig = FrontierTrainConfig::default();
    fconfig.batch_size = 4;
    fconfig.replay_capacity = 5000;
    fconfig.self_synthesis_rate = 0.15;
    fconfig.curriculum_stage_intervals = vec![500, 2000, 5000, 10000];
    fconfig.twin_loss_weight = 0.1;
    fconfig.twin_perturbation_scale = 0.05;
    fconfig.graph_contrastive_weight = 0.1;  // keep on

    let mut trainer = FrontierTrainer::new(model, fconfig)?;

    // 4. Initial seed data - diverse concepts
    let seed_examples = vec![
        ("quantum", "quantum superposition entanglement"),
        ("neural", "neural network backpropagation attention"),
        ("learning", "reinforcement learning meta learning"),
        ("classical", "classical mechanics thermodynamics"),
        ("information", "information entropy mutual information"),
        ("quantum", "quantum tunneling coherence"),
        ("neural", "convolutional neural networks"),
        ("learning", "transfer learning domain adaptation"),
    ];

    println!("🌱 Seeding replay buffer with {} examples...", seed_examples.len());
    for (concept, text) in &seed_examples {
        let doc = compiler.compile(concept, text.as_bytes())?;
        let _ = trainer.learn_step(doc.tokens, Some(concept.to_string()));
    }

    // 5. Continual learning loop
    println!("\n▶️  Starting continual learning (10,000 steps)...");
    println!("   (Press Ctrl+C to stop)\n");

    let start = Instant::now();
    let mut last_print = start.elapsed().as_secs();
    let mut best_loss = f32::INFINITY;

    for step in 0..10_000 {
        // Determine source: replay buffer, self-synthesis, or seed
        let (tokens, label) = {
            if !trainer.synthesis_pool.is_empty() && step % 3 == 0 {
                // Use self-synthesized
                let idx = (step / 3) % trainer.synthesis_pool.len();
                (trainer.synthesis_pool[idx].clone(), Some("self-synthesized".to_string()))
            } else if !trainer.replay_buffer.buffer.is_empty() {
                // Replay from buffer
                let batch = trainer.replay_buffer.sample_batch(1);
                if let Some(exp) = batch.first() {
                    (exp.tokens.clone(), exp.concept_label.clone())
                } else {
                    fallback_example(step, &seed_examples, &compiler)?
                }
            } else {
                fallback_example(step, &seed_examples, &compiler)?
            }
        };

        // Learning step
        let result = trainer.learn_step(tokens, label)?;

        // Track best loss
        if result.loss < best_loss {
            best_loss = result.loss;
        }

        // Periodic reporting
        let now = start.elapsed().as_secs();
        if now > last_print && (now - last_print) >= 5 {
            let stats = trainer.stats();
            println!("  step {:>5} | loss={:.4} | best={:.4} | buf={:>4} | synth={:>3} | stage={}",
                result.step, result.loss, best_loss, 
                stats.buffer_size, stats.synthesis_pool_size, stats.curriculum_stage);
            last_print = now;
        }

        // Optional: Save checkpoint every 2000 steps
        if step > 0 && step % 2000 == 0 {
            let ckpt_path = format!("frontier_checkpoint_{}.bin", step);
            trainer.model().save_weights(&ckpt_path)?;
            println!("💾 Checkpoint saved: {}", ckpt_path);
        }
    }

    let elapsed = start.elapsed();
    println!("\n✅ Frontier training complete!");
    println!("   Steps: {}", trainer.stats().step);
    println!("   Best loss: {:.4}", best_loss);
    println!("   Final buffer size: {}", trainer.replay_buffer.buffer.len());
    println!("   Synthesis pool: {} sequences", trainer.synthesis_pool.len());
    println!("   Curriculum stage: {}", trainer.curriculum_stage);
    println!("   Time: {:.1}s ({:.2} steps/s)", 
        elapsed.as_secs_f32(), 
        trainer.stats().step as f32 / elapsed.as_secs_f32());

    // Save final model
    let final_path = "bqip_frontier_model.bin";
    trainer.model().save_weights(final_path)?;
    println!("💾 Final model saved to {}", final_path);

    Ok(())
}

fn fallback_example(
    step: usize,
    seeds: &[(&str, &str)],
    compiler: &ConceptTokenCompiler,
) -> Result<(Vec<u32>, Option<String>), Box<dyn std::error::Error>> {
    let (concept, text) = seeds[step % seeds.len()];
    let doc = compiler.compile(concept, text.as_bytes())?;
    Ok((doc.tokens, Some(concept.to_string())))
}
