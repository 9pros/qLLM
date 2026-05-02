use std::path::PathBuf;

use bqip_core::{InterfaceKind, PhaseEnvelope, Register, RegisterLane, REGISTER_BYTES};
use bqip_knowledge_graph::{
    AppendOnlyGraphLog, BlockId, BlockRange, ConceptId, ConceptProjection, EndpointGrounding,
    EndpointKind, GraphError, GraphLogEvent, GraphSubscriptionFilter, GraphSubscriptionId,
    MemoryBlock, ProjectionRequest, ProjectionWeights, ReactiveKnowledgeGraph,
    ReactiveNotification, WeightedBlockRef,
};
use bqip_transformer::{
    AppendLogLazyBqipMemory, DualStatePrediction, HybridConfig, HybridTransformer, LazyNodeId,
    TransformerError,
};
use serde::{Deserialize, Serialize};

const DEFAULT_OBSERVATION_FRESHNESS_HALF_LIFE_MS: u64 = 86_400_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenseEvent {
    pub source: EventSource,
    pub tokens: Vec<u32>,
    pub timestamp_ms: u64,
    pub concept_label: String,
    pub endpoint_lane: RegisterLane,
    pub endpoint_interface_kind: InterfaceKind,
    pub endpoint_kind: EndpointKind,
    pub endpoint_slot_index: u64,
    pub canonical_endpoint: Vec<u8>,
    pub payload: Vec<u8>,
    pub freshness_half_life_millis: u64,
}

impl SenseEvent {
    pub fn new(source: EventSource, tokens: Vec<u32>, timestamp_ms: u64) -> Self {
        let payload = tokens_to_payload(&tokens);
        Self {
            concept_label: source.default_concept_label().to_string(),
            endpoint_lane: source.default_lane(),
            endpoint_interface_kind: InterfaceKind::Application,
            endpoint_kind: source.default_endpoint_kind(),
            endpoint_slot_index: source.default_slot_index(),
            canonical_endpoint: source.default_endpoint().to_vec(),
            freshness_half_life_millis: DEFAULT_OBSERVATION_FRESHNESS_HALF_LIFE_MS,
            source,
            tokens,
            timestamp_ms,
            payload,
        }
    }

    pub fn with_endpoint_observation(
        mut self,
        concept_label: impl Into<String>,
        endpoint_lane: RegisterLane,
        endpoint_interface_kind: InterfaceKind,
        endpoint_kind: EndpointKind,
        endpoint_slot_index: u64,
        canonical_endpoint: impl Into<Vec<u8>>,
        payload: impl Into<Vec<u8>>,
        freshness_half_life_millis: u64,
    ) -> Self {
        self.concept_label = concept_label.into();
        self.endpoint_lane = endpoint_lane;
        self.endpoint_interface_kind = endpoint_interface_kind;
        self.endpoint_kind = endpoint_kind;
        self.endpoint_slot_index = endpoint_slot_index;
        self.canonical_endpoint = canonical_endpoint.into();
        self.payload = payload.into();
        self.freshness_half_life_millis = freshness_half_life_millis;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    User,
    Tool,
    System,
    PredictionFeedback,
}

impl EventSource {
    const fn tag(&self) -> &'static [u8] {
        match self {
            Self::User => b"user",
            Self::Tool => b"tool",
            Self::System => b"system",
            Self::PredictionFeedback => b"prediction-feedback",
        }
    }

