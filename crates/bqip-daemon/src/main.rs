use std::path::PathBuf;

use bqip_core::REGISTER_BYTES;
use bqip_daemon::{BqipDaemon, DaemonConfig, EventSource, Goal, SenseEvent};
use bqip_transformer::HybridConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let memory_log_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("bqip-daemon.replay"));
    let graph_log_path = memory_log_path.with_extension("graph");
    let config = DaemonConfig {
        model: HybridConfig {
            vocab_size: 512,
            d_model: 64,
            max_context: 128,
            ..HybridConfig::default()
        },
        top_k: 4,
        compaction_interval_records: 12,
        graph_projection_limit: 4,
        graph_projection_token_budget: 8,
        node_public_key: [77u8; REGISTER_BYTES],
        memory_log_path,
        graph_log_path,
    };
    let mut daemon = BqipDaemon::open(config)?;
    daemon.add_goal(Goal::new(1, "stabilize predicted token stream", 378, 0.8));

    let events = [
        SenseEvent::new(EventSource::User, vec![12, 14, 18, 71], 1),
        SenseEvent::new(EventSource::Tool, vec![73, 75, 18, 14], 2),
        SenseEvent::new(EventSource::System, vec![12, 14, 18, 71, 73, 75], 3),
    ];

    for event in events {
        let tick = daemon.tick(event)?;
        println!(
            "tick={} concept_matches={} predicted_token={} reward={:.6} action={:?}",
            tick.tick_index,
            tick.graph_projection.matches.len(),
            tick.prediction.predicted_token_id,
            tick.outcome.reward,
            tick.action
        );
    }
    println!("memory_records={}", daemon.memory_record_count());
    println!("memory_nodes={}", daemon.memory_node_count());
    println!("graph_blocks={}", daemon.graph_block_count());
    println!("graph_concepts={}", daemon.graph_concept_count());
    println!("cumulative_reward={:.6}", daemon.cumulative_reward());
    println!("checkpoint={}", daemon.checkpoint_path().display());

    Ok(())
}
