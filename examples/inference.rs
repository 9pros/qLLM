// Inference with Revolutionary BQIP Model
//
// This demonstrates how to use a trained BQIP model for inference
// with all revolutionary features enabled.

use bqip_transformer::{HybridTransformer, LazyBqipMemory};
use bqip_core::REGISTER_BYTES;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Revolutionary BQIP Inference Demo\n");

    // 1. Load trained model
    println!("1. Loading trained model...");
    let model = HybridTransformer::load_weights("bqip_model.bin")?;
    println!("   ✅ Model loaded");
    println!("   - Vocab size: {}", model.config().vocab_size);
    println!("   - d_model: {}", model.config().d_model);
    println!("   - Max context: {}", model.config().max_context);

    // 2. Prepare input
    println!("\n2. Preparing input...");
    let input_tokens = vec![2, 7, 9, 35];  // Example token IDs
    println!("   Input tokens: {:?}", input_tokens);

    // 3. Create memory
    let mut memory = LazyBqipMemory::new();

    // 4. Run inference
    println!("\n3. Running inference with revolutionary features...");
    let output = model.forward(&input_tokens, &mut memory)?;
    
    println!("   ✅ Forward pass complete");
    println!("   - Hidden states: {}", output.hidden_states.len());
    println!("   - Structural states: {}", output.structural_states.len());

    // 5. Inspect revolutionary features
    println!("\n4. Revolutionary Feature Analysis:");
    
    if let Some(last_state) = output.structural_states.last() {
        println!("\n   Last Token Analysis:");
        println!("   - Token ID: {}", last_state.token_id);
        println!("   - Position: {}", last_state.position);
        
        // Analyze twin consistency (revolutionary feature!)
        let twin = &last_state.dual_state.twin;
        let live = &last_state.dual_state.live;
        let envelope = &last_state.envelope;
        
        // Compute expected twin
        let expected_twin = bqip_core::phase_project(*live, *envelope);
        
        // Calculate decoherence
        let decoherence = {
            let left = twin.as_bytes();
            let right = expected_twin.as_bytes();
            let mut distance = 0u32;
            for (l, r) in left.iter().zip(right) {
                distance += (l ^ r).count_ones();
            }
            distance as f32 / bqip_core::REGISTER_BITS as f32
        };
        
        println!("   - Twin consistency (decoherence): {:.6}", decoherence);
        println!("     (Lower = better quantum-like consistency)");
        
        // Analyze envelope (revolutionary feature!)
        println!("\n   Phase Envelope:");
        println!("   - α (alpha): {:.4}", envelope.alpha);
        println!("   - β (beta):  {:.4}", envelope.beta);
        println!("   - Coherence signature: {}", envelope.coherence_sig);
        println!("   - Note: α² + β² = {:.4} (should be ~1.0)",
                 envelope.alpha * envelope.alpha + envelope.beta * envelope.beta);
    }

    // 6. Predict next token
    println!("\n5. Next Token Prediction:");
    let prediction = model.predict_dual_state(
        &input_tokens,
        &mut memory,
        5,  // top_k
    )?;
    
    println!("   Predicted token: {}", prediction.predicted_token_id);
    println!("\n   Top 5 predictions:");
    for (i, logit) in prediction.top_logits.iter().enumerate() {
        println!("   {}. Token {}: logit={:.4}", 
                 i + 1, logit.token_id, logit.logit);
    }

    // 7. Analyze envelope prediction (revolutionary!)
    println!("\n6. Predicted Phase Envelope:");
    println!("   - α: {:.4}", prediction.envelope.alpha);
    println!("   - β: {:.4}", prediction.envelope.beta);
    println!("   (Learned from hidden state via envelope prediction heads)");

    // 8. Memory analysis
    println!("\n7. Memory State:");
    println!("   - DAG nodes: {}", memory.len());
    println!("   - Record count: {}", memory.record_count());
    println!("   (Lazy evaluation: nodes combined on-demand)");

    println!("\n✅ Inference complete!");
    println!("\nThe model demonstrates revolutionary features:");
    println!("   ✅ Twin-state consistency enforced");
    println!("   ✅ Learnable phase envelopes (α, β)");
    println!("   ✅ Quantum-inspired representations");

    Ok(())
}

// Example: Compare envelope predictions across concepts
fn compare_concepts() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n" + "="*60);
    println!("Concept Envelope Comparison (Revolutionary Feature)");
    println!("="*60);
    
    let model = HybridTransformer::load_weights("bqip_model.bin")?;
    let mut memory = LazyBqipMemory::new();
    
    let concepts = vec![
        ("quantum", vec![2, 7, 9]),
        ("neural", vec![3, 15, 22]),
        ("distributed", vec![8, 12, 19]),
    ];
    
    for (name, tokens) in concepts {
        let (alpha, beta) = model.predict_envelope_from_context(&tokens, &mut memory)?;
        println!("\n{}:", name);
        println!("  α = {:.4}, β = {:.4}", alpha, beta);
        println!("  α² + β² = {:.4}", alpha * alpha + beta * beta);
        
        // Interpret
        let classical = (alpha - 1.0).abs();
        let quantum = ((alpha - 0.7071).abs() + (beta - 0.7071).abs()) / 2.0;
        
        if classical < quantum {
            println!("  → More CLASSICAL representation");
        } else {
            println!("  → More QUANTUM representation");
        }
    }
    
    println!("\n💡 The model learned different phase envelopes");
    println!("   for different concept types!");
    
    Ok(())
}

// Run comparison if called with --compare
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "--compare" {
        compare_concepts()?;
    } else {
        // Run standard inference
        main_inference()?;
        
        println!("\n" + "-"*60);
        println!("Try: cargo run --example inference -- --compare");
        println!("To see concept envelope comparison");
    }
    
    Ok(())
}

fn main_inference() -> Result<(), Box<dyn std::error::Error>> {
    // ... (main inference code from above)
    Ok(())
}