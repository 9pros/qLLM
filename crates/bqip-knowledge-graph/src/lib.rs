use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use bqip_core::{
    derive_register_id, phase_project, DualState, InterfaceKind, PhaseEnvelope, Register,
    RegisterId, RegisterLane, REGISTER_BITS, REGISTER_BYTES,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const LOG_RECORD_MAGIC: &[u8; 8] = b"BQIPKG01";
const HEADER_BYTES: usize = 48;
const GENESIS_LOG_HASH: [u8; 32] = [0u8; 32];

fn encode_checked<T: Serialize>(magic: &[u8; 8], value: &T) -> Result<Vec<u8>, GraphError> {
    let payload = bincode::serialize(value)?;
    let hash = blake3::hash(&payload);
    let mut bytes = Vec::with_capacity(HEADER_BYTES + payload.len());
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(hash.as_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

fn decode_checked_with_len<T: DeserializeOwned>(
    magic: &[u8; 8],
    bytes: &[u8],
) -> Result<(T, usize), GraphError> {
    if bytes.len() < HEADER_BYTES || &bytes[..8] != magic {
        return Err(GraphError::InvalidPersistentHeader);
    }

    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&bytes[8..16]);
    let payload_len = u64::from_le_bytes(len_bytes) as usize;
    let required = HEADER_BYTES
        .checked_add(payload_len)
        .ok_or(GraphError::InvalidPersistentHeader)?;
    if required > bytes.len() {
        return Err(GraphError::PersistentCapacity {
            capacity: bytes.len(),
            required,
        });
    }

    let payload = &bytes[HEADER_BYTES..required];
    let actual_hash = blake3::hash(payload);
    if &bytes[16..48] != actual_hash.as_bytes() {
        return Err(GraphError::PersistentHashMismatch);
    }

    Ok((bincode::deserialize(payload)?, required))
}

fn record_chain_hash(
    previous_hash: [u8; 32],
    record: &GraphLogRecord,
) -> Result<[u8; 32], GraphError> {
    let payload = bincode::serialize(record)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&previous_hash);
    hasher.update(&payload);
    Ok(*hasher.finalize().as_bytes())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId([u8; REGISTER_BYTES]);

impl BlockId {
    pub const fn as_bytes(&self) -> &[u8; REGISTER_BYTES] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConceptId([u8; REGISTER_BYTES]);

impl ConceptId {
    pub const fn as_bytes(&self) -> &[u8; REGISTER_BYTES] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId([u8; REGISTER_BYTES]);

impl EdgeId {
    pub const fn as_bytes(&self) -> &[u8; REGISTER_BYTES] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointKind {
    Website,
    PublicApi,
    PublicProxy,
    Document,
    Repository,
    Feed,
    Socket,
    Dataset,
}

impl EndpointKind {
    pub const fn tag(&self) -> &'static [u8] {
        match self {
            Self::Website => b"website",
            Self::PublicApi => b"public-api",
            Self::PublicProxy => b"public-proxy",
            Self::Document => b"document",
            Self::Repository => b"repository",
            Self::Feed => b"feed",
            Self::Socket => b"socket",
            Self::Dataset => b"dataset",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointGrounding {
    pub lane: RegisterLane,
    pub interface_kind: InterfaceKind,
    pub endpoint_kind: EndpointKind,
    pub slot_index: u64,
    pub canonical_address: Vec<u8>,
    pub register_id: RegisterId,
    pub observed_at_unix_millis: u64,
    pub tls_spki_hash: Option<[u8; REGISTER_BYTES]>,
    pub content_address_hint: Option<[u8; REGISTER_BYTES]>,
}

impl EndpointGrounding {
    pub fn new(
        lane: RegisterLane,
        slot_index: u64,
        canonical_address: impl Into<Vec<u8>>,
        interface_kind: InterfaceKind,
        endpoint_kind: EndpointKind,
        node_public_key: &[u8; REGISTER_BYTES],
        observed_at_unix_millis: u64,
    ) -> Result<Self, GraphError> {
        let canonical_address = canonical_address.into();
        if canonical_address.is_empty() {
            return Err(GraphError::EmptyCanonicalAddress);
        }
        let register_id = derive_register_id(
            lane,
            slot_index,
            &canonical_address,
            interface_kind,
            node_public_key,
        );
        Ok(Self {
            lane,
            interface_kind,
            endpoint_kind,
            slot_index,
            canonical_address,
            register_id,
            observed_at_unix_millis,
            tls_spki_hash: None,
            content_address_hint: None,
        })
    }

    pub fn with_tls_spki_hash(mut self, tls_spki_hash: [u8; REGISTER_BYTES]) -> Self {
        self.tls_spki_hash = Some(tls_spki_hash);
        self
    }

    pub fn with_content_address_hint(mut self, content_address_hint: [u8; REGISTER_BYTES]) -> Self {
        self.content_address_hint = Some(content_address_hint);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRange {
    pub start: u64,
    pub end: u64,
}

impl BlockRange {
    pub fn new(start: u64, end: u64) -> Result<Self, GraphError> {
        if end < start {
            return Err(GraphError::InvalidBlockRange { start, end });
        }
        Ok(Self { start, end })
    }

    pub const fn len(&self) -> u64 {
        self.end - self.start
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryBlock {
    pub id: BlockId,
    pub source: EndpointGrounding,
    pub byte_range: BlockRange,
    pub object_path: Option<String>,
    pub content_hash: [u8; REGISTER_BYTES],
    pub content_len: u64,
    pub extracted_features: Register,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub observed_at_unix_millis: u64,
    pub freshness_half_life_millis: u64,
}

impl MemoryBlock {
    pub fn from_content(
        source: EndpointGrounding,
        byte_range: BlockRange,
        object_path: Option<String>,
        content: &[u8],
        extracted_features: Register,
        envelope: PhaseEnvelope,
        freshness_half_life_millis: u64,
    ) -> Result<Self, GraphError> {
        envelope.validate()?;
        if freshness_half_life_millis == 0 {
            return Err(GraphError::InvalidFreshnessHalfLife);
        }
        if byte_range.len() != content.len() as u64 {
            return Err(GraphError::ContentLengthMismatch {
                range_len: byte_range.len(),
                content_len: content.len() as u64,
            });
        }

        let content_hash = *blake3::hash(content).as_bytes();
        let live = block_live_state(
            source.register_id,
            byte_range,
            &object_path,
            &content_hash,
            extracted_features,
            envelope,
        );
        let state = DualState::from_live(live, envelope);
        let id = derive_block_id(
            source.register_id,
            byte_range,
            &object_path,
            &content_hash,
            extracted_features,
            envelope,
        );
        Ok(Self {
            id,
            observed_at_unix_millis: source.observed_at_unix_millis,
            source,
            byte_range,
            object_path,
            content_hash,
            content_len: content.len() as u64,
            extracted_features,
            envelope,
            state,
            freshness_half_life_millis,
        })
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        self.envelope.validate()?;
        if self.freshness_half_life_millis == 0 {
            return Err(GraphError::InvalidFreshnessHalfLife);
        }
        if self.state.twin != phase_project(self.state.live, self.envelope) {
            return Err(GraphError::InvalidTwinProjection);
        }
        let expected_id = derive_block_id(
            self.source.register_id,
            self.byte_range,
            &self.object_path,
            &self.content_hash,
            self.extracted_features,
            self.envelope,
        );
        if expected_id != self.id {
            return Err(GraphError::InvalidBlockIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Mentions,
    Defines,
    Contradicts,
    Implements,
    DependsOn,
    Measures,
    AuthoredBy,
    ServedFrom,
    SchemaOf,
    SimilarTo,
    DerivedFrom,
}

impl EdgeKind {
    pub const fn tag(self) -> &'static [u8] {
        match self {
            Self::Mentions => b"mentions",
            Self::Defines => b"defines",
            Self::Contradicts => b"contradicts",
            Self::Implements => b"implements",
            Self::DependsOn => b"depends-on",
            Self::Measures => b"measures",
            Self::AuthoredBy => b"authored-by",
            Self::ServedFrom => b"served-from",
            Self::SchemaOf => b"schema-of",
            Self::SimilarTo => b"similar-to",
            Self::DerivedFrom => b"derived-from",
        }
    }

    pub const fn is_contradiction(self) -> bool {
        matches!(self, Self::Contradicts)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphNodeRef {
    Block(BlockId),
    Concept(ConceptId),
    Endpoint(RegisterId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightedBlockRef {
    pub block_id: BlockId,
    pub weight: f32,
}

impl WeightedBlockRef {
    pub fn new(block_id: BlockId, weight: f32) -> Result<Self, GraphError> {
        validate_weight(weight, "block membership weight")?;
        Ok(Self { block_id, weight })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightedConceptRef {
    pub concept_id: ConceptId,
    pub weight: f32,
}

impl WeightedConceptRef {
    pub fn new(concept_id: ConceptId, weight: f32) -> Result<Self, GraphError> {
        validate_weight(weight, "concept membership weight")?;
        Ok(Self { concept_id, weight })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConceptLayer {
    pub id: ConceptId,
    pub label: String,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub block_members: Vec<WeightedBlockRef>,
    pub concept_members: Vec<WeightedConceptRef>,
    pub edge_ids: Vec<EdgeId>,
    pub updated_at_unix_millis: u64,
    pub version: u64,
}

impl ConceptLayer {
    pub fn validate(&self) -> Result<(), GraphError> {
        if self.label.trim().is_empty() {
            return Err(GraphError::EmptyConceptLabel);
        }
        self.envelope.validate()?;
        if self.block_members.is_empty() && self.concept_members.is_empty() {
            return Err(GraphError::EmptyConceptLayer);
        }
        for member in &self.block_members {
            validate_weight(member.weight, "block membership weight")?;
        }
        for member in &self.concept_members {
            validate_weight(member.weight, "concept membership weight")?;
        }
        if self.state.twin != phase_project(self.state.live, self.envelope) {
            return Err(GraphError::InvalidTwinProjection);
        }
        if self.id != derive_concept_id(&self.label, self.envelope.coherence_sig) {
            return Err(GraphError::InvalidConceptIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypedEdge {
    pub id: EdgeId,
    pub from: GraphNodeRef,
    pub to: GraphNodeRef,
    pub kind: EdgeKind,
    pub weight: f32,
    pub evidence_blocks: Vec<BlockId>,
    pub envelope: PhaseEnvelope,
    pub state: DualState,
    pub created_at_unix_millis: u64,
}

impl TypedEdge {
    pub fn new(
        from: GraphNodeRef,
        to: GraphNodeRef,
        kind: EdgeKind,
        weight: f32,
        evidence_blocks: Vec<BlockId>,
        envelope: PhaseEnvelope,
        created_at_unix_millis: u64,
    ) -> Result<Self, GraphError> {
        validate_weight(weight, "edge weight")?;
        envelope.validate()?;
        let live = edge_live_state(from, to, kind, weight, &evidence_blocks, envelope);
        let state = DualState::from_live(live, envelope);
        let id = derive_edge_id(from, to, kind, weight, &evidence_blocks, envelope);
        Ok(Self {
            id,
            from,
            to,
            kind,
            weight,
            evidence_blocks,
            envelope,
            state,
            created_at_unix_millis,
        })
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        validate_weight(self.weight, "edge weight")?;
        self.envelope.validate()?;
        if self.state.twin != phase_project(self.state.live, self.envelope) {
            return Err(GraphError::InvalidTwinProjection);
        }
        let expected_id = derive_edge_id(
            self.from,
            self.to,
            self.kind,
            self.weight,
            &self.evidence_blocks,
            self.envelope,
        );
        if expected_id != self.id {
            return Err(GraphError::InvalidEdgeIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionWeights {
    pub semantic: f32,
    pub freshness: f32,
    pub confidence: f32,
    pub graph_support: f32,
    pub contradiction_penalty: f32,
}

impl Default for ProjectionWeights {
    fn default() -> Self {
        Self {
            semantic: 0.52,
            freshness: 0.16,
            confidence: 0.16,
            graph_support: 0.16,
            contradiction_penalty: 0.45,
        }
    }
}

impl ProjectionWeights {
    pub fn validate(&self) -> Result<(), GraphError> {
        for (name, value) in [
            ("semantic", self.semantic),
            ("freshness", self.freshness),
            ("confidence", self.confidence),
            ("graph_support", self.graph_support),
            ("contradiction_penalty", self.contradiction_penalty),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(GraphError::InvalidProjectionWeight(name));
            }
        }
        let positive_sum = self.semantic + self.freshness + self.confidence + self.graph_support;
        if positive_sum <= f32::EPSILON {
            return Err(GraphError::InvalidProjectionWeight("positive weights sum"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionRequest {
    pub query_label: String,
    pub query_state: Register,
    pub reference_time_unix_millis: u64,
    pub limit: usize,
    pub weights: ProjectionWeights,
}

impl ProjectionRequest {
    pub fn from_query_text(
        query_label: impl Into<String>,
        reference_time_unix_millis: u64,
        limit: usize,
        weights: ProjectionWeights,
    ) -> Result<Self, GraphError> {
        let query_label = query_label.into();
        if query_label.trim().is_empty() {
            return Err(GraphError::EmptyQueryLabel);
        }
        Ok(Self {
            query_state: Register::deterministic(query_label.as_bytes()),
            query_label,
            reference_time_unix_millis,
            limit,
            weights,
        })
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        if self.query_label.trim().is_empty() {
            return Err(GraphError::EmptyQueryLabel);
        }
        if self.limit == 0 {
            return Err(GraphError::ProjectionLimitIsZero);
        }
        self.weights.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConceptMatch {
    pub concept_id: ConceptId,
    pub label: String,
    pub score: f32,
    pub semantic: f32,
    pub freshness: f32,
    pub confidence: f32,
    pub contradiction: f32,
    pub graph_support: f32,
    pub grounding_blocks: Vec<BlockId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConceptProjection {
    pub query_label: String,
    pub matches: Vec<ConceptMatch>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphSubscriptionId(u64);

impl GraphSubscriptionId {
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphDeltaKind {
    MemoryBlockInserted,
    ConceptUpserted,
    EdgeInserted,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GraphDelta {
    MemoryBlockInserted {
        block_id: BlockId,
        endpoint_register_id: RegisterId,
        concept_source: Vec<u8>,
        observed_at_unix_millis: u64,
    },
    ConceptUpserted {
        concept_id: ConceptId,
        label: String,
        version: u64,
        updated_at_unix_millis: u64,
    },
    EdgeInserted {
        edge_id: EdgeId,
        from: GraphNodeRef,
        to: GraphNodeRef,
        kind: EdgeKind,
        weight: f32,
        created_at_unix_millis: u64,
    },
}

impl GraphDelta {
    pub const fn kind(&self) -> GraphDeltaKind {
        match self {
            Self::MemoryBlockInserted { .. } => GraphDeltaKind::MemoryBlockInserted,
            Self::ConceptUpserted { .. } => GraphDeltaKind::ConceptUpserted,
            Self::EdgeInserted { .. } => GraphDeltaKind::EdgeInserted,
        }
    }

    pub fn touches_node(&self, node: GraphNodeRef) -> bool {
        match self {
            Self::MemoryBlockInserted { block_id, .. } => node == GraphNodeRef::Block(*block_id),
            Self::ConceptUpserted { concept_id, .. } => node == GraphNodeRef::Concept(*concept_id),
            Self::EdgeInserted { from, to, .. } => *from == node || *to == node,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphSubscriptionFilter {
    pub kinds: Vec<GraphDeltaKind>,
    pub concept_labels: Vec<String>,
    pub touched_nodes: Vec<GraphNodeRef>,
    pub minimum_edge_weight: f32,
}

impl GraphSubscriptionFilter {
    pub fn all() -> Self {
        Self {
            kinds: Vec::new(),
            concept_labels: Vec::new(),
            touched_nodes: Vec::new(),
            minimum_edge_weight: 0.0,
        }
    }

    pub fn for_concept_label(label: impl Into<String>) -> Self {
        Self {
            concept_labels: vec![canonical_label(&label.into())],
            ..Self::all()
        }
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        validate_weight(self.minimum_edge_weight, "minimum edge weight")?;
        Ok(())
    }

    fn matches(&self, delta: &GraphDelta) -> bool {
        if !self.kinds.is_empty() && !self.kinds.contains(&delta.kind()) {
            return false;
        }
        if !self.concept_labels.is_empty() {
            let label_matches = match delta {
                GraphDelta::ConceptUpserted { label, .. } => {
                    self.concept_labels.contains(&canonical_label(label))
                }
                _ => false,
            };
            if !label_matches {
                return false;
            }
        }
        if !self.touched_nodes.is_empty()
            && !self
                .touched_nodes
                .iter()
                .any(|node| delta.touches_node(*node))
        {
            return false;
        }
        if let GraphDelta::EdgeInserted { weight, .. } = delta {
            if *weight < self.minimum_edge_weight {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReactiveNotification {
    pub sequence: u64,
    pub subscription_id: GraphSubscriptionId,
    pub delta: GraphDelta,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReactiveKnowledgeGraph {
    graph: LiveKnowledgeGraph,
    subscriptions: HashMap<GraphSubscriptionId, GraphSubscriptionFilter>,
    pending: HashMap<GraphSubscriptionId, Vec<ReactiveNotification>>,
    next_subscription_id: u64,
    sequence: u64,
}

impl Default for ReactiveKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactiveKnowledgeGraph {
    pub fn new() -> Self {
        Self::from_graph(LiveKnowledgeGraph::new())
    }

    pub fn from_graph(graph: LiveKnowledgeGraph) -> Self {
        Self {
            graph,
            subscriptions: HashMap::new(),
            pending: HashMap::new(),
            next_subscription_id: 0,
            sequence: 0,
        }
    }

    pub fn graph(&self) -> &LiveKnowledgeGraph {
        &self.graph
    }

    pub fn subscribe(
        &mut self,
        filter: GraphSubscriptionFilter,
    ) -> Result<GraphSubscriptionId, GraphError> {
        filter.validate()?;
        let id = GraphSubscriptionId(self.next_subscription_id);
        self.next_subscription_id = self.next_subscription_id.saturating_add(1);
        self.subscriptions.insert(id, filter);
        self.pending.insert(id, Vec::new());
        Ok(id)
    }

    pub fn unsubscribe(&mut self, id: GraphSubscriptionId) -> Result<(), GraphError> {
        if self.subscriptions.remove(&id).is_none() {
            return Err(GraphError::MissingSubscription { id });
        }
        self.pending.remove(&id);
        Ok(())
    }

    pub fn apply_event(&mut self, event: GraphLogEvent) -> Result<GraphDelta, GraphError> {
        let delta = self.delta_for_event(&event)?;
        self.graph.apply_event(event)?;
        self.publish(delta.clone());
        Ok(delta)
    }

    pub fn drain(
        &mut self,
        id: GraphSubscriptionId,
    ) -> Result<Vec<ReactiveNotification>, GraphError> {
        if !self.subscriptions.contains_key(&id) {
            return Err(GraphError::MissingSubscription { id });
        }
        Ok(std::mem::take(self.pending.entry(id).or_default()))
    }

    pub fn pending_len(&self, id: GraphSubscriptionId) -> Result<usize, GraphError> {
        if !self.subscriptions.contains_key(&id) {
            return Err(GraphError::MissingSubscription { id });
        }
        Ok(self.pending.get(&id).map(Vec::len).unwrap_or(0))
    }

    fn publish(&mut self, delta: GraphDelta) {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        for (subscription_id, filter) in &self.subscriptions {
            if filter.matches(&delta) {
                self.pending
                    .entry(*subscription_id)
                    .or_default()
                    .push(ReactiveNotification {
                        sequence,
                        subscription_id: *subscription_id,
                        delta: delta.clone(),
                    });
            }
        }
    }

    fn delta_for_event(&self, event: &GraphLogEvent) -> Result<GraphDelta, GraphError> {
        match event {
            GraphLogEvent::InsertMemoryBlock(block) => {
                block.validate()?;
                Ok(GraphDelta::MemoryBlockInserted {
                    block_id: block.id,
                    endpoint_register_id: block.source.register_id,
                    concept_source: block.source.canonical_address.clone(),
                    observed_at_unix_millis: block.observed_at_unix_millis,
                })
            }
            GraphLogEvent::UpsertConcept {
                label,
                envelope,
                block_members,
                concept_members,
                updated_at_unix_millis,
            } => {
                let mut candidate = self.graph.clone();
                let concept_id = candidate.upsert_concept(
                    label.clone(),
                    *envelope,
                    block_members.clone(),
                    concept_members.clone(),
                    *updated_at_unix_millis,
                )?;
                let concept = candidate
                    .concept(concept_id)
                    .ok_or(GraphError::MissingConcept { id: concept_id })?;
                Ok(GraphDelta::ConceptUpserted {
                    concept_id,
                    label: concept.label.clone(),
                    version: concept.version,
                    updated_at_unix_millis: concept.updated_at_unix_millis,
                })
            }
            GraphLogEvent::InsertEdge(edge) => {
                edge.validate()?;
                Ok(GraphDelta::EdgeInserted {
                    edge_id: edge.id,
                    from: edge.from,
                    to: edge.to,
                    kind: edge.kind,
                    weight: edge.weight,
                    created_at_unix_millis: edge.created_at_unix_millis,
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GraphLogEvent {
    InsertMemoryBlock(MemoryBlock),
    UpsertConcept {
        label: String,
        envelope: PhaseEnvelope,
        block_members: Vec<WeightedBlockRef>,
        concept_members: Vec<WeightedConceptRef>,
        updated_at_unix_millis: u64,
    },
    InsertEdge(TypedEdge),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphLogRecord {
    pub sequence: u64,
    pub previous_hash: [u8; REGISTER_BYTES],
    pub event: GraphLogEvent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphObservationLog {
    records: Vec<GraphLogRecord>,
    head_hash: [u8; REGISTER_BYTES],
}

impl Default for GraphObservationLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphObservationLog {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            head_hash: GENESIS_LOG_HASH,
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub const fn head_hash(&self) -> [u8; REGISTER_BYTES] {
        self.head_hash
    }

    pub fn records(&self) -> &[GraphLogRecord] {
        &self.records
    }

    pub fn append(&mut self, event: GraphLogEvent) -> Result<[u8; REGISTER_BYTES], GraphError> {
        let record = GraphLogRecord {
            sequence: self.records.len() as u64,
            previous_hash: self.head_hash,
            event,
        };
        self.push_record(record)
    }

    fn push_record(&mut self, record: GraphLogRecord) -> Result<[u8; REGISTER_BYTES], GraphError> {
        let expected_sequence = self.records.len() as u64;
        if record.sequence != expected_sequence {
            return Err(GraphError::AppendLogSequence {
                expected: expected_sequence,
                actual: record.sequence,
            });
        }
        if record.previous_hash != self.head_hash {
            return Err(GraphError::AppendLogChainMismatch {
                sequence: record.sequence,
            });
        }
        let next_hash = record_chain_hash(self.head_hash, &record)?;
        self.records.push(record);
        self.head_hash = next_hash;
        Ok(next_hash)
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        let mut head_hash = GENESIS_LOG_HASH;
        for (index, record) in self.records.iter().enumerate() {
            if record.sequence != index as u64 {
                return Err(GraphError::AppendLogSequence {
                    expected: index as u64,
                    actual: record.sequence,
                });
            }
            if record.previous_hash != head_hash {
                return Err(GraphError::AppendLogChainMismatch {
                    sequence: record.sequence,
                });
            }
            head_hash = record_chain_hash(head_hash, record)?;
        }
        if head_hash != self.head_hash {
            return Err(GraphError::AppendLogHeadMismatch);
        }
        Ok(())
    }

    pub fn replay(&self) -> Result<LiveKnowledgeGraph, GraphError> {
        self.validate()?;
        let mut graph = LiveKnowledgeGraph::new();
        for record in &self.records {
            graph.apply_event(record.event.clone())?;
        }
        Ok(graph)
    }
}

#[derive(Debug)]
pub struct AppendOnlyGraphLog {
    path: PathBuf,
    log: GraphObservationLog,
}

impl AppendOnlyGraphLog {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, GraphError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let log = if path.exists() {
            read_log_records(&path)?
        } else {
            File::create(&path)?;
            GraphObservationLog::new()
        };
        Ok(Self { path, log })
    }

    pub fn append(&mut self, event: GraphLogEvent) -> Result<[u8; REGISTER_BYTES], GraphError> {
        let record = GraphLogRecord {
            sequence: self.log.records.len() as u64,
            previous_hash: self.log.head_hash,
            event,
        };
        let encoded = encode_checked(LOG_RECORD_MAGIC, &record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(&encoded)?;
        file.sync_data()?;
        self.log.push_record(record)
    }

    pub fn replay(&self) -> Result<LiveKnowledgeGraph, GraphError> {
        self.log.replay()
    }

    pub fn log(&self) -> &GraphObservationLog {
        &self.log
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct LiveKnowledgeGraph {
    blocks: HashMap<BlockId, MemoryBlock>,
    concepts: HashMap<ConceptId, ConceptLayer>,
    concept_by_label: HashMap<String, ConceptId>,
    edges: HashMap<EdgeId, TypedEdge>,
    edges_by_node: HashMap<GraphNodeRef, Vec<EdgeId>>,
}

impl LiveKnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn block(&self, id: BlockId) -> Option<&MemoryBlock> {
        self.blocks.get(&id)
    }

    pub fn concept(&self, id: ConceptId) -> Option<&ConceptLayer> {
        self.concepts.get(&id)
    }

    pub fn concept_id_for_label(&self, label: &str) -> Option<ConceptId> {
        self.concept_by_label.get(&canonical_label(label)).copied()
    }

    pub fn edge(&self, id: EdgeId) -> Option<&TypedEdge> {
        self.edges.get(&id)
    }

    pub fn insert_memory_block(&mut self, block: MemoryBlock) -> Result<BlockId, GraphError> {
        block.validate()?;
        let id = block.id;
        if let Some(existing) = self.blocks.get(&id) {
            if existing != &block {
                return Err(GraphError::BlockIdentityCollision);
            }
            return Ok(id);
        }
        self.blocks.insert(id, block);
        Ok(id)
    }

    pub fn upsert_concept(
        &mut self,
        label: impl Into<String>,
        envelope: PhaseEnvelope,
        block_members: Vec<WeightedBlockRef>,
        concept_members: Vec<WeightedConceptRef>,
        updated_at_unix_millis: u64,
    ) -> Result<ConceptId, GraphError> {
        let label = label.into();
        let canonical = canonical_label(&label);
        if canonical.is_empty() {
            return Err(GraphError::EmptyConceptLabel);
        }
        envelope.validate()?;
        if block_members.is_empty() && concept_members.is_empty() {
            return Err(GraphError::EmptyConceptLayer);
        }
        for member in &block_members {
            validate_weight(member.weight, "block membership weight")?;
            self.require_block(member.block_id)?;
        }
        for member in &concept_members {
            validate_weight(member.weight, "concept membership weight")?;
            self.require_concept(member.concept_id)?;
        }

        let id = derive_concept_id(&label, envelope.coherence_sig);
        let version = self
            .concepts
            .get(&id)
            .map(|concept| concept.version.saturating_add(1))
            .unwrap_or(0);
        let edge_ids = self
            .concepts
            .get(&id)
            .map(|concept| concept.edge_ids.clone())
            .unwrap_or_default();
        let live = self.build_concept_live_state(&label, &block_members, &concept_members);
        let concept = ConceptLayer {
            id,
            label,
            envelope,
            state: DualState::from_live(live, envelope),
            block_members,
            concept_members,
            edge_ids,
            updated_at_unix_millis,
            version,
        };
        concept.validate()?;
        self.concepts.insert(id, concept);
        self.concept_by_label.insert(canonical, id);
        Ok(id)
    }

    pub fn insert_edge(&mut self, edge: TypedEdge) -> Result<EdgeId, GraphError> {
        edge.validate()?;
        self.require_node(edge.from)?;
        self.require_node(edge.to)?;
        for block_id in &edge.evidence_blocks {
            self.require_block(*block_id)?;
        }
        let id = edge.id;
        if let Some(existing) = self.edges.get(&id) {
            if existing != &edge {
                return Err(GraphError::EdgeIdentityCollision);
            }
            return Ok(id);
        }

        self.edges_by_node.entry(edge.from).or_default().push(id);
        self.edges_by_node.entry(edge.to).or_default().push(id);
        if let GraphNodeRef::Concept(concept_id) = edge.from {
            self.attach_edge_to_concept(concept_id, id)?;
        }
        if let GraphNodeRef::Concept(concept_id) = edge.to {
            self.attach_edge_to_concept(concept_id, id)?;
        }
        self.edges.insert(id, edge);
        Ok(id)
    }

    pub fn apply_event(&mut self, event: GraphLogEvent) -> Result<(), GraphError> {
        match event {
            GraphLogEvent::InsertMemoryBlock(block) => {
                self.insert_memory_block(block)?;
            }
            GraphLogEvent::UpsertConcept {
                label,
                envelope,
                block_members,
                concept_members,
                updated_at_unix_millis,
            } => {
                self.upsert_concept(
                    label,
                    envelope,
                    block_members,
                    concept_members,
                    updated_at_unix_millis,
                )?;
            }
            GraphLogEvent::InsertEdge(edge) => {
                self.insert_edge(edge)?;
            }
        }
        Ok(())
    }

    pub fn project(&self, request: &ProjectionRequest) -> Result<ConceptProjection, GraphError> {
        request.validate()?;
        let mut matches = self
            .concepts
            .values()
            .map(|concept| self.score_concept(concept, request))
            .collect::<Result<Vec<_>, GraphError>>()?;
        matches.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.label.cmp(&right.label))
        });
        matches.truncate(request.limit);
        Ok(ConceptProjection {
            query_label: request.query_label.clone(),
            matches,
        })
    }

    fn score_concept(
        &self,
        concept: &ConceptLayer,
        request: &ProjectionRequest,
    ) -> Result<ConceptMatch, GraphError> {
        let semantic = 1.0 - normalized_hamming(request.query_state, concept.state.live);
        let freshness = self.concept_freshness(concept, request.reference_time_unix_millis)?;
        let confidence = concept.envelope.alpha * concept.envelope.alpha;
        let contradiction = self.contradiction_mass(concept);
        let graph_support = self.graph_support(concept);
        let positive_sum = request.weights.semantic
            + request.weights.freshness
            + request.weights.confidence
            + request.weights.graph_support;
        let positive_score = semantic * request.weights.semantic
            + freshness * request.weights.freshness
            + confidence * request.weights.confidence
            + graph_support * request.weights.graph_support;
        let score = (positive_score / positive_sum
            - contradiction * request.weights.contradiction_penalty)
            .clamp(0.0, 1.0);
        Ok(ConceptMatch {
            concept_id: concept.id,
            label: concept.label.clone(),
            score,
            semantic,
            freshness,
            confidence,
            contradiction,
            graph_support,
            grounding_blocks: self.grounding_blocks_for(concept),
        })
    }

    fn build_concept_live_state(
        &self,
        label: &str,
        block_members: &[WeightedBlockRef],
        concept_members: &[WeightedConceptRef],
    ) -> Register {
        let mut live = Register::deterministic(canonical_label(label).as_bytes());
        for member in block_members {
            if let Some(block) = self.blocks.get(&member.block_id) {
                let contribution =
                    weight_register(block.state.live, member.weight, member.block_id.as_bytes());
                live = live.xor(&contribution);
                let feature_contribution = weight_register(
                    block.extracted_features,
                    member.weight * 0.5,
                    block.content_hash.as_ref(),
                );
                live = live.xor(&feature_contribution);
            }
        }
        for member in concept_members {
            if let Some(concept) = self.concepts.get(&member.concept_id) {
                let contribution = weight_register(
                    concept.state.live,
                    member.weight,
                    member.concept_id.as_bytes(),
                );
                live = live.xor(&contribution);
            }
        }
        live
    }

    fn concept_freshness(
        &self,
        concept: &ConceptLayer,
        reference_time_unix_millis: u64,
    ) -> Result<f32, GraphError> {
        let mut weighted_total = 0.0;
        let mut weight_sum = 0.0;
        for member in &concept.block_members {
            let block = self.require_block(member.block_id)?;
            let age = reference_time_unix_millis.saturating_sub(block.observed_at_unix_millis);
            let age_ratio = age as f32 / block.freshness_half_life_millis as f32;
            weighted_total += member.weight * (1.0 / (1.0 + age_ratio));
            weight_sum += member.weight;
        }
        for member in &concept.concept_members {
            let nested = self.require_concept(member.concept_id)?;
            let age = reference_time_unix_millis.saturating_sub(nested.updated_at_unix_millis);
            let age_ratio = age as f32 / 86_400_000.0;
            weighted_total += member.weight * (1.0 / (1.0 + age_ratio));
            weight_sum += member.weight;
        }
        if weight_sum <= f32::EPSILON {
            Ok(0.0)
        } else {
            Ok((weighted_total / weight_sum).clamp(0.0, 1.0))
        }
    }

    fn contradiction_mass(&self, concept: &ConceptLayer) -> f32 {
        let envelope_mass = concept.envelope.beta * concept.envelope.beta;
        let edge_mass = self
            .edges_by_node
            .get(&GraphNodeRef::Concept(concept.id))
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|edge_id| self.edges.get(edge_id))
                    .filter(|edge| edge.kind.is_contradiction())
                    .map(|edge| edge.weight * edge.envelope.beta.max(0.0))
                    .fold(0.0f32, f32::max)
            })
            .unwrap_or(0.0);
        envelope_mass.max(edge_mass).clamp(0.0, 1.0)
    }

    fn graph_support(&self, concept: &ConceptLayer) -> f32 {
        let edge_count = self
            .edges_by_node
            .get(&GraphNodeRef::Concept(concept.id))
            .map(Vec::len)
            .unwrap_or(0);
        let support_units =
            concept.block_members.len() + concept.concept_members.len() + edge_count;
        1.0 - 1.0 / (1.0 + support_units as f32)
    }

    fn grounding_blocks_for(&self, concept: &ConceptLayer) -> Vec<BlockId> {
        let mut seen = HashSet::new();
        let mut blocks = Vec::new();
        for member in &concept.block_members {
            if seen.insert(member.block_id) {
                blocks.push(member.block_id);
            }
        }
        if let Some(edge_ids) = self.edges_by_node.get(&GraphNodeRef::Concept(concept.id)) {
            for edge_id in edge_ids {
                if let Some(edge) = self.edges.get(edge_id) {
                    for block_id in &edge.evidence_blocks {
                        if seen.insert(*block_id) {
                            blocks.push(*block_id);
                        }
                    }
                }
            }
        }
        blocks
    }

    fn attach_edge_to_concept(
        &mut self,
        concept_id: ConceptId,
        edge_id: EdgeId,
    ) -> Result<(), GraphError> {
        let concept = self.require_concept_mut(concept_id)?;
        if !concept.edge_ids.contains(&edge_id) {
            concept.edge_ids.push(edge_id);
        }
        Ok(())
    }

    fn require_node(&self, node: GraphNodeRef) -> Result<(), GraphError> {
        match node {
            GraphNodeRef::Block(block_id) => {
                self.require_block(block_id)?;
            }
            GraphNodeRef::Concept(concept_id) => {
                self.require_concept(concept_id)?;
            }
            GraphNodeRef::Endpoint(_) => {}
        }
        Ok(())
    }

    fn require_block(&self, id: BlockId) -> Result<&MemoryBlock, GraphError> {
        self.blocks.get(&id).ok_or(GraphError::MissingBlock { id })
    }

    fn require_concept(&self, id: ConceptId) -> Result<&ConceptLayer, GraphError> {
        self.concepts
            .get(&id)
            .ok_or(GraphError::MissingConcept { id })
    }

    fn require_concept_mut(&mut self, id: ConceptId) -> Result<&mut ConceptLayer, GraphError> {
        self.concepts
            .get_mut(&id)
            .ok_or(GraphError::MissingConcept { id })
    }
}

fn read_log_records(path: &Path) -> Result<GraphObservationLog, GraphError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut offset = 0usize;
    let mut log = GraphObservationLog::new();
    while offset < bytes.len() {
        let (record, consumed) =
            decode_checked_with_len::<GraphLogRecord>(LOG_RECORD_MAGIC, &bytes[offset..])?;
        log.push_record(record)?;
        offset = offset
            .checked_add(consumed)
            .ok_or(GraphError::InvalidPersistentHeader)?;
    }
    Ok(log)
}

fn validate_weight(weight: f32, name: &'static str) -> Result<(), GraphError> {
    if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
        return Err(GraphError::InvalidWeight { name, weight });
    }
    Ok(())
}

fn canonical_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn derive_concept_id(label: &str, coherence_sig: u64) -> ConceptId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-concept-layer");
    hasher.update(canonical_label(label).as_bytes());
    hasher.update(&coherence_sig.to_le_bytes());
    ConceptId(*hasher.finalize().as_bytes())
}

fn derive_block_id(
    source_register_id: RegisterId,
    byte_range: BlockRange,
    object_path: &Option<String>,
    content_hash: &[u8; REGISTER_BYTES],
    extracted_features: Register,
    envelope: PhaseEnvelope,
) -> BlockId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-memory-block");
    hasher.update(source_register_id.as_bytes());
    hasher.update(&byte_range.start.to_le_bytes());
    hasher.update(&byte_range.end.to_le_bytes());
    if let Some(object_path) = object_path {
        hasher.update(object_path.as_bytes());
    }
    hasher.update(content_hash);
    hasher.update(extracted_features.as_bytes());
    hasher.update(&envelope.coherence_sig.to_le_bytes());
    hasher.update(&envelope.alpha.to_bits().to_le_bytes());
    hasher.update(&envelope.beta.to_bits().to_le_bytes());
    hasher.update(&envelope.resuperposition_n.to_le_bytes());
    BlockId(*hasher.finalize().as_bytes())
}

fn block_live_state(
    source_register_id: RegisterId,
    byte_range: BlockRange,
    object_path: &Option<String>,
    content_hash: &[u8; REGISTER_BYTES],
    extracted_features: Register,
    envelope: PhaseEnvelope,
) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-memory-block-live");
    hasher.update(source_register_id.as_bytes());
    hasher.update(&byte_range.start.to_le_bytes());
    hasher.update(&byte_range.end.to_le_bytes());
    if let Some(object_path) = object_path {
        hasher.update(object_path.as_bytes());
    }
    hasher.update(content_hash);
    hasher.update(extracted_features.as_bytes());
    hasher.update(&envelope.coherence_sig.to_le_bytes());
    Register::from_bytes(*hasher.finalize().as_bytes()).xor(&extracted_features)
}

fn derive_edge_id(
    from: GraphNodeRef,
    to: GraphNodeRef,
    kind: EdgeKind,
    weight: f32,
    evidence_blocks: &[BlockId],
    envelope: PhaseEnvelope,
) -> EdgeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-typed-edge");
    hash_node_ref(&mut hasher, from);
    hash_node_ref(&mut hasher, to);
    hasher.update(kind.tag());
    hasher.update(&weight.to_bits().to_le_bytes());
    for block_id in evidence_blocks {
        hasher.update(block_id.as_bytes());
    }
    hasher.update(&envelope.coherence_sig.to_le_bytes());
    hasher.update(&envelope.alpha.to_bits().to_le_bytes());
    hasher.update(&envelope.beta.to_bits().to_le_bytes());
    hasher.update(&envelope.resuperposition_n.to_le_bytes());
    EdgeId(*hasher.finalize().as_bytes())
}

fn edge_live_state(
    from: GraphNodeRef,
    to: GraphNodeRef,
    kind: EdgeKind,
    weight: f32,
    evidence_blocks: &[BlockId],
    envelope: PhaseEnvelope,
) -> Register {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-typed-edge-live");
    hash_node_ref(&mut hasher, from);
    hash_node_ref(&mut hasher, to);
    hasher.update(kind.tag());
    hasher.update(&weight.to_bits().to_le_bytes());
    hasher.update(&envelope.coherence_sig.to_le_bytes());
    for block_id in evidence_blocks {
        hasher.update(block_id.as_bytes());
    }
    Register::from_bytes(*hasher.finalize().as_bytes())
}

fn hash_node_ref(hasher: &mut blake3::Hasher, node: GraphNodeRef) {
    match node {
        GraphNodeRef::Block(id) => {
            hasher.update(b"block");
            hasher.update(id.as_bytes());
        }
        GraphNodeRef::Concept(id) => {
            hasher.update(b"concept");
            hasher.update(id.as_bytes());
        }
        GraphNodeRef::Endpoint(id) => {
            hasher.update(b"endpoint");
            hasher.update(id.as_bytes());
        }
    }
}

fn weight_register(register: Register, weight: f32, salt: &[u8]) -> Register {
    if weight <= 0.0 {
        return Register::zero();
    }
    if weight >= 1.0 {
        return register;
    }
    register.and(&weighted_mask(weight, salt))
}

fn weighted_mask(weight: f32, salt: &[u8]) -> Register {
    let target_bits = (weight.clamp(0.0, 1.0) * REGISTER_BITS as f32).round() as usize;
    if target_bits == 0 {
        return Register::zero();
    }
    if target_bits >= REGISTER_BITS {
        return Register::all_ones();
    }

    let mut scored_bits = Vec::with_capacity(REGISTER_BITS);
    for bit_index in 0..REGISTER_BITS {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bqip-weight-mask");
        hasher.update(salt);
        hasher.update(&bit_index.to_le_bytes());
        let hash = hasher.finalize();
        let mut score = [0u8; 8];
        score.copy_from_slice(&hash.as_bytes()[..8]);
        scored_bits.push((u64::from_le_bytes(score), bit_index));
    }
    scored_bits.sort_unstable_by_key(|(score, bit_index)| (*score, *bit_index));

    let mut bytes = [0u8; REGISTER_BYTES];
    for (_, bit_index) in scored_bits.into_iter().take(target_bits) {
        let byte_index = bit_index / 8;
        let bit_in_byte = 7 - (bit_index % 8);
        bytes[byte_index] |= 1 << bit_in_byte;
    }
    Register::from_bytes(bytes)
}

fn normalized_hamming(left: Register, right: Register) -> f32 {
    let distance = left
        .as_bytes()
        .iter()
        .zip(right.as_bytes())
        .map(|(left, right)| (left ^ right).count_ones())
        .sum::<u32>();
    distance as f32 / REGISTER_BITS as f32
}

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("canonical endpoint address must be nonempty")]
    EmptyCanonicalAddress,
    #[error("concept label must be nonempty")]
    EmptyConceptLabel,
    #[error("query label must be nonempty")]
    EmptyQueryLabel,
    #[error("concept layer must contain at least one block or concept member")]
    EmptyConceptLayer,
    #[error("projection limit must be greater than zero")]
    ProjectionLimitIsZero,
    #[error("block range end {end} is before start {start}")]
    InvalidBlockRange { start: u64, end: u64 },
    #[error("content length {content_len} does not match block range length {range_len}")]
    ContentLengthMismatch { range_len: u64, content_len: u64 },
    #[error("freshness half-life must be greater than zero milliseconds")]
    InvalidFreshnessHalfLife,
    #[error("dual-state twin projection does not match phase envelope")]
    InvalidTwinProjection,
    #[error("memory block identity does not match deterministic derivation")]
    InvalidBlockIdentity,
    #[error("concept identity does not match deterministic derivation")]
    InvalidConceptIdentity,
    #[error("edge identity does not match deterministic derivation")]
    InvalidEdgeIdentity,
    #[error("memory block identity collision")]
    BlockIdentityCollision,
    #[error("edge identity collision")]
    EdgeIdentityCollision,
    #[error("missing memory block {id:?}")]
    MissingBlock { id: BlockId },
    #[error("missing concept {id:?}")]
    MissingConcept { id: ConceptId },
    #[error("invalid {name}: {weight}")]
    InvalidWeight { name: &'static str, weight: f32 },
    #[error("invalid projection weight: {0}")]
    InvalidProjectionWeight(&'static str),
    #[error("persistent graph log header is invalid")]
    InvalidPersistentHeader,
    #[error("persistent graph log capacity {capacity} bytes cannot hold {required} bytes")]
    PersistentCapacity { capacity: usize, required: usize },
    #[error("persistent graph log payload hash mismatch")]
    PersistentHashMismatch,
    #[error("append log sequence mismatch: expected {expected}, got {actual}")]
    AppendLogSequence { expected: u64, actual: u64 },
    #[error("append log hash chain mismatch at sequence {sequence}")]
    AppendLogChainMismatch { sequence: u64 },
    #[error("append log head hash mismatch")]
    AppendLogHeadMismatch,
    #[error("missing graph subscription {id:?}")]
    MissingSubscription { id: GraphSubscriptionId },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Codec(#[from] Box<bincode::ErrorKind>),
    #[error(transparent)]
    Core(#[from] bqip_core::CoreError),
}
