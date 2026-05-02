use bqip_control::{
    ControlAction, ControlError, ConversationMessage, ConversationRole,
    NaturalLanguageTrainingController, TrainingControlConfig,
};
use bqip_training::{TrainableScope, TrainingError};
use bqip_transformer::{HybridConfig, MixerKind};

const REGISTER_BYTES: usize = 32;

fn config() -> TrainingControlConfig {
    TrainingControlConfig {
        model: HybridConfig {
            vocab_size: 64,
            d_model: 16,
            max_context: 12,
            mixer: MixerKind::DeltaNet,
            num_heads: 4,
            phase_bucket_size: 8,
            structural_feedback: 0.01,
            ..HybridConfig::default()
        },
        tokenizer: bqip_training::ConceptTokenizerConfig {
            vocab_size: 64,
            max_tokens: 32,
            ngram_min: 1,
            ngram_max: 2,
            include_byte_tokens: true,
        },
        train: bqip_training::TrainConfig {
            epochs: 2,
            batch_size: 2,
            learning_rate: 0.08,
            max_grad_norm: 1.5,
            trainable_scope: TrainableScope::LmHead,
            twin_loss_weight: 0.05,
            phase_delta_increment: 0,
            twin_perturbation_scale: 0.02,
            graph_contrastive_weight: 0.1,
            ..bqip_training::TrainConfig::default()
        },
        split_ratios: bqip_training::SplitRatios {
            train: 0.75,
            validation: 0.25,
            test: 0.0,
        },
        node_public_key: [33u8; REGISTER_BYTES],
        allow_model_self_training: true,
    }
}

fn operator(content: &str, timestamp_ms: u64) -> ConversationMessage {
    ConversationMessage::new(ConversationRole::Operator, content, timestamp_ms)
        .expect("operator message validates")
}

fn model(content: &str, timestamp_ms: u64) -> ConversationMessage {
    ConversationMessage::new(ConversationRole::Model, content, timestamp_ms)
        .expect("model message validates")
}

#[test]
fn operator_conversation_ingests_configures_trains_and_evaluates() {
    let mut controller = NaturalLanguageTrainingController::new(config()).expect("controller");
    let ingest = controller
        .process(operator(
            "learn concept training control: natural language can add corpora and request flash learning updates",
            1,
        ))
        .expect("ingest");
    assert!(matches!(
        ingest.actions[0],
        ControlAction::TextIngested { .. }
    ));

    let configure = controller
        .process(operator(
            "set epochs 3 batch size 2 learning rate 0.1 max grad norm 1.25 scope lm head",
            2,
        ))
        .expect("configure");
    assert_eq!(controller.train_config().epochs, 3);
    assert_eq!(controller.train_config().batch_size, 2);
    assert_eq!(
        controller.train_config().trainable_scope,
        TrainableScope::LmHead
    );
    assert!(configure.reply.contains("learning_rate"));

    let train = controller
        .process(operator("train now and evaluate", 3))
        .expect("train/eval");
    assert!(train.training_report.is_some());
    assert!(train
        .actions
        .iter()
        .any(|action| matches!(action, ControlAction::TrainingCompleted { .. })));
    assert!(train
        .actions
        .iter()
        .any(|action| matches!(action, ControlAction::EvaluationCompleted { .. })));
    assert!(!controller.ledger().is_empty());
}

#[test]
fn model_role_can_self_direct_its_training_when_enabled() {
    let mut controller = NaturalLanguageTrainingController::new(config()).expect("controller");
    controller
        .process(operator(
            "learn concept self training: the model may schedule its own updates from grounded traces",
            10,
        ))
        .expect("operator ingestion");
    let turn = controller
        .process(model("self train with epochs 2 learning rate 0.06", 11))
        .expect("model-authored self training");

    assert!(turn.training_report.is_some());
    assert!(turn
        .actions
        .iter()
        .any(|action| matches!(action, ControlAction::TrainingCompleted { .. })));
}

#[test]
fn model_role_self_training_can_be_disabled_by_config() {
    let mut cfg = config();
    cfg.allow_model_self_training = false;
    let mut controller = NaturalLanguageTrainingController::new(cfg).expect("controller");
    let error = controller
        .process(model("learn: model should not mutate when disabled", 20))
        .expect_err("model mutation disabled");

    assert!(matches!(error, ControlError::ModelSelfTrainingDisabled));
}

#[test]
fn inference_command_becomes_training_data() {
    let mut controller = NaturalLanguageTrainingController::new(config()).expect("controller");
    controller
        .process(operator(
            "inference provider=global; model=reasoner-a; concept=collective training; prompt=what should be learned?; response=prioritize grounded traces with high novelty and low contradiction; confidence=0.82",
            30,
        ))
        .expect("inference ingestion");
    assert_eq!(controller.inference_count(), 1);

    let train = controller
        .process(operator("train now", 31))
        .expect("train from inference");
    assert!(train.training_report.is_some());
}

#[test]
fn training_without_data_is_rejected() {
    let mut controller = NaturalLanguageTrainingController::new(config()).expect("controller");
    let error = controller
        .process(operator("train now", 40))
        .expect_err("no data to train on");

    assert!(matches!(error, ControlError::NoTrainingData));
}

#[test]
fn invalid_learning_rate_is_rejected_by_training_config() {
    let mut controller = NaturalLanguageTrainingController::new(config()).expect("controller");
    let error = controller
        .process(operator("learning rate 0", 50))
        .expect_err("zero learning rate must fail");

    assert!(matches!(
        error,
        ControlError::Training(TrainingError::InvalidTrainConfig(_))
    ));
}
