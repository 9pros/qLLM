use std::fs;

use bqip_core::{InterfaceKind, PhaseEnvelope, Register, RegisterLane, REGISTER_BYTES};
use bqip_knowledge_graph::{
    AppendOnlyGraphLog, BlockRange, EdgeKind, EndpointGrounding, EndpointKind, GraphDelta,
    GraphError, GraphLogEvent, GraphNodeRef, GraphObservationLog, GraphSubscriptionFilter,
    LiveKnowledgeGraph, MemoryBlock, ProjectionRequest, ProjectionWeights, ReactiveKnowledgeGraph,
    TypedEdge, WeightedBlockRef,
};

fn node_key() -> [u8; REGISTER_BYTES] {
    [19u8; REGISTER_BYTES]
}

fn envelope(sig: u64) -> PhaseEnvelope {
    PhaseEnvelope::live_trusted(sig)
}

fn grounding(address: &str, timestamp: u64) -> EndpointGrounding {
    EndpointGrounding::new(
        RegisterLane::GenericEndpoint,
        timestamp,
        address.as_bytes().to_vec(),
        InterfaceKind::Application,
        EndpointKind::Website,
        &node_key(),
        timestamp,
    )
    .expect("endpoint grounding should derive a register id")
}

fn block(address: &str, content: &[u8], sig: u64, timestamp: u64) -> MemoryBlock {
    MemoryBlock::from_content(
        grounding(address, timestamp),
        BlockRange::new(0, content.len() as u64).expect("range should match content length"),
        Some("/".to_string()),
        content,
        Register::deterministic(content),
        envelope(sig),
        86_400_000,
    )
    .expect("memory block should be valid")
}

#[test]
fn memory_block_identity_and_twin_projection_are_deterministic() {
    let first = block(
        "https://example.com/bqip",
        b"DeltaNet grounded concept memory block",
        0x10,
        1_000,
    );
    let second = block(
        "https://example.com/bqip",
        b"DeltaNet grounded concept memory block",
        0x10,
        1_000,
    );

    assert_eq!(first.id, second.id);
    assert_eq!(first.state, second.state);
    assert_eq!(
        first.state.twin,
        bqip_core::phase_project(first.state.live, first.envelope)
    );
}

#[test]
fn concept_layer_projects_query_against_grounded_blocks() {
    let mut graph = LiveKnowledgeGraph::new();
    let deltanet = block(
        "https://papers.example/deltanet",
        b"DeltaNet linear recurrent attention with online state updates",
        0x20,
        2_000,
    );
    let metal = block(
        "https://developer.example/metal",
        b"Metal compute command buffers and unified memory scheduling",
        0x20,
        2_100,
    );
    graph
        .insert_memory_block(deltanet.clone())
        .expect("insert block");
    graph
        .insert_memory_block(metal.clone())
        .expect("insert block");

    let deltanet_concept = graph
        .upsert_concept(
            "DeltaNet",
            envelope(0x20),
            vec![WeightedBlockRef::new(deltanet.id, 1.0).expect("valid weight")],
            vec![],
            2_200,
        )
        .expect("insert deltanet concept");
    graph
        .upsert_concept(
            "Metal Compute",
            envelope(0x20),
            vec![WeightedBlockRef::new(metal.id, 1.0).expect("valid weight")],
            vec![],
            2_200,
        )
        .expect("insert metal concept");

    let query_state = graph
        .concept(deltanet_concept)
        .expect("concept exists")
        .state
        .live;
    let projection = graph
        .project(&ProjectionRequest {
            query_label: "online recurrent attention".to_string(),
            query_state,
            reference_time_unix_millis: 2_300,
            limit: 2,
            weights: ProjectionWeights::default(),
        })
        .expect("projection should rank concepts");

    assert_eq!(projection.matches[0].concept_id, deltanet_concept);
    assert_eq!(projection.matches[0].grounding_blocks, vec![deltanet.id]);
    assert!(projection.matches[0].score > projection.matches[1].score);
}

