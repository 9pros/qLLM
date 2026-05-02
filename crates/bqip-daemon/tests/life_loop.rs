use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bqip_core::{InterfaceKind, RegisterLane, REGISTER_BYTES};
use bqip_daemon::{
    prediction_confidence, Action, BqipDaemon, DaemonConfig, DaemonError, EventSource, Goal,
    SenseEvent,
};
use bqip_knowledge_graph::{EndpointKind, GraphDelta};
use bqip_transformer::HybridConfig;

fn temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bqip-daemon-{label}-{}-{nanos}.replay",
        std::process::id()
    ))
}

fn config(path: PathBuf) -> DaemonConfig {
    let graph_log_path = path.with_extension("graph");
    DaemonConfig {
        model: HybridConfig {
            vocab_size: 256,
            d_model: 32,
            max_context: 32,
            num_heads: 4,
            ..HybridConfig::default()
        },
        top_k: 4,
        compaction_interval_records: 8,
        graph_projection_limit: 4,
        graph_projection_token_budget: 8,
        node_public_key: [31u8; REGISTER_BYTES],
        memory_log_path: path,
        graph_log_path,
    }
}

fn cleanup(config: &DaemonConfig) {
    let _ = fs::remove_file(&config.memory_log_path);
    let _ = fs::remove_file(config.memory_log_path.with_extension("checkpoint"));
    let _ = fs::remove_file(&config.graph_log_path);
}

#[test]
fn daemon_tick_commits_prediction_and_scores_outcome() {
    let path = temp_path("tick");
    let config = config(path);
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();
    let tick = daemon
        .tick(SenseEvent::new(EventSource::User, vec![3, 4, 5, 19], 100))
        .unwrap();

    assert_eq!(tick.tick_index, 0);
    assert_eq!(
        tick.committed_prediction_node.index(),
        daemon.memory_node_count() - 1
    );
    assert_eq!(
        daemon.memory_record_count(),
        tick.model_context.len() as u64 + 1
    );
    assert_eq!(daemon.tick_index(), 1);
    assert!(prediction_confidence(&tick.prediction).is_finite());
    assert!(tick.outcome.reward.is_finite());
    assert_eq!(daemon.graph_block_count(), 1);
    assert_eq!(daemon.graph_concept_count(), 1);
    assert_eq!(daemon.graph_log_record_count(), 2);
    assert_eq!(tick.graph_notifications.len(), 2);
    assert_eq!(daemon.pending_graph_notification_count().unwrap(), 0);
    cleanup(&config);
}

#[test]
fn daemon_goal_satisfaction_updates_on_match() {
    let path = temp_path("goal");
    let config = config(path);
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();
    let first_tick = daemon
        .tick(SenseEvent::new(EventSource::User, vec![7, 8, 9], 101))
        .unwrap();
    let predicted = first_tick.prediction.predicted_token_id;
    daemon.add_goal(Goal::new(1, "match current predictor", predicted, 1.0));

    let second_tick = daemon
        .tick(SenseEvent::new(EventSource::User, vec![7, 8, 9], 102))
        .unwrap();

    assert_eq!(
        second_tick.active_goal.as_ref().unwrap().target_token,
        predicted
    );
    assert!(second_tick.outcome.prediction_matched_goal);
    assert!(daemon.goals()[0].satisfaction > 0.0);
    cleanup(&config);
}

#[test]
fn daemon_compacts_on_record_interval() {
    let path = temp_path("compact");
    let mut config = config(path);
    config.compaction_interval_records = 6;
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();

    let first = daemon
        .tick(SenseEvent::new(EventSource::User, vec![1, 2, 3], 1))
        .unwrap();

    assert!(first.outcome.compacted);
    assert!(daemon.checkpoint_path().exists());
    cleanup(&config);
}