    const fn default_concept_label(&self) -> &'static str {
        match self {
            Self::User => "user observation",
            Self::Tool => "tool observation",
            Self::System => "system observation",
            Self::PredictionFeedback => "prediction feedback",
        }
    }

    const fn default_endpoint(&self) -> &'static [u8] {
        match self {
            Self::User => b"bqip://sense/user",
            Self::Tool => b"bqip://sense/tool",
            Self::System => b"bqip://sense/system",
            Self::PredictionFeedback => b"bqip://sense/prediction-feedback",
        }
    }

    const fn default_slot_index(&self) -> u64 {
        match self {
            Self::User => 1,
            Self::Tool => 2,
            Self::System => 3,
            Self::PredictionFeedback => 4,
        }
    }

    const fn default_lane(&self) -> RegisterLane {
        match self {
            Self::User => RegisterLane::GenericEndpoint,
            Self::Tool => RegisterLane::PublicApi,
            Self::System => RegisterLane::GenericEndpoint,
            Self::PredictionFeedback => RegisterLane::GenericEndpoint,
        }
    }

    const fn default_endpoint_kind(&self) -> EndpointKind {
        match self {
            Self::User => EndpointKind::Document,
            Self::Tool => EndpointKind::PublicApi,
            Self::System => EndpointKind::Feed,
            Self::PredictionFeedback => EndpointKind::Dataset,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: u64,
    pub label: String,
    pub target_token: u32,
    pub priority: f32,
    pub satisfaction: f32,
}

impl Goal {
    pub fn new(id: u64, label: impl Into<String>, target_token: u32, priority: f32) -> Self {
        Self {
            id,
            label: label.into(),
            target_token,
            priority: priority.clamp(0.0, 1.0),
            satisfaction: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    EmitPrediction {
        token_id: u32,
        confidence: f32,
        memory_node: usize,
    },
    StabilizeMemory {
        record_count: u64,
    },
    Hold {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub reward: f32,
    pub prediction_matched_goal: bool,
    pub compacted: bool,
}

#[derive(Clone, Debug)]
pub struct LifeTick {
    pub tick_index: u64,
    pub event: SenseEvent,
    pub graph_block_id: BlockId,
    pub graph_concept_id: ConceptId,
    pub graph_projection: ConceptProjection,
    pub graph_notifications: Vec<ReactiveNotification>,
    pub graph_log_head: [u8; REGISTER_BYTES],
    pub model_context: Vec<u32>,
    pub prediction: DualStatePrediction,
    pub committed_prediction_node: LazyNodeId,
    pub action: Action,
    pub outcome: Outcome,
    pub active_goal: Option<Goal>,
}

#[derive(Clone, Debug)]
pub struct DaemonConfig {
    pub model: HybridConfig,
    pub top_k: usize,
    pub compaction_interval_records: u64,
    pub graph_projection_limit: usize,
    pub graph_projection_token_budget: usize,
    pub node_public_key: [u8; REGISTER_BYTES],
    pub memory_log_path: PathBuf,
    pub graph_log_path: PathBuf,
}

impl DaemonConfig {
    pub fn validate(&self) -> Result<(), DaemonError> {
        self.model.validate()?;
        if self.top_k == 0 {
            return Err(DaemonError::InvalidConfig("top_k must be nonzero"));
        }
        if self.compaction_interval_records == 0 {
            return Err(DaemonError::InvalidConfig(
                "compaction_interval_records must be nonzero",
            ));
        }
        if self.graph_projection_limit == 0 {
            return Err(DaemonError::InvalidConfig(
                "graph_projection_limit must be nonzero",
            ));
        }
        if self.graph_projection_token_budget == 0 {
            return Err(DaemonError::InvalidConfig(
                "graph_projection_token_budget must be nonzero",
            ));
        }
        Ok(())
    }
}

pub struct BqipDaemon {
    model: HybridTransformer,
    memory: AppendLogLazyBqipMemory,
    graph: ReactiveKnowledgeGraph,
    graph_log: AppendOnlyGraphLog,
    graph_subscription_id: GraphSubscriptionId,
    config: DaemonConfig,
    goals: Vec<Goal>,
    tick_index: u64,
    cumulative_reward: f32,
}

impl BqipDaemon {
    pub fn open(config: DaemonConfig) -> Result<Self, DaemonError> {
        config.validate()?;
        let model = HybridTransformer::new(config.model.clone(), config.node_public_key)?;
        let memory = AppendLogLazyBqipMemory::open(&config.memory_log_path)?;
        let graph_log = AppendOnlyGraphLog::open(&config.graph_log_path)?;
        let mut graph = ReactiveKnowledgeGraph::from_graph(graph_log.replay()?);
        let graph_subscription_id = graph.subscribe(GraphSubscriptionFilter::all())?;
        Ok(Self {
            model,
            memory,
            graph,
            graph_log,
            graph_subscription_id,
            config,
            goals: Vec::new(),
            tick_index: 0,
            cumulative_reward: 0.0,
        })
    }

    pub fn add_goal(&mut self, goal: Goal) {
        if let Some(existing) = self
            .goals
            .iter_mut()
            .find(|existing| existing.id == goal.id)
        {
            *existing = goal;
        } else {
            self.goals.push(goal);
        }
        self.goals.sort_by(|left, right| {
            right
                .priority
                .total_cmp(&left.priority)
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    pub fn tick(&mut self, event: SenseEvent) -> Result<LifeTick, DaemonError> {
        if event.tokens.is_empty() {
            return Err(DaemonError::EmptyEvent);
        }
        if event.payload.is_empty() {
            return Err(DaemonError::EmptyObservationPayload);
        }
        if event.concept_label.trim().is_empty() {
            return Err(DaemonError::InvalidObservation("concept_label is empty"));
        }
        if event.canonical_endpoint.is_empty() {
            return Err(DaemonError::InvalidObservation(
                "canonical_endpoint is empty",
            ));
        }
        if event.freshness_half_life_millis == 0 {
            return Err(DaemonError::InvalidObservation(
                "freshness_half_life_millis is zero",
            ));
        }

        let graph_ingestion = self.ingest_observation(&event)?;
        let graph_notifications = self.graph.drain(self.graph_subscription_id)?;
        let model_context = self.build_model_context(&event, &graph_ingestion.projection);
        let prediction = self.model.predict_dual_state_logged(
            &model_context,
            &mut self.memory,
            self.config.top_k,
        )?;
        let committed_prediction_node =
            prediction.commit(&mut self.memory, self.model.config().phase_delta)?;
        let active_goal = self.select_goal(prediction.predicted_token_id).cloned();
        let confidence = prediction_confidence(&prediction);
        let action = self.select_action(
            prediction.predicted_token_id,
            committed_prediction_node,
            confidence,
        );
        let mut compacted = false;
        if self.memory.record_count() % self.config.compaction_interval_records == 0 {
            self.memory.compact()?;
            compacted = true;
        }
        let outcome = self.score_outcome(prediction.predicted_token_id, confidence, compacted);
        self.cumulative_reward += outcome.reward;
        self.update_goal_satisfaction(prediction.predicted_token_id, outcome.reward);

        let tick = LifeTick {
            tick_index: self.tick_index,
            event,
            graph_block_id: graph_ingestion.block_id,
            graph_concept_id: graph_ingestion.concept_id,
            graph_projection: graph_ingestion.projection,
            graph_notifications,
            graph_log_head: self.graph_log.log().head_hash(),
            model_context,
            prediction,
            committed_prediction_node,
            action,
            outcome,
            active_goal,
        };
        self.tick_index += 1;
        Ok(tick)
    }

    pub fn memory_record_count(&self) -> u64 {
        self.memory.record_count()
    }

    pub fn memory_node_count(&self) -> usize {
        self.memory.len()
    }

    pub fn graph_block_count(&self) -> usize {
        self.graph.graph().block_count()
    }

    pub fn graph_concept_count(&self) -> usize {
        self.graph.graph().concept_count()
    }

    pub fn graph_edge_count(&self) -> usize {
        self.graph.graph().edge_count()
    }

    pub fn graph_log_record_count(&self) -> usize {
        self.graph_log.log().len()
    }

    pub fn graph_log_head(&self) -> [u8; REGISTER_BYTES] {
        self.graph_log.log().head_hash()
    }

    pub fn concept_id_for_label(&self, label: &str) -> Option<ConceptId> {
        self.graph.graph().concept_id_for_label(label)
    }

    pub fn pending_graph_notification_count(&self) -> Result<usize, DaemonError> {
        Ok(self.graph.pending_len(self.graph_subscription_id)?)
    }

    pub fn tick_index(&self) -> u64 {
        self.tick_index
    }

    pub fn cumulative_reward(&self) -> f32 {
        self.cumulative_reward
    }

    pub fn goals(&self) -> &[Goal] {
        &self.goals
    }

    pub fn checkpoint_path(&self) -> PathBuf {
        self.memory.checkpoint_path().to_path_buf()
    }

    pub fn project_graph(
        &self,
        request: &ProjectionRequest,
    ) -> Result<ConceptProjection, DaemonError> {
        Ok(self.graph.graph().project(request)?)
    }

    fn ingest_observation(&mut self, event: &SenseEvent) -> Result<GraphIngestion, DaemonError> {
        let envelope = self.observation_envelope(event)?;
        let endpoint = EndpointGrounding::new(
            event.endpoint_lane,
            event.endpoint_slot_index,
            event.canonical_endpoint.clone(),
            event.endpoint_interface_kind,
            event.endpoint_kind.clone(),
            &self.config.node_public_key,
            event.timestamp_ms,
        )?;
        let block = MemoryBlock::from_content(
            endpoint,
            BlockRange::new(0, event.payload.len() as u64)?,
            None,
            &event.payload,
            observation_features(event),
            envelope,
            event.freshness_half_life_millis,
        )?;
        let block_id = block.id;
        self.append_graph_event(GraphLogEvent::InsertMemoryBlock(block))?;
        let concept_event = GraphLogEvent::UpsertConcept {
            label: event.concept_label.clone(),
            envelope,
            block_members: vec![WeightedBlockRef::new(block_id, 1.0)?],
            concept_members: Vec::new(),
            updated_at_unix_millis: event.timestamp_ms,
        };
        self.append_graph_event(concept_event)?;
        let concept_id = self
            .graph
            .graph()
            .concept_id_for_label(&event.concept_label)
            .ok_or(DaemonError::GraphConceptMissingAfterUpsert)?;
        let query_state = self
            .graph
            .graph()
            .concept(concept_id)
            .ok_or(DaemonError::GraphConceptMissingAfterUpsert)?
            .state
            .live;
        let projection = self.graph.graph().project(&ProjectionRequest {
            query_label: event.concept_label.clone(),
            query_state,
            reference_time_unix_millis: event.timestamp_ms,
            limit: self.config.graph_projection_limit,
            weights: ProjectionWeights::default(),
        })?;
        Ok(GraphIngestion {
            block_id,
            concept_id,
            projection,
        })
    }

    fn append_graph_event(&mut self, event: GraphLogEvent) -> Result<(), DaemonError> {
        let mut candidate_graph = self.graph.clone();
        candidate_graph.apply_event(event.clone())?;
        self.graph_log.append(event)?;
        self.graph = candidate_graph;
        Ok(())
    }

    fn observation_envelope(&self, event: &SenseEvent) -> Result<PhaseEnvelope, DaemonError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bqip-daemon-observation-envelope");
        hasher.update(event.source.tag());
        hasher.update(event.concept_label.as_bytes());
        hasher.update(&event.canonical_endpoint);
        hasher.update(&self.config.node_public_key);
        let hash = hasher.finalize();
        let mut sig_bytes = [0u8; 8];
        sig_bytes.copy_from_slice(&hash.as_bytes()[..8]);
        Ok(PhaseEnvelope::new(
            u64::from_le_bytes(sig_bytes),
            self.config.model.alpha,
            self.config.model.beta,
            self.tick_index.min(u32::MAX as u64) as u32,
        )?)
    }

    fn build_model_context(&self, event: &SenseEvent, projection: &ConceptProjection) -> Vec<u32> {
        let mut context = event.tokens.clone();
        context.extend(projection_context_tokens(
            projection,
            self.config.model.vocab_size,
            self.config.graph_projection_token_budget,
        ));
        if context.len() > self.config.model.max_context {
            context[context.len() - self.config.model.max_context..].to_vec()
        } else {
            context
        }
    }

    fn select_goal(&self, predicted_token: u32) -> Option<&Goal> {
        self.goals
            .iter()
            .find(|goal| goal.target_token == predicted_token)
            .or_else(|| self.goals.first())
    }

    fn select_action(
        &self,
        predicted_token: u32,
        memory_node: LazyNodeId,
        confidence: f32,
    ) -> Action {
        if confidence >= 0.35 {
            Action::EmitPrediction {
                token_id: predicted_token,
                confidence,
                memory_node: memory_node.index(),
            }
        } else if self.memory.record_count() >= self.config.compaction_interval_records {
            Action::StabilizeMemory {
                record_count: self.memory.record_count(),
            }
        } else {
            Action::Hold {
                reason: "prediction confidence below action threshold".to_string(),
            }
        }
    }

    fn score_outcome(&self, predicted_token: u32, confidence: f32, compacted: bool) -> Outcome {
        let prediction_matched_goal = self
            .goals
            .iter()
            .any(|goal| goal.target_token == predicted_token);
        let goal_reward = if prediction_matched_goal { 1.0 } else { -0.05 };
        let compaction_reward = if compacted { 0.05 } else { 0.0 };
        Outcome {
            reward: (goal_reward * confidence + compaction_reward).clamp(-1.0, 1.0),
            prediction_matched_goal,
            compacted,
        }
    }

    fn update_goal_satisfaction(&mut self, predicted_token: u32, reward: f32) {
        for goal in &mut self.goals {
            if goal.target_token == predicted_token {
                goal.satisfaction = (goal.satisfaction + reward.max(0.0)).clamp(0.0, 1.0);
            } else {
                goal.satisfaction = (goal.satisfaction * 0.995).clamp(0.0, 1.0);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct GraphIngestion {
    block_id: BlockId,
    concept_id: ConceptId,
    projection: ConceptProjection,
}

pub fn prediction_confidence(prediction: &DualStatePrediction) -> f32 {
    let Some(best) = prediction.top_logits.first() else {
        return 0.0;
    };
    if prediction.top_logits.len() == 1 {
        return 1.0;
    }
    let runner_up = prediction.top_logits[1].logit;
    let margin = best.logit - runner_up;
    1.0 / (1.0 + (-margin).exp())
}

fn tokens_to_payload(tokens: &[u32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(tokens.len() * 4);
    for token in tokens {
        payload.extend_from_slice(&token.to_le_bytes());
    }
    payload
}

fn observation_features(event: &SenseEvent) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-daemon-observation-features");
    hasher.update(event.source.tag());
    hasher.update(event.concept_label.as_bytes());
    hasher.update(&event.canonical_endpoint);
    hasher.update(&event.timestamp_ms.to_le_bytes());
    for token in &event.tokens {
        hasher.update(&token.to_le_bytes());
    }
    hasher.update(&event.payload);
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn projection_context_tokens(
    projection: &ConceptProjection,
    vocab_size: usize,
    token_budget: usize,
) -> Vec<u32> {
    let mut tokens = Vec::with_capacity(token_budget);
    for concept_match in &projection.matches {
        if tokens.len() >= token_budget {
            break;
        }
        tokens.push(projected_token(
            concept_match.concept_id.as_bytes(),
            vocab_size,
        ));
        if tokens.len() >= token_budget {
            break;
        }
        let score_bucket = (concept_match.score.clamp(0.0, 1.0) * 65_535.0).round() as u32;
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bqip-projection-score-token");
        hasher.update(concept_match.concept_id.as_bytes());
        hasher.update(&score_bucket.to_le_bytes());
        tokens.push(projected_token(hasher.finalize().as_bytes(), vocab_size));
    }
    tokens
}

fn projected_token(bytes: &[u8], vocab_size: usize) -> u32 {
    let mut token_bytes = [0u8; 4];
    token_bytes.copy_from_slice(&bytes[..4]);
    u32::from_le_bytes(token_bytes) % vocab_size as u32
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("invalid daemon config: {0}")]
    InvalidConfig(&'static str),
    #[error("sense event must contain at least one token")]
    EmptyEvent,
    #[error("sense event observation payload must contain at least one byte")]
    EmptyObservationPayload,
    #[error("invalid sense observation: {0}")]
    InvalidObservation(&'static str),
    #[error("graph concept missing after upsert")]
    GraphConceptMissingAfterUpsert,
    #[error(transparent)]
    Graph(#[from] GraphError),
    #[error(transparent)]
    Transformer(#[from] TransformerError),
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
}
