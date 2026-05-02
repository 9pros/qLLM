use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bqip_training::{
    evaluate, AppendOnlyMetricsLedger, ConceptTokenCompiler, ConceptTokenizerConfig, EvalSplit,
    HybridTrainer, MetricsLedger, SplitRatios, TrainConfig, TrainableScope, TrainingCorpus,
    TrainingError,
};
use bqip_transformer::{HybridConfig, HybridTransformer, MixerKind};

const REGISTER_BYTES: usize = 32;

fn temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bqip-training-{label}-{}-{nanos}.corpus",
        std::process::id()
    ))
}

fn model() -> HybridTransformer {
    HybridTransformer::new(
        HybridConfig {
            vocab_size: 32,
            d_model: 16,
            max_context: 8,
            mixer: MixerKind::DeltaNet,
            num_heads: 4,
            phase_bucket_size: 8,
            structural_feedback: 0.02,
            ..HybridConfig::default()
        },
        [44u8; REGISTER_BYTES],
    )
    .expect("model config should be valid")
}

fn corpus() -> TrainingCorpus {
    TrainingCorpus::from_sequences(
        32,
        4,
        &[
            vec![3, 7, 11, 15, 19, 23],
            vec![3, 7, 11, 15, 19, 23],
            vec![3, 7, 11, 15, 19, 23],
            vec![4, 8, 12, 16, 20, 24],
            vec![4, 8, 12, 16, 20, 24],
            vec![5, 9, 13, 17, 21, 25],
        ],
        Some("training fixture".to_string()),
    )
    .expect("corpus should be valid")
}

#[test]
fn tied_lm_head_training_reduces_loss_and_updates_weights() {
    let base_model = model();
    let base_weights = base_model.weights();
    let corpus = corpus();
    let initial = evaluate(&base_model, &corpus).expect("initial evaluation");
    let trained = HybridTrainer::new(
        base_model,
        TrainConfig {
            epochs: 10,
            batch_size: 3,
            learning_rate: 0.18,
            weight_decay: 0.0001,
            max_grad_norm: 2.0,
            twin_loss_weight: 0.05,
            phase_delta_increment: 0,
            twin_perturbation_scale: 0.02,
            graph_contrastive_weight: 0.1,
            ..TrainConfig::default()
        },
    )
    .expect("trainer config should be valid")
    .train(&corpus)
    .expect("training should complete");
    let final_metrics = evaluate(&trained.model, &corpus).expect("final evaluation");
    let trained_weights = trained.model.weights();

    assert!(final_metrics.mean_loss < initial.mean_loss);
    assert!(trained.report.final_loss < trained.report.initial_loss);
    assert_ne!(base_weights.lm_head.data, trained_weights.lm_head.data);
    assert_eq!(base_weights.query.data, trained_weights.query.data);
    assert_eq!(trained.report.epochs.len(), 10);
}

#[test]
fn concept_tokenizer_split_and_metrics_ledger_form_training_run() {
    let compiler = ConceptTokenCompiler::new(ConceptTokenizerConfig {
        vocab_size: 32,
        max_tokens: 24,
        ngram_min: 1,
        ngram_max: 3,
        include_byte_tokens: true,
    })
    .expect("tokenizer config should be valid");
    let first = compiler
        .compile(
            "Reactive Graph",
            b"reactive graph subscriptions emit deterministic concept deltas",
        )
        .expect("first document tokenizes");
    let second = compiler
        .compile(
            "Endpoint Ingestor",
            b"endpoint ingestor fetches public api payloads and builds training windows",
        )
        .expect("second document tokenizes");
    let corpus = TrainingCorpus::from_tokenized_documents(32, 5, &[first, second])
        .expect("tokenized corpus should build");
    let split = corpus
        .split(
            SplitRatios {
                train: 0.6,
                validation: 0.2,
                test: 0.2,
            },
            b"deterministic split",
        )
        .expect("split should validate");
    let mut ledger = MetricsLedger::new();
    let trained = HybridTrainer::new(
        model(),
        TrainConfig {
            epochs: 3,
            batch_size: 2,
            learning_rate: 0.12,
            max_grad_norm: 2.0,
            twin_loss_weight: 0.05,
            phase_delta_increment: 0,
            twin_perturbation_scale: 0.02,
            graph_contrastive_weight: 0.1,
            ..TrainConfig::default()
        },
    )
    .expect("trainer config should be valid")
    .train_with_split(&split, &mut ledger)
    .expect("split training should complete");

    assert!(trained.report.train.final_loss.is_finite());
    assert!(ledger.len() >= 5);
    ledger.validate().expect("ledger hash chain validates");
    assert!(matches!(
        ledger.records()[0].event,
        bqip_training::MetricsLedgerEvent::Evaluation {
            split: EvalSplit::Train,
            ..
        }
    ));
}

