use bqip_core::{PhaseEnvelope, REGISTER_BYTES};
use bqip_multimodal::{
    analyze_frame, analyze_video, ground_frame, ground_video, PixelFormat, VideoClip, VisualFrame,
};
use bqip_training::{ConceptTokenCompiler, ConceptTokenizerConfig};

fn compiler() -> ConceptTokenCompiler {
    ConceptTokenCompiler::new(ConceptTokenizerConfig {
        vocab_size: 128,
        max_tokens: 64,
        ngram_min: 1,
        ngram_max: 2,
        include_byte_tokens: true,
    })
    .expect("tokenizer config should validate")
}

fn envelope() -> PhaseEnvelope {
    PhaseEnvelope::balanced(0x5151_aa55)
}

fn rgb_frame(timestamp_ms: u64, invert: bool) -> VisualFrame {
    let mut pixels = Vec::with_capacity(8 * 8 * 3);
    for y in 0..8 {
        for x in 0..8 {
            let edge = if x >= 4 { 220 } else { 24 };
            let value = if invert { 255 - edge } else { edge };
            pixels.extend_from_slice(&[value, (y * 32) as u8, (x * 32) as u8]);
        }
    }
    VisualFrame::new(8, 8, PixelFormat::Rgb8, timestamp_ms, pixels).expect("frame validates")
}

#[test]
fn visual_features_are_deterministic_and_phase_projected() {
    let frame = rgb_frame(10, false);
    let first = analyze_frame(&frame, envelope()).expect("frame analysis succeeds");
    let second = analyze_frame(&frame, envelope()).expect("frame analysis is deterministic");

    assert_eq!(first, second);
    assert_eq!(first.color_histogram.len(), 48);
    assert_eq!(first.edge_histogram.len(), 8);
    assert_eq!(first.luma_grid.len(), 256);
    assert_eq!(
        first.state.twin,
        bqip_core::phase_project(first.state.live, first.envelope)
    );
    assert!(first.contrast > 0.0);
}

#[test]
fn video_features_track_motion_and_scene_changes() {
    let clip = VideoClip::new(
        "moving edge",
        vec![
            rgb_frame(0, false),
            rgb_frame(33, true),
            rgb_frame(66, false),
        ],
    )
    .expect("video clip validates");
    let features = analyze_video(&clip, envelope()).expect("video analysis succeeds");

    assert_eq!(features.frame_count, 3);
    assert_eq!(features.duration_ms, 66);
    assert!(features.mean_motion > 0.1);
    assert!(features.scene_change_count >= 1);
    assert_eq!(
        features.state.twin,
        bqip_core::phase_project(features.state.live, features.envelope)
    );
}

#[test]
fn multimodal_groundings_compile_training_documents() {
    let frame = rgb_frame(42, false);
    let image_grounding = ground_frame("visual edge", &frame, envelope(), &compiler())
        .expect("image grounding succeeds");
    let clip = VideoClip::new("video edge", vec![rgb_frame(0, false), rgb_frame(33, true)])
        .expect("clip validates");
    let video_grounding = ground_video(&clip, envelope(), &compiler()).expect("video grounding");

    assert_eq!(image_grounding.content_hash.len(), REGISTER_BYTES);
    assert!(image_grounding.document.tokens.len() >= 2);
    assert!(video_grounding.document.tokens.len() >= 2);
    assert_ne!(image_grounding.feature_state, video_grounding.feature_state);
}