#[test]
fn daemon_reopens_existing_replay_memory() {
    let path = temp_path("reopen");
    let config = config(path);
    {
        let mut daemon = BqipDaemon::open(config.clone()).unwrap();
        daemon
            .tick(SenseEvent::new(EventSource::User, vec![10, 11, 12], 1))
            .unwrap();
    }

    let reopened = BqipDaemon::open(config.clone()).unwrap();

    assert!(reopened.memory_record_count() >= 4);
    assert!(reopened.memory_node_count() >= 4);
    assert_eq!(reopened.graph_block_count(), 1);
    assert_eq!(reopened.graph_concept_count(), 1);
    assert_eq!(reopened.graph_log_record_count(), 2);
    cleanup(&config);
}

#[test]
fn daemon_rejects_empty_events() {
    let path = temp_path("empty");
    let config = config(path);
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();

    assert!(matches!(
        daemon.tick(SenseEvent::new(EventSource::User, Vec::new(), 1)),
        Err(DaemonError::EmptyEvent)
    ));
    assert_eq!(daemon.memory_record_count(), 0);
    assert_eq!(daemon.graph_log_record_count(), 0);
    cleanup(&config);
}

#[test]
fn low_confidence_action_is_still_explicit() {
    let path = temp_path("action");
    let config = config(path);
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();
    let tick = daemon
        .tick(SenseEvent::new(EventSource::System, vec![2, 3, 5, 8], 1))
        .unwrap();

    match tick.action {
        Action::EmitPrediction { confidence, .. } => assert!(confidence >= 0.35),
        Action::StabilizeMemory { record_count } => assert!(record_count > 0),
        Action::Hold { ref reason } => assert!(!reason.is_empty()),
    }
    cleanup(&config);
}

#[test]
fn endpoint_observation_updates_reactive_graph_and_projection_context() {
    let path = temp_path("graph");
    let config = config(path);
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();
    let event = SenseEvent::new(EventSource::Tool, vec![21, 34, 55], 1_000)
        .with_endpoint_observation(
            "Live Public API",
            RegisterLane::PublicApi,
            InterfaceKind::Application,
            EndpointKind::PublicApi,
            42,
            b"https://api.example/v1/search".to_vec(),
            br#"{"capability":"concept projection","status":"live"}"#.to_vec(),
            60_000,
        );

    let original_len = event.tokens.len();
    let tick = daemon.tick(event).unwrap();

    assert_eq!(daemon.graph_block_count(), 1);
    assert_eq!(daemon.graph_concept_count(), 1);
    assert_eq!(daemon.graph_log_record_count(), 2);
    assert!(daemon.concept_id_for_label("live public api").is_some());
    assert!(tick.model_context.len() > original_len);
    assert_eq!(tick.graph_projection.matches[0].label, "Live Public API");
    assert_eq!(tick.graph_notifications.len(), 2);
    assert!(matches!(
        tick.graph_notifications[0].delta,
        GraphDelta::MemoryBlockInserted { .. }
    ));
    assert!(matches!(
        tick.graph_notifications[1].delta,
        GraphDelta::ConceptUpserted { .. }
    ));
    cleanup(&config);
}

#[test]
fn daemon_rejects_empty_observation_payload_before_graph_mutation() {
    let path = temp_path("empty-payload");
    let config = config(path);
    let mut daemon = BqipDaemon::open(config.clone()).unwrap();
    let event = SenseEvent::new(EventSource::Tool, vec![1], 1).with_endpoint_observation(
        "Empty Payload",
        RegisterLane::PublicApi,
        InterfaceKind::Application,
        EndpointKind::PublicApi,
        1,
        b"https://api.example/empty".to_vec(),
        Vec::new(),
        60_000,
    );

    assert!(matches!(
        daemon.tick(event),
        Err(DaemonError::EmptyObservationPayload)
    ));
    assert_eq!(daemon.graph_block_count(), 0);
    assert_eq!(daemon.graph_log_record_count(), 0);
    cleanup(&config);
}
