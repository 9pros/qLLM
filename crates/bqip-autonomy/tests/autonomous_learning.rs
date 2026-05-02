use bqip_autonomy::{
    AutonomousLearner, AutonomousLearningConfig, CollectiveInferenceSuperposition,
    ExplorationCandidate, ExplorationKind, InferenceObservation, ValueFitnessWeights,
};
use bqip_core::{PhaseEnvelope, Register, REGISTER_BYTES};
use bqip_multimodal::{ground_frame, PixelFormat, VisualFrame};
use bqip_training::{
    ConceptTokenCompiler, ConceptTokenizerConfig, MetricsLedger, SplitRatios, TrainConfig,
    TrainableScope,
};
use bqip_transformer::{HybridConfig, HybridTransformer, MixerKind};

fn compiler() -> ConceptTokenCompiler {
    ConceptTokenCompiler::new(ConceptTokenizerConfig {
        vocab_size: 64,
        max_tokens: 48,
        ngram_min: 1,
        ngram_max: 2,
        include_byte_tokens: true,
    })
    .expect("compiler config validates")
}

fn learner() -> AutonomousLearner {
    AutonomousLearner::new(
        compiler(),
        AutonomousLearningConfig {
            vocab_size: 64,
            max_context: 8,
            split_ratios: SplitRatios {
                train: 0.75,
                validation: 0.25,
                test: 0.0,
            },
            fitness_weights: ValueFitnessWeights::default(),
        },
    )
    .expect("learner config validates")
}

fn model() -> HybridTransformer {
    HybridTransformer::new(
        HybridConfig {
            vocab_size: 64,
            d_model: 16,
            max_context: 8,
            mixer: MixerKind::DeltaNet,
            num_heads: 4,
            phase_bucket_size: 8,
            structural_feedback: 0.01,
            ..HybridConfig::default()
        },
        [12u8; REGISTER_BYTES],
    )
    .expect("model validates")
}

fn envelope() -> PhaseEnvelope {
    PhaseEnvelope::balanced(0xabc0_1111)
}

fn inference(provider: &str, response: &str, confidence: f32) -> InferenceObservation {
    InferenceObservation::new(
        provider,
        "frontier-reasoner",
        "collective inference",
        "derive the best exploration policy",
        response,
        confidence,
        320,
        1200,
        10_000,
        envelope(),
    )
    .expect("inference observation validates")
}

fn visual_grounding() -> bqip_multimodal::MultimodalGrounding {
    let mut pixels = Vec::with_capacity(6 * 6);
    for y in 0..6 {
        for x in 0..6 {
            pixels.push(if x == y { 240 } else { 16 });
        }
    }
    let frame = VisualFrame::new(6, 6, PixelFormat::Gray8, 11, pixels).expect("frame validates");
    ground_frame("diagonal visual policy", &frame, envelope(), &compiler())
        .expect("visual grounding compiles")
}

#[test]
fn collective_inference_superposition_is_order_independent() {
    let left = inference("model-a", "rank high novelty low cost endpoints", 0.82);
    let right = inference(
        "model-b",
        "prefer high novelty and low cost endpoints",
        0.78,
    );
    let first = CollectiveInferenceSuperposition::from_observations(
        "collective inference",
        &[left.clone(), right.clone()],
        envelope(),
    )
    .expect("collective state builds");
    let second = CollectiveInferenceSuperposition::from_observations(
        "collective inference",
        &[right, left],
        envelope(),
    )
    .expect("collective state is order independent");

    assert_eq!(first.source_hash, second.source_hash);
    assert_eq!(first.state, second.state);
    assert_eq!(first.provider_count, 2);
    assert!(first.consensus.is_finite());
    assert!(first.conflict.is_finite());
}

#[test]
fn autonomous_exploration_ranks_value_novelty_and_cost() {
    let learner = learner();
    let basis = Register::deterministic(b"known memory basis");
    let high_value = ExplorationCandidate {
        id: 1,
        concept_label: "valuable endpoint".to_string(),
        kind: ExplorationKind::Endpoint {
            url: "https://api.example/live".to_string(),
        },
        expected_value: 0.9,
        novelty_state: Register::all_ones(),
        confidence_prior: 0.82,
        estimated_cost_microunits: 100,
        estimated_latency_ms: 100,
    };
    let low_value = ExplorationCandidate {
        id: 2,
        concept_label: "expensive endpoint".to_string(),
        kind: ExplorationKind::Endpoint {
            url: "https://slow.example/live".to_string(),
        },
        expected_value: 0.3,
        novelty_state: basis,
        confidence_prior: 0.4,
        estimated_cost_microunits: 5_000_000,
        estimated_latency_ms: 20_000,
    };

    let ranked = learner
        .rank_exploration(&[low_value, high_value], basis)
        .expect("ranking succeeds");
    assert_eq!(ranked[0].candidate.id, 1);
    assert!(ranked[0].score.total > ranked[1].score.total);
}

#[test]
fn flash_learning_consumes_inference_and_multimodal_groundings() {
    let learner = learner();
    let observations = vec![
        inference(
            "model-a",
            "flash learning should prioritize fresh grounded inference traces",
            0.8,
        ),
        inference(
            "model-b",
            "grounded inference traces become compact training windows",
            0.76,
        ),
    ];
    let multimodal = vec![visual_grounding()];
    let corpus = learner
        .build_flash_corpus(&observations, &multimodal)
        .expect("flash corpus builds");
    let mut ledger = MetricsLedger::new();
    let trained = learner
        .flash_learn(
            model(),
            &corpus,
                TrainConfig {
                    epochs: 2,
                    batch_size: 2,
                    learning_rate: 0.08,
                    max_grad_norm: 1.5,
                    trainable_scope: TrainableScope::LmHead,
                    twin_loss_weight: 0.05,
                    phase_delta_increment: 0,
                    twin_perturbation_scale: 0.02,
                    graph_contrastive_weight: 0.1,
                    ..TrainConfig::default()
                },
            &mut ledger,
            b"flash split",
        )
        .expect("flash learning completes");

    assert!(trained.report.train.final_loss.is_finite());
    assert!(!ledger.is_empty());
    ledger.validate().expect("metrics ledger validates");
}