#[test]
fn append_only_metrics_ledger_reopens_and_rejects_corruption() {
    let path = temp_path("metrics").with_extension("ledger");
    let _ = fs::remove_file(&path);
    {
        let mut ledger = AppendOnlyMetricsLedger::open(&path).expect("open metrics ledger");
        ledger
            .append(bqip_training::MetricsLedgerEvent::Evaluation {
                split: EvalSplit::Train,
                metrics: evaluate(&model(), &corpus()).expect("evaluation metrics"),
            })
            .expect("append evaluation");
        assert_eq!(ledger.ledger().len(), 1);
    }
    let reopened = AppendOnlyMetricsLedger::open(&path).expect("reopen metrics ledger");
    assert_eq!(reopened.ledger().len(), 1);
    let mut bytes = fs::read(&path).expect("read ledger bytes");
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&path, bytes).expect("write corrupted ledger");
    let error = AppendOnlyMetricsLedger::open(&path).expect_err("corruption must fail");
    assert!(matches!(
        error,
        TrainingError::PersistentHashMismatch | TrainingError::Codec(_)
    ));
    let _ = fs::remove_file(path);
}

#[test]
fn dense_finite_difference_scope_updates_internal_transformer_weights() {
    let small_model = HybridTransformer::new(
        HybridConfig {
            vocab_size: 8,
            d_model: 4,
            max_context: 4,
            mixer: MixerKind::SoftmaxAttention,
            num_heads: 1,
            phase_bucket_size: 4,
            structural_feedback: 0.0,
            ..HybridConfig::default()
        },
        [9u8; REGISTER_BYTES],
    )
    .expect("small model config should be valid");
    let base_weights = small_model.weights();
    let corpus = TrainingCorpus::from_sequences(
        8,
        3,
        &[vec![1, 2, 3, 4], vec![1, 2, 3, 4], vec![2, 3, 4, 5]],
        Some("finite difference".to_string()),
    )
    .expect("small corpus should be valid");
    let trained = HybridTrainer::new(
        small_model,
        TrainConfig {
            epochs: 1,
            batch_size: 2,
            learning_rate: 0.02,
            max_grad_norm: 0.5,
            trainable_scope: TrainableScope::DenseFiniteDifference,
            finite_difference_epsilon: 1.0e-2,
            twin_loss_weight: 0.05,
            phase_delta_increment: 0,
            twin_perturbation_scale: 0.02,
            graph_contrastive_weight: 0.1,
            ..TrainConfig::default()
        },
    )
    .expect("finite difference config should be valid")
    .train(&corpus)
    .expect("finite difference training should complete");
    let trained_weights = trained.model.weights();

    assert_ne!(base_weights.query.data, trained_weights.query.data);
    assert_ne!(
        base_weights.embeddings.data,
        trained_weights.embeddings.data
    );
    assert!(trained.report.final_loss.is_finite());
}

#[test]
fn corpus_round_trip_preserves_checked_examples() {
    let path = temp_path("round-trip");
    let corpus = corpus();
    corpus.save(&path).expect("save checked corpus");
    let loaded = TrainingCorpus::load(&path).expect("load checked corpus");
    assert_eq!(loaded, corpus);
    fs::remove_file(path).expect("remove corpus");
}

#[test]
fn corpus_hash_mismatch_is_rejected() {
    let mut corpus = corpus();
    corpus.examples[0].target_token ^= 1;
    let error = corpus.validate().expect_err("tampered corpus must fail");
    assert!(matches!(error, TrainingError::SourceHashMismatch));
}

#[test]
fn model_corpus_mismatch_is_rejected_before_training() {
    let model = model();
    let mismatched = TrainingCorpus::from_sequences(16, 4, &[vec![1, 2, 3]], None)
        .expect("mismatched corpus is internally valid");
    let error = HybridTrainer::new(model, TrainConfig::default())
        .expect("trainer config should be valid")
        .train(&mismatched)
        .expect_err("vocab mismatch must fail");
    assert!(matches!(
        error,
        TrainingError::CorpusModelMismatch {
            field: "vocab_size",
            ..
        }
    ));
}
