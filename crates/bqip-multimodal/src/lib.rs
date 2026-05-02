use bqip_core::{DualState, PhaseEnvelope, Register, REGISTER_BYTES};
use bqip_training::{ConceptTokenCompiler, TokenizedDocument, TrainingError};
use serde::{Deserialize, Serialize};

const GRID_WIDTH: usize = 16;
const GRID_HEIGHT: usize = 16;
const COLOR_BINS: usize = 16;
const EDGE_BINS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Gray8,
    Rgb8,
    Rgba8,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Gray8 => 1,
            Self::Rgb8 => 3,
            Self::Rgba8 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualFrame {
    pub width: usize,
    pub height: usize,
    pub format: PixelFormat,
    pub timestamp_ms: u64,
    pub pixels: Vec<u8>,
}

impl VisualFrame {
    pub fn new(
        width: usize,
        height: usize,
        format: PixelFormat,
        timestamp_ms: u64,
        pixels: Vec<u8>,
    ) -> Result<Self, MultimodalError> {
        let frame = Self {
            width,
            height,
            format,
            timestamp_ms,
            pixels,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), MultimodalError> {
        if self.width == 0 || self.height == 0 {
            return Err(MultimodalError::InvalidDimensions);
        }
        let expected = self
            .width
            .checked_mul(self.height)
            .and_then(|pixels| pixels.checked_mul(self.format.bytes_per_pixel()))
            .ok_or(MultimodalError::FrameTooLarge)?;
        if self.pixels.len() != expected {
            return Err(MultimodalError::PixelLengthMismatch {
                expected,
                actual: self.pixels.len(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualFeatures {
    pub frame_hash: [u8; REGISTER_BYTES],
    pub width: usize,
    pub height: usize,
    pub timestamp_ms: u64,
    pub mean_luma: f32,
    pub contrast: f32,
    pub entropy: f32,
    pub color_histogram: Vec<u32>,
    pub edge_histogram: Vec<u32>,
    pub luma_grid: Vec<u8>,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
}

impl VisualFeatures {
    pub fn validate(&self) -> Result<(), MultimodalError> {
        if self.color_histogram.len() != COLOR_BINS * 3 {
            return Err(MultimodalError::InvalidFeatureShape("color_histogram"));
        }
        if self.edge_histogram.len() != EDGE_BINS {
            return Err(MultimodalError::InvalidFeatureShape("edge_histogram"));
        }
        if self.luma_grid.len() != GRID_WIDTH * GRID_HEIGHT {
            return Err(MultimodalError::InvalidFeatureShape("luma_grid"));
        }
        if !self.mean_luma.is_finite() || !self.contrast.is_finite() || !self.entropy.is_finite() {
            return Err(MultimodalError::NonFiniteFeature);
        }
        self.envelope.validate()?;
        if self.state.twin != bqip_core::phase_project(self.state.live, self.envelope) {
            return Err(MultimodalError::InvalidTwinProjection);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoClip {
    pub frames: Vec<VisualFrame>,
    pub concept_label: String,
}

impl VideoClip {
    pub fn new(
        concept_label: impl Into<String>,
        frames: Vec<VisualFrame>,
    ) -> Result<Self, MultimodalError> {
        let clip = Self {
            concept_label: concept_label.into(),
            frames,
        };
        clip.validate()?;
        Ok(clip)
    }

    pub fn validate(&self) -> Result<(), MultimodalError> {
        if self.concept_label.trim().is_empty() {
            return Err(MultimodalError::EmptyConceptLabel);
        }
        if self.frames.is_empty() {
            return Err(MultimodalError::EmptyVideo);
        }
        for frame in &self.frames {
            frame.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoFeatures {
    pub clip_hash: [u8; REGISTER_BYTES],
    pub frame_count: usize,
    pub duration_ms: u64,
    pub mean_motion: f32,
    pub max_motion: f32,
    pub scene_change_count: u32,
    pub frame_features: Vec<VisualFeatures>,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
}

impl VideoFeatures {
    pub fn validate(&self) -> Result<(), MultimodalError> {
        if self.frame_count == 0 || self.frame_features.len() != self.frame_count {
            return Err(MultimodalError::InvalidFeatureShape("frame_features"));
        }
        if !self.mean_motion.is_finite() || !self.max_motion.is_finite() {
            return Err(MultimodalError::NonFiniteFeature);
        }
        for frame in &self.frame_features {
            frame.validate()?;
        }
        self.envelope.validate()?;
        if self.state.twin != bqip_core::phase_project(self.state.live, self.envelope) {
            return Err(MultimodalError::InvalidTwinProjection);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MultimodalGrounding {
    pub concept_label: String,
    pub modality: Modality,
    pub content_hash: [u8; REGISTER_BYTES],
    pub feature_state: DualState,
    pub document: TokenizedDocument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modality {
    Image,
    Video,
}

pub fn analyze_frame(
    frame: &VisualFrame,
    envelope: PhaseEnvelope,
) -> Result<VisualFeatures, MultimodalError> {
    frame.validate()?;
    envelope.validate()?;
    let frame_hash = *blake3::hash(&frame.pixels).as_bytes();
    let luma = frame_luma(frame);
    let (mean_luma, contrast, entropy) = luma_statistics(&luma);
    let color_histogram = color_histogram(frame);
    let edge_histogram = edge_histogram(&luma, frame.width, frame.height);
    let luma_grid = downsample_luma(&luma, frame.width, frame.height);
    let live = visual_register(
        &frame_hash,
        frame.width,
        frame.height,
        mean_luma,
        contrast,
        entropy,
        &color_histogram,
        &edge_histogram,
        &luma_grid,
    );
    let features = VisualFeatures {
        frame_hash,
        width: frame.width,
        height: frame.height,
        timestamp_ms: frame.timestamp_ms,
        mean_luma,
        contrast,
        entropy,
        color_histogram,
        edge_histogram,
        luma_grid,
        envelope,
        state: DualState::from_live(live, envelope),
    };
    features.validate()?;
    Ok(features)
}

pub fn analyze_video(
    clip: &VideoClip,
    envelope: PhaseEnvelope,
) -> Result<VideoFeatures, MultimodalError> {
    clip.validate()?;
    envelope.validate()?;
    let mut frame_features = Vec::with_capacity(clip.frames.len());
    for frame in &clip.frames {
        frame_features.push(analyze_frame(frame, envelope)?);
    }

    let mut motion_values = Vec::new();
    for pair in frame_features.windows(2) {
        motion_values.push(mean_abs_diff(&pair[0].luma_grid, &pair[1].luma_grid));
    }
    let mean_motion = if motion_values.is_empty() {
        0.0
    } else {
        motion_values.iter().sum::<f32>() / motion_values.len() as f32
    };
    let max_motion = motion_values.iter().copied().fold(0.0f32, f32::max);
    let scene_change_count = motion_values
        .iter()
        .filter(|motion| **motion >= 0.12)
        .count()
        .min(u32::MAX as usize) as u32;
    let duration_ms = clip
        .frames
        .last()
        .map(|last| last.timestamp_ms)
        .unwrap_or(0)
        .saturating_sub(
            clip.frames
                .first()
                .map(|first| first.timestamp_ms)
                .unwrap_or(0),
        );
    let clip_hash = clip_hash(clip);
    let live = video_register(
        &clip_hash,
        duration_ms,
        mean_motion,
        max_motion,
        scene_change_count,
        &frame_features,
    );
    let features = VideoFeatures {
        clip_hash,
        frame_count: clip.frames.len(),
        duration_ms,
        mean_motion,
        max_motion,
        scene_change_count,
        frame_features,
        envelope,
        state: DualState::from_live(live, envelope),
    };
    features.validate()?;
    Ok(features)
}

pub fn ground_frame(
    concept_label: impl Into<String>,
    frame: &VisualFrame,
    envelope: PhaseEnvelope,
    compiler: &ConceptTokenCompiler,
) -> Result<MultimodalGrounding, MultimodalError> {
    let concept_label = concept_label.into();
    if concept_label.trim().is_empty() {
        return Err(MultimodalError::EmptyConceptLabel);
    }
    let features = analyze_frame(frame, envelope)?;
    let bytes = visual_feature_bytes(&features);
    let document = compiler.compile(concept_label.clone(), &bytes)?;
    Ok(MultimodalGrounding {
        concept_label,
        modality: Modality::Image,
        content_hash: features.frame_hash,
        feature_state: features.state,
        document,
    })
}

pub fn ground_video(
    clip: &VideoClip,
    envelope: PhaseEnvelope,
    compiler: &ConceptTokenCompiler,
) -> Result<MultimodalGrounding, MultimodalError> {
    let features = analyze_video(clip, envelope)?;
    let bytes = video_feature_bytes(&features);
    let document = compiler.compile(clip.concept_label.clone(), &bytes)?;
    Ok(MultimodalGrounding {
        concept_label: clip.concept_label.clone(),
        modality: Modality::Video,
        content_hash: features.clip_hash,
        feature_state: features.state,
        document,
    })
}

fn frame_luma(frame: &VisualFrame) -> Vec<u8> {
    let mut luma = Vec::with_capacity(frame.width * frame.height);
    for pixel in frame.pixels.chunks_exact(frame.format.bytes_per_pixel()) {
        let value = match frame.format {
            PixelFormat::Gray8 => pixel[0],
            PixelFormat::Rgb8 | PixelFormat::Rgba8 => {
                let red = pixel[0] as u32;
                let green = pixel[1] as u32;
                let blue = pixel[2] as u32;
                ((red * 77 + green * 150 + blue * 29) >> 8) as u8
            }
        };
        luma.push(value);
    }
    luma
}

fn luma_statistics(luma: &[u8]) -> (f32, f32, f32) {
    let count = luma.len().max(1) as f32;
    let mean = luma.iter().map(|value| *value as f32).sum::<f32>() / count;
    let variance = luma
        .iter()
        .map(|value| {
            let centered = *value as f32 - mean;
            centered * centered
        })
        .sum::<f32>()
        / count;
    let mut bins = [0u32; 256];
    for value in luma {
        bins[*value as usize] += 1;
    }
    let mut entropy = 0.0f32;
    for bin in bins {
        if bin == 0 {
            continue;
        }
        let probability = bin as f32 / count;
        entropy -= probability * probability.log2();
    }
    (mean / 255.0, variance.sqrt() / 255.0, entropy / 8.0)
}

fn color_histogram(frame: &VisualFrame) -> Vec<u32> {
    let mut histogram = vec![0u32; COLOR_BINS * 3];
    for pixel in frame.pixels.chunks_exact(frame.format.bytes_per_pixel()) {
        let (red, green, blue) = match frame.format {
            PixelFormat::Gray8 => (pixel[0], pixel[0], pixel[0]),
            PixelFormat::Rgb8 | PixelFormat::Rgba8 => (pixel[0], pixel[1], pixel[2]),
        };
        for (channel, value) in [red, green, blue].into_iter().enumerate() {
            let bin = (value as usize * COLOR_BINS / 256).min(COLOR_BINS - 1);
            histogram[channel * COLOR_BINS + bin] += 1;
        }
    }
    histogram
}

fn edge_histogram(luma: &[u8], width: usize, height: usize) -> Vec<u32> {
    let mut histogram = vec![0u32; EDGE_BINS];
    if width < 3 || height < 3 {
        return histogram;
    }
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = |x: usize, y: usize| luma[y * width + x] as i32;
            let gx = -index(x - 1, y - 1) + index(x + 1, y - 1) - 2 * index(x - 1, y)
                + 2 * index(x + 1, y)
                - index(x - 1, y + 1)
                + index(x + 1, y + 1);
            let gy = -index(x - 1, y - 1) - 2 * index(x, y - 1) - index(x + 1, y - 1)
                + index(x - 1, y + 1)
                + 2 * index(x, y + 1)
                + index(x + 1, y + 1);
            let magnitude = ((gx * gx + gy * gy) as f32).sqrt().min(1442.0);
            let bin = ((magnitude / 1442.0) * EDGE_BINS as f32).floor() as usize;
            histogram[bin.min(EDGE_BINS - 1)] += 1;
        }
    }
    histogram
}

fn downsample_luma(luma: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut grid = vec![0u8; GRID_WIDTH * GRID_HEIGHT];
    for gy in 0..GRID_HEIGHT {
        for gx in 0..GRID_WIDTH {
            let x_start = gx * width / GRID_WIDTH;
            let x_end = ((gx + 1) * width / GRID_WIDTH).max(x_start + 1).min(width);
            let y_start = gy * height / GRID_HEIGHT;
            let y_end = ((gy + 1) * height / GRID_HEIGHT)
                .max(y_start + 1)
                .min(height);
            let mut sum = 0u32;
            let mut count = 0u32;
            for y in y_start..y_end {
                for x in x_start..x_end {
                    sum += luma[y * width + x] as u32;
                    count += 1;
                }
            }
            grid[gy * GRID_WIDTH + gx] = (sum / count.max(1)) as u8;
        }
    }
    grid
}

fn mean_abs_diff(left: &[u8], right: &[u8]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left.abs_diff(*right) as f32 / 255.0)
        .sum::<f32>()
        / left.len().max(1) as f32
}

fn clip_hash(clip: &VideoClip) -> [u8; REGISTER_BYTES] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-video-clip");
    hasher.update(clip.concept_label.as_bytes());
    for frame in &clip.frames {
        hasher.update(&frame.timestamp_ms.to_le_bytes());
        hasher.update(&frame.width.to_le_bytes());
        hasher.update(&frame.height.to_le_bytes());
        hasher.update(&frame.pixels);
    }
    *hasher.finalize().as_bytes()
}

fn visual_register(
    frame_hash: &[u8; REGISTER_BYTES],
    width: usize,
    height: usize,
    mean_luma: f32,
    contrast: f32,
    entropy: f32,
    color_histogram: &[u32],
    edge_histogram: &[u32],
    luma_grid: &[u8],
) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-visual-register");
    hasher.update(frame_hash);
    hasher.update(&width.to_le_bytes());
    hasher.update(&height.to_le_bytes());
    hasher.update(&mean_luma.to_bits().to_le_bytes());
    hasher.update(&contrast.to_bits().to_le_bytes());
    hasher.update(&entropy.to_bits().to_le_bytes());
    for value in color_histogram {
        hasher.update(&value.to_le_bytes());
    }
    for value in edge_histogram {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(luma_grid);
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn video_register(
    clip_hash: &[u8; REGISTER_BYTES],
    duration_ms: u64,
    mean_motion: f32,
    max_motion: f32,
    scene_change_count: u32,
    frame_features: &[VisualFeatures],
) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-video-register");
    hasher.update(clip_hash);
    hasher.update(&duration_ms.to_le_bytes());
    hasher.update(&mean_motion.to_bits().to_le_bytes());
    hasher.update(&max_motion.to_bits().to_le_bytes());
    hasher.update(&scene_change_count.to_le_bytes());
    for frame in frame_features {
        hasher.update(frame.state.live.as_bytes());
    }
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn visual_feature_bytes(features: &VisualFeatures) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"visual ");
    bytes.extend_from_slice(&features.width.to_le_bytes());
    bytes.extend_from_slice(&features.height.to_le_bytes());
    bytes.extend_from_slice(&features.mean_luma.to_bits().to_le_bytes());
    bytes.extend_from_slice(&features.contrast.to_bits().to_le_bytes());
    bytes.extend_from_slice(&features.entropy.to_bits().to_le_bytes());
    for value in &features.color_histogram {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &features.edge_histogram {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&features.luma_grid);
    bytes
}

fn video_feature_bytes(features: &VideoFeatures) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"video ");
    bytes.extend_from_slice(&features.frame_count.to_le_bytes());
    bytes.extend_from_slice(&features.duration_ms.to_le_bytes());
    bytes.extend_from_slice(&features.mean_motion.to_bits().to_le_bytes());
    bytes.extend_from_slice(&features.max_motion.to_bits().to_le_bytes());
    bytes.extend_from_slice(&features.scene_change_count.to_le_bytes());
    for frame in &features.frame_features {
        bytes.extend_from_slice(frame.state.live.as_bytes());
    }
    bytes
}

#[derive(Debug, thiserror::Error)]
pub enum MultimodalError {
    #[error("frame width and height must be nonzero")]
    InvalidDimensions,
    #[error("frame byte length mismatch: expected {expected}, actual {actual}")]
    PixelLengthMismatch { expected: usize, actual: usize },
    #[error("frame byte size overflow")]
    FrameTooLarge,
    #[error("concept label must be nonempty")]
    EmptyConceptLabel,
    #[error("video clip must contain at least one frame")]
    EmptyVideo,
    #[error("invalid feature shape: {0}")]
    InvalidFeatureShape(&'static str),
    #[error("visual feature contains a non-finite value")]
    NonFiniteFeature,
    #[error("dual-state twin projection does not match phase envelope")]
    InvalidTwinProjection,
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
    #[error(transparent)]
    Training(#[from] TrainingError),
}
