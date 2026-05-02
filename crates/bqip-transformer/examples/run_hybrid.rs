use bqip_core::REGISTER_BYTES;
use bqip_transformer::{AppendLogLazyBqipMemory, HybridConfig, HybridTransformer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = HybridConfig {
        vocab_size: 512,
        d_model: 64,
        max_context: 128,
        ..HybridConfig::default()
    };
    let model = HybridTransformer::new(config, [42u8; REGISTER_BYTES])?;
    let weights_path = std::env::temp_dir().join("bqip-hybrid-example.weights");
    let memory_path = std::env::temp_dir().join("bqip-hybrid-example.replay");
    model.save_weights(&weights_path)?;

    let loaded = HybridTransformer::load_weights(&weights_path)?;
    let mut memory = AppendLogLazyBqipMemory::open(&memory_path)?;
    let output = loaded.forward_logged(&[12, 14, 18, 71, 73, 75, 18, 14], &mut memory)?;
    let prediction =
        loaded.predict_dual_state_logged(&[12, 14, 18, 71, 73, 75, 18, 14], &mut memory, 3)?;
    let prediction_node = prediction.commit(&mut memory, loaded.config().phase_delta)?;

    println!("hidden_states={}", output.hidden_states.len());
    println!("structural_states={}", output.structural_states.len());
    println!("predicted_token={}", prediction.predicted_token_id);
    println!("prediction_node={}", prediction_node.index());
    println!("prediction_top_logit={}", prediction.top_logits[0].logit);
    println!("append_log_records={}", memory.record_count());
    println!("replayed_memory_nodes={}", memory.len());
    memory.compact()?;
    println!("checkpoint={}", memory.checkpoint_path().display());
    if let Some(last) = output.structural_states.last() {
        println!("last_memory_node={}", last.memory_node.index());
        println!("last_coherence_sig={}", last.envelope.coherence_sig);
    }

    std::fs::remove_file(weights_path)?;
    std::fs::remove_file(memory_path)?;
    std::fs::remove_file(memory.checkpoint_path())?;
    Ok(())
}
