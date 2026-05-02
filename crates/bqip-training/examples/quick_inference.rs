// Quick Inference Demo
//
// Run: cargo run --example quick_inference --package bqip-training
//
// This loads a trained BQIP model and runs a forward pass.

use bqip_training::ConceptTokenCompiler;  // for encoding text if needed
use bqip_transformer::{HybridTransformer, LazyBqipMemory};
use bqip_core::REGISTER_BYTES;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔮 BQIP Inference Demo\n");

    // Load trained model
    let model_path = "bqip_solid_model.bin";
    println!("📂 Loading model from {}...", model_path);
    let model = HybridTransformer::load_weights(model_path)?;
    println!("   ✅ Loaded: vocab={}, d_model={}, max_context={}", 
        model.config().vocab_size,
        model.config().d_model,
        model.config().max_context);

    // Prepare a sequence of token ids (example)
    // For a real use case, you'd use ConceptTokenCompiler to encode text
    let tokens = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Run forward pass
    let mut memory = LazyBqipMemory::new();
    println!("\n🧠 Running forward pass on {} tokens...", tokens.len());
    let output = model.forward(&tokens, &mut memory)?;

    // Show last hidden state
    let last_hidden = output.hidden_states.last().unwrap();
    println!("   📊 Last hidden state (first 8 dims): {:.4?}", 
        &last_hidden[..8.min(last_hidden.len())]);

    // Compute logits: LM head linear layer
    let vocab_size = model.config.vocab_size;
    let mut logits = Vec::with_capacity(vocab_size);
    for token_id in 0..vocab_size {
        let row = model.lm_head.row(token_id);
        let mut dot = 0.0;
        for (i, w) in row.iter().enumerate() {
            dot += w * last_hidden[i];
        }
        logits.push(dot);
    }

    // Softmax
    let max_logit = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let exp: Vec<f32> = logits.iter().map(|&l| (l - max_logit).exp()).collect();
    let sum_exp: f32 = exp.iter().sum();
    let probs: Vec<f32> = exp.iter().map(|&e| e / sum_exp).collect();

    // Top-5 predictions
    let top_k = 5;
    let mut ranked: Vec<_> = probs.iter().enumerate().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n🏆 Top {} next-token predictions:", top_k);
    for (i, (token_id, &prob)) in ranked.iter().take(top_k).enumerate() {
        println!("   {}. token {}  prob={:.6}  logit={:.4}", 
            i+1, token_id, prob, logits[*token_id]);
    }

    // Show envelope prediction
    let (alpha, beta) = model.predict_envelope(last_hidden);
    println!("\n🌊 Predicted Phase Envelope:");
    println!("   α = {:.4}, β = {:.4}  (norm = {:.4})", 
        alpha, beta, (alpha*alpha + beta*beta).sqrt());

    Ok(())
}

    // Show envelope prediction
    let (alpha, beta) = model.predict_envelope(last_hidden);
    println!("\n🌊 Predicted Phase Envelope:");
    println!("   α = {:.4}, β = {:.4}  (norm = {:.4})", 
        alpha, beta, (alpha*alpha + beta*beta).sqrt());

    Ok(())
}
