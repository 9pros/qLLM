use bqip_core::{phase_project, REGISTER_BYTES};
use bqip_transformer::{
    HybridConfig, HybridTransformer, LazyBqipMemory, MixerKind, TransformerError,
};

#[test]
fn gated_deltanet_bqip_pipeline_materializes_structural_state() {
    let config = HybridConfig {
        vocab_size: 1024,
        d_model: 48,
        max_context: 64,
        mixer: MixerKind::GatedDeltaNet,
        num_heads: 6,
        phase_delta: 1,
        phase_bucket_size: 32,
        delta_step: 0.3,
        forget_floor: 0.03,
        alpha: std::f32::consts::FRAC_1_SQRT_2,
        beta: std::f32::consts::FRAC_1_SQRT_2,
        structural_feedback: 0.06,
    };
    let model = HybridTransformer::new(config, [17u8; REGISTER_BYTES]).unwrap();
    let mut memory = LazyBqipMemory::new();

    let output = model
        .forward(&[2, 7, 9, 35, 39, 72, 73, 74], &mut memory)
        .unwrap();

    assert_eq!(output.hidden_states.len(), 8);
    assert_eq!(output.structural_states.len(), 8);
    assert!(memory.len() >= 8);

    for state in &output.structural_states {
        let materialized = memory.materialize(state.memory_node).unwrap();
        assert_eq!(
            state.dual_state.twin,
            phase_project(state.dual_state.live, state.envelope)
        );
        assert!(materialized.live.as_bytes().iter().any(|byte| *byte != 0));
    }
}

#[test]
fn context_limit_is_enforced_before_memory_mutation() {
    let config = HybridConfig {
        max_context: 2,
        ..HybridConfig::default()
    };
    let model = HybridTransformer::new(config, [21u8; REGISTER_BYTES]).unwrap();
    let mut memory = LazyBqipMemory::new();

    let err = model.forward(&[1, 2, 3], &mut memory).unwrap_err();

    assert!(matches!(
        err,
        TransformerError::ContextTooLong {
            requested: 3,
            max: 2
        }
    ));
    assert!(memory.is_empty());
}