#[test]
fn contradiction_edges_downweight_projection_without_destroying_grounding() {
    let mut graph = LiveKnowledgeGraph::new();
    let support = block(
        "https://docs.example/api",
        b"Public API supports graph projection",
        0x30,
        3_000,
    );
    let contradicting = block(
        "https://status.example/api",
        b"Public API endpoint retired",
        0x30,
        3_100,
    );
    graph
        .insert_memory_block(support.clone())
        .expect("insert support");
    graph
        .insert_memory_block(contradicting.clone())
        .expect("insert contradiction evidence");
    let concept = graph
        .upsert_concept(
            "Public API Projection",
            envelope(0x30),
            vec![WeightedBlockRef::new(support.id, 1.0).expect("valid weight")],
            vec![],
            3_200,
        )
        .expect("insert concept");
    let baseline_query = ProjectionRequest {
        query_label: "public api projection".to_string(),
        query_state: graph.concept(concept).expect("concept exists").state.live,
        reference_time_unix_millis: 3_300,
        limit: 1,
        weights: ProjectionWeights {
            semantic: 1.0,
            freshness: 0.0,
            confidence: 0.0,
            graph_support: 0.0,
            contradiction_penalty: 1.0,
        },
    };
    let baseline = graph.project(&baseline_query).expect("baseline projection");

    let contradiction_envelope =
        PhaseEnvelope::new(0x30, 0.0, 1.0, 1).expect("pure contradiction envelope is normalized");
    let edge = TypedEdge::new(
        GraphNodeRef::Concept(concept),
        GraphNodeRef::Block(contradicting.id),
        EdgeKind::Contradicts,
        1.0,
        vec![contradicting.id],
        contradiction_envelope,
        3_250,
    )
    .expect("contradiction edge should be valid");
    graph.insert_edge(edge).expect("insert contradiction edge");

    let after = graph
        .project(&baseline_query)
        .expect("contradictory projection");
    assert!(after.matches[0].score < baseline.matches[0].score);
    assert!(after.matches[0].grounding_blocks.contains(&support.id));
    assert!(after.matches[0]
        .grounding_blocks
        .contains(&contradicting.id));
}

#[test]
fn observation_log_replays_to_identical_graph_state() {
    let mut source_graph = LiveKnowledgeGraph::new();
    let memory_block = block(
        "https://source.example/live",
        b"live endpoint grounded concept layer",
        0x40,
        4_000,
    );
    let mut log = GraphObservationLog::new();

    source_graph
        .apply_event(GraphLogEvent::InsertMemoryBlock(memory_block.clone()))
        .expect("apply block");
    log.append(GraphLogEvent::InsertMemoryBlock(memory_block.clone()))
        .expect("append block");

    let concept_event = GraphLogEvent::UpsertConcept {
        label: "Live Endpoint".to_string(),
        envelope: envelope(0x40),
        block_members: vec![WeightedBlockRef::new(memory_block.id, 1.0).expect("valid weight")],
        concept_members: vec![],
        updated_at_unix_millis: 4_100,
    };
    source_graph
        .apply_event(concept_event.clone())
        .expect("apply concept");
    log.append(concept_event).expect("append concept");

    let replayed = log.replay().expect("replay graph");
    assert_eq!(source_graph.block_count(), replayed.block_count());
    assert_eq!(source_graph.concept_count(), replayed.concept_count());
    let concept_id = source_graph
        .concept_id_for_label("live endpoint")
        .expect("concept id exists");
    assert_eq!(
        source_graph
            .concept(concept_id)
            .expect("source concept")
            .state,
        replayed
            .concept(concept_id)
            .expect("replayed concept")
            .state
    );
}

#[test]
fn append_only_file_log_persists_records_and_rejects_corruption() {
    let path =
        std::env::temp_dir().join(format!("bqip-kg-{}.log", u64::from_le_bytes(*b"kgtest01")));
    let _ = fs::remove_file(&path);

    let memory_block = block(
        "https://persist.example/live",
        b"persistent graph observation",
        0x50,
        5_000,
    );
    {
        let mut file_log = AppendOnlyGraphLog::open(&path).expect("open append-only log");
        file_log
            .append(GraphLogEvent::InsertMemoryBlock(memory_block.clone()))
            .expect("append persistent block");
        assert_eq!(file_log.log().len(), 1);
    }
    let reopened = AppendOnlyGraphLog::open(&path).expect("reopen append-only log");
    assert_eq!(reopened.log().len(), 1);
    assert_eq!(
        reopened
            .replay()
            .expect("replay persisted log")
            .block_count(),
        1
    );

    let mut bytes = fs::read(&path).expect("read persistent log");
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&path, bytes).expect("write corrupted log");
    let error = AppendOnlyGraphLog::open(&path).expect_err("corruption should be rejected");
    assert!(matches!(
        error,
        GraphError::PersistentHashMismatch | GraphError::Codec(_)
    ));
    let _ = fs::remove_file(&path);
}

#[test]
fn invalid_membership_and_content_ranges_are_rejected() {
    let invalid_range = BlockRange::new(0, 8).expect("range is ordered");
    let content_error = MemoryBlock::from_content(
        grounding("https://bad.example/range", 6_000),
        invalid_range,
        None,
        b"short",
        Register::deterministic(b"short"),
        envelope(0x60),
        1_000,
    )
    .expect_err("content length mismatch must fail");
    assert!(matches!(
        content_error,
        GraphError::ContentLengthMismatch { .. }
    ));

    let mut graph = LiveKnowledgeGraph::new();
    let concept_error = graph
        .upsert_concept("empty", envelope(0x60), vec![], vec![], 6_100)
        .expect_err("empty concepts are not valid graph knowledge");
    assert!(matches!(concept_error, GraphError::EmptyConceptLayer));
}

#[test]
fn reactive_graph_subscriptions_filter_and_drain_deltas() {
    let memory_block = block(
        "https://reactive.example/live",
        b"reactive graph concept observation",
        0x70,
        7_000,
    );
    let mut reactive = ReactiveKnowledgeGraph::new();
    let all = reactive
        .subscribe(GraphSubscriptionFilter::all())
        .expect("subscribe to all graph deltas");
    let label = reactive
        .subscribe(GraphSubscriptionFilter::for_concept_label(
            "Reactive Concept",
        ))
        .expect("subscribe to concept label");

    reactive
        .apply_event(GraphLogEvent::InsertMemoryBlock(memory_block.clone()))
        .expect("apply block event");
    assert_eq!(reactive.pending_len(all).expect("pending all"), 1);
    assert_eq!(reactive.pending_len(label).expect("pending label"), 0);

    reactive
        .apply_event(GraphLogEvent::UpsertConcept {
            label: "Reactive Concept".to_string(),
            envelope: envelope(0x70),
            block_members: vec![WeightedBlockRef::new(memory_block.id, 1.0).expect("valid")],
            concept_members: vec![],
            updated_at_unix_millis: 7_100,
        })
        .expect("apply concept event");

    let all_deltas = reactive.drain(all).expect("drain all");
    let label_deltas = reactive.drain(label).expect("drain label");
    assert_eq!(all_deltas.len(), 2);
    assert_eq!(label_deltas.len(), 1);
    assert!(matches!(
        all_deltas[0].delta,
        GraphDelta::MemoryBlockInserted { .. }
    ));
    assert!(matches!(
        all_deltas[1].delta,
        GraphDelta::ConceptUpserted { .. }
    ));
    assert!(matches!(
        label_deltas[0].delta,
        GraphDelta::ConceptUpserted { .. }
    ));
    assert_eq!(reactive.pending_len(all).expect("pending all"), 0);
}
