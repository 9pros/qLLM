use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::io::{Cursor, Read};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, Method, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use bqip_control::{
    ControlAction, ConversationMessage, ConversationRole, NaturalLanguageTrainingController,
    TrainingControlConfig,
};
use bqip_core::{PhaseEnvelope, REGISTER_BYTES};
use bqip_ingestor::resolve_huggingface_registers;
use bqip_multimodal::{analyze_frame, PixelFormat, VisualFrame};
use lopdf::content::Content;
use lopdf::Object;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use zip::ZipArchive;

pub const MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_DOCUMENT_TEXT_CHARS: usize = 262_144;
pub const DEFAULT_MAX_TOOL_CALLS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: MessageContent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    pub fn text_fragments(&self) -> Vec<String> {
        match self {
            Self::Text(text) => vec![text.clone()],
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect(),
        }
    }

    pub fn upload_refs(&self) -> Vec<String> {
        match self {
            Self::Text(_) => Vec::new(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::UploadRef { upload_id } => Some(upload_id.clone()),
                    _ => None,
                })
                .collect(),
        }
    }

    pub fn tool_calls(&self) -> Vec<ToolInvocation> {
        match self {
            Self::Text(_) => Vec::new(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::ToolCall { name, arguments } => Some(ToolInvocation {
                        name: name.clone(),
                        arguments: arguments.clone(),
                    }),
                    _ => None,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    UploadRef { upload_id: String },
    ToolCall { name: String, arguments: Value },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub upload_ids: Vec<String>,
    #[serde(default)]
    pub action_mode: ActionMode,
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: usize,
}

fn default_max_tool_calls() -> usize {
    DEFAULT_MAX_TOOL_CALLS
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMode {
    #[default]
    Inline,
    LogOnly,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub rich_content: Vec<RichBlock>,
    pub actions: Vec<ActionEvent>,
    pub tool_calls: Vec<ToolCallView>,
    pub usage: Usage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: usize,
    pub message: AssistantMessage,
    pub finish_reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichBlock {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    Bullets { items: Vec<String> },
    CodeBlock { language: String, code: String },
    ToolCard { tool: ToolCallView },
    UploadCard { upload: UploadCard },
    ActionTimeline { events: Vec<ActionEvent> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UploadCard {
    pub upload_id: String,
    pub filename: String,
    pub mime_type: String,
    pub kind: UploadKind,
    pub summary: String,
    pub size_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub name: String,
    pub arguments: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCallView {
    pub id: String,
    pub name: String,
    pub input: Value,
    pub status: ToolStatus,
    pub output: Value,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEvent {
    pub id: String,
    pub kind: String,
    pub status: ActionStatus,
    pub title: String,
    pub detail: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Started,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    ResponseStarted {
        response_id: String,
        created: u64,
        model: String,
    },
    RichBlockDelta {
        response_id: String,
        index: usize,
        block: RichBlock,
    },
    ToolCallDelta {
        response_id: String,
        tool_call: ToolCallView,
    },
    ActionDelta {
        response_id: String,
        action: ActionEvent,
    },
    ResponseCompleted {
        response: ChatCompletionResponse,
    },
    Error {
        message: String,
    },
}

impl ChatStreamEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::ResponseStarted { .. } => "response.started",
            Self::RichBlockDelta { .. } => "response.rich_delta",
            Self::ToolCallDelta { .. } => "response.tool_call",
            Self::ActionDelta { .. } => "response.action",
            Self::ResponseCompleted { .. } => "response.completed",
            Self::Error { .. } => "response.error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadKind {
    Document,
    Image,
    Binary,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UploadRecord {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub content_hash: [u8; REGISTER_BYTES],
    pub uploaded_at_ms: u64,
    pub kind: UploadKind,
    pub analysis: UploadAnalysis,
}

impl UploadRecord {
    pub fn card(&self) -> UploadCard {
        UploadCard {
            upload_id: self.id.clone(),
            filename: self.filename.clone(),
            mime_type: self.mime_type.clone(),
            kind: self.kind.clone(),
            summary: self.analysis.summary(),
            size_bytes: self.size_bytes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UploadAnalysis {
    Document {
        text: String,
        summary: String,
        word_count: usize,
        char_count: usize,
    },
    Image {
        width: usize,
        height: usize,
        mean_luma: f32,
        contrast: f32,
        entropy: f32,
        frame_hash: [u8; REGISTER_BYTES],
        feature_hash: [u8; REGISTER_BYTES],
        summary: String,
    },
    Binary {
        summary: String,
    },
}

impl UploadAnalysis {
    pub fn summary(&self) -> String {
        match self {
            Self::Document { summary, .. }
            | Self::Image { summary, .. }
            | Self::Binary { summary } => summary.clone(),
        }
    }

    pub fn searchable_text(&self) -> String {
        match self {
            Self::Document { text, summary, .. } => format!("{summary}\n{text}"),
            Self::Image { summary, .. } | Self::Binary { summary } => summary.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UploadResponse {
    pub uploads: Vec<UploadRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub uploads: usize,
    pub actions: usize,
    pub built_in_tools: Vec<ToolSpec>,
}

pub struct ChatEngine {
    uploads: HashMap<String, UploadRecord>,
    actions: Vec<ActionEvent>,
    controller: NaturalLanguageTrainingController,
    node_public_key: [u8; REGISTER_BYTES],
    sequence: u64,
}

impl ChatEngine {
    pub fn new(config: TrainingControlConfig) -> Result<Self, ChatError> {
        let node_public_key = config.node_public_key;
        Ok(Self {
            uploads: HashMap::new(),
            actions: Vec::new(),
            controller: NaturalLanguageTrainingController::new(config)?,
            node_public_key,
            sequence: 0,
        })
    }

    pub fn uploads(&self) -> &HashMap<String, UploadRecord> {
        &self.uploads
    }

    pub fn actions(&self) -> &[ActionEvent] {
        &self.actions
    }

    pub fn built_in_tools() -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "calculator".to_string(),
                description: "Evaluate deterministic arithmetic expressions with +, -, *, /, unary signs, and parentheses.".to_string(),
                input_schema: json!({"type":"object","properties":{"expression":{"type":"string"}},"required":["expression"]}),
            },
            ToolSpec {
                name: "search_uploads".to_string(),
                description: "Search uploaded documents, image summaries, and binary summaries.".to_string(),
                input_schema: json!({"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer"}},"required":["query"]}),
            },
            ToolSpec {
                name: "summarize_upload".to_string(),
                description: "Return the structured summary and metadata for one uploaded file.".to_string(),
                input_schema: json!({"type":"object","properties":{"upload_id":{"type":"string"}},"required":["upload_id"]}),
            },
            ToolSpec {
                name: "analyze_image".to_string(),
                description: "Return image feature statistics generated by the BQIP multimodal analyzer.".to_string(),
                input_schema: json!({"type":"object","properties":{"upload_id":{"type":"string"}},"required":["upload_id"]}),
            },
            ToolSpec {
                name: "training_control".to_string(),
                description: "Route natural-language training commands through the BQIP self-training controller.".to_string(),
                input_schema: json!({"type":"object","properties":{"instruction":{"type":"string"}},"required":["instruction"]}),
            },
            ToolSpec {
                name: "resolve_huggingface_registers".to_string(),
                description: "Resolve Hugging Face public endpoints into IPv4 and IPv6 register lane observations.".to_string(),
                input_schema: json!({"type":"object","properties":{}}),
            },
        ]
    }

    pub fn upload_bytes(
        &mut self,
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        bytes: &[u8],
    ) -> Result<UploadRecord, ChatError> {
        if bytes.is_empty() {
            return Err(ChatError::EmptyUpload);
        }
        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(ChatError::UploadTooLarge {
                limit: MAX_UPLOAD_BYTES,
                actual: bytes.len(),
            });
        }
        let filename = filename.into();
        if filename.trim().is_empty() {
            return Err(ChatError::InvalidUploadName);
        }
        let mime_type = mime_type.into();
        let uploaded_at_ms = unix_millis()?;
        let content_hash = *blake3::hash(bytes).as_bytes();
        let id = self.next_stable_id("upload", &[filename.as_bytes(), &content_hash]);
        let (kind, analysis) = analyze_upload(&filename, &mime_type, bytes, uploaded_at_ms)?;
        let record = UploadRecord {
            id: id.clone(),
            filename,
            mime_type,
            size_bytes: bytes.len(),
            content_hash,
            uploaded_at_ms,
            kind,
            analysis,
        };
        self.uploads.insert(id.clone(), record.clone());
        self.record_action(
            "upload",
            ActionStatus::Completed,
            "Upload analyzed",
            format!("{} bytes stored as {}", record.size_bytes, record.id),
        )?;
        Ok(record)
    }

    pub fn complete(
        &mut self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ChatError> {
        request.validate()?;
        let action_start = self.actions.len();
        let created = unix_millis()?;
        let response_id = self.next_stable_id(
            "chatcmpl",
            &[request.model.as_bytes(), &created.to_le_bytes()],
        );
        self.record_action(
            "chat",
            ActionStatus::Started,
            "Chat completion started",
            format!(
                "model={} messages={}",
                request.model,
                request.messages.len()
            ),
        )?;

        let prompt_text = collect_prompt_text(&request.messages);
        let prompt_tokens = count_tokens(&prompt_text);
        let referenced_uploads = self.collect_uploads(&request)?;
        let allowed_tools = allowed_tool_names(&request.tools);
        let requested_tools = self.collect_tool_invocations(&request, &prompt_text)?;

        let mut rich_content = vec![RichBlock::Heading {
            level: 2,
            text: "Completion".to_string(),
        }];
        if prompt_text.trim().is_empty() {
            rich_content.push(RichBlock::Paragraph {
                text: "The request contains upload references and tool actions without free-form text.".to_string(),
            });
        } else {
            rich_content.push(RichBlock::Paragraph {
                text: response_lead(
                    &prompt_text,
                    referenced_uploads.len(),
                    requested_tools.len(),
                ),
            });
        }
        for upload in &referenced_uploads {
            rich_content.push(RichBlock::UploadCard {
                upload: upload.card(),
            });
        }

        let mut tool_calls = Vec::new();
        for invocation in requested_tools {
            if tool_calls.len() >= request.max_tool_calls {
                self.record_action(
                    "tool",
                    ActionStatus::Failed,
                    "Tool budget exhausted",
                    format!("max_tool_calls={}", request.max_tool_calls),
                )?;
                break;
            }
            let tool_call = self.execute_tool(invocation, &allowed_tools)?;
            rich_content.push(RichBlock::ToolCard {
                tool: tool_call.clone(),
            });
            tool_calls.push(tool_call);
        }

        let synthesis_items = synthesis_items(&referenced_uploads, &tool_calls);
        if !synthesis_items.is_empty() {
            rich_content.push(RichBlock::Bullets {
                items: synthesis_items,
            });
        }

        self.record_action(
            "chat",
            ActionStatus::Completed,
            "Chat completion completed",
            format!(
                "rich_blocks={} tool_calls={}",
                rich_content.len(),
                tool_calls.len()
            ),
        )?;
        let mut response_actions = self.actions[action_start..].to_vec();
        if request.action_mode == ActionMode::Inline {
            rich_content.push(RichBlock::ActionTimeline {
                events: response_actions.clone(),
            });
        } else {
            response_actions.clear();
        }
        let markdown = render_markdown(&rich_content)?;
        let completion_tokens = count_tokens(&markdown);
        Ok(ChatCompletionResponse {
            id: response_id,
            object: "chat.completion".to_string(),
            created,
            model: request.model,
            choices: vec![ChatChoice {
                index: 0,
                message: AssistantMessage {
                    role: "assistant".to_string(),
                    content: markdown,
                },
                finish_reason: "stop".to_string(),
            }],
            rich_content,
            actions: response_actions,
            tool_calls,
            usage: Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens.saturating_add(completion_tokens),
            },
        })
    }

    pub fn stream_events(
        &mut self,
        request: ChatCompletionRequest,
    ) -> Result<Vec<ChatStreamEvent>, ChatError> {
        let response = self.complete(request)?;
        let mut events = Vec::with_capacity(
            1 + response.rich_content.len()
                + response.tool_calls.len()
                + response.actions.len()
                + 1,
        );
        events.push(ChatStreamEvent::ResponseStarted {
            response_id: response.id.clone(),
            created: response.created,
            model: response.model.clone(),
        });
        for (index, block) in response.rich_content.iter().cloned().enumerate() {
            events.push(ChatStreamEvent::RichBlockDelta {
                response_id: response.id.clone(),
                index,
                block,
            });
        }
        for tool_call in response.tool_calls.iter().cloned() {
            events.push(ChatStreamEvent::ToolCallDelta {
                response_id: response.id.clone(),
                tool_call,
            });
        }
        for action in response.actions.iter().cloned() {
            events.push(ChatStreamEvent::ActionDelta {
                response_id: response.id.clone(),
                action,
            });
        }
        events.push(ChatStreamEvent::ResponseCompleted { response });
        Ok(events)
    }

    fn collect_uploads(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<Vec<UploadRecord>, ChatError> {
        let mut ids = request.upload_ids.clone();
        for message in &request.messages {
            ids.extend(message.content.upload_refs());
        }
        let mut seen = HashSet::new();
        let mut uploads = Vec::new();
        for id in ids {
            if seen.insert(id.clone()) {
                let upload = self
                    .uploads
                    .get(&id)
                    .ok_or_else(|| ChatError::MissingUpload(id.clone()))?;
                uploads.push(upload.clone());
            }
        }
        Ok(uploads)
    }

    fn collect_tool_invocations(
        &self,
        request: &ChatCompletionRequest,
        prompt_text: &str,
    ) -> Result<Vec<ToolInvocation>, ChatError> {
        let mut invocations = parse_inline_tools(prompt_text)?;
        for message in &request.messages {
            invocations.extend(message.content.tool_calls());
        }
        Ok(invocations)
    }

    fn execute_tool(
        &mut self,
        invocation: ToolInvocation,
        allowed_tools: &HashSet<String>,
    ) -> Result<ToolCallView, ChatError> {
        let started_at_ms = unix_millis()?;
        let call_id = self.next_stable_id("toolcall", &[invocation.name.as_bytes()]);
        self.record_action(
            "tool",
            ActionStatus::Started,
            format!("Tool `{}` started", invocation.name),
            compact_json(&invocation.arguments),
        )?;
        let allowed = allowed_tools.is_empty() || allowed_tools.contains(&invocation.name);
        let result = if allowed {
            self.execute_allowed_tool(&invocation.name, &invocation.arguments)
        } else {
            Err(ChatError::ToolNotAllowed(invocation.name.clone()))
        };
        let completed_at_ms = unix_millis()?;
        match result {
            Ok(output) => {
                self.record_action(
                    "tool",
                    ActionStatus::Completed,
                    format!("Tool `{}` completed", invocation.name),
                    compact_json(&output),
                )?;
                Ok(ToolCallView {
                    id: call_id,
                    name: invocation.name,
                    input: invocation.arguments,
                    status: ToolStatus::Completed,
                    output,
                    started_at_ms,
                    completed_at_ms,
                })
            }
            Err(error) => {
                let message = error.to_string();
                self.record_action(
                    "tool",
                    ActionStatus::Failed,
                    format!("Tool `{}` failed", invocation.name),
                    message.clone(),
                )?;
                Ok(ToolCallView {
                    id: call_id,
                    name: invocation.name,
                    input: invocation.arguments,
                    status: ToolStatus::Failed,
                    output: json!({ "error": message }),
                    started_at_ms,
                    completed_at_ms,
                })
            }
        }
    }

    fn execute_allowed_tool(&mut self, name: &str, arguments: &Value) -> Result<Value, ChatError> {
        match name {
            "calculator" => {
                let expression = string_argument(arguments, "expression")?;
                let value = Calculator::new(&expression).parse()?;
                Ok(json!({
                    "expression": expression,
                    "value": value,
                    "display": format_number(value)
                }))
            }
            "search_uploads" => {
                let query = string_argument(arguments, "query")?;
                let limit = usize_argument(arguments, "limit").unwrap_or(8).clamp(1, 32);
                Ok(json!({
                    "query": query,
                    "matches": self.search_uploads(&query, limit)
                }))
            }
            "summarize_upload" => {
                let upload_id = string_argument(arguments, "upload_id")?;
                let upload = self
                    .uploads
                    .get(&upload_id)
                    .ok_or_else(|| ChatError::MissingUpload(upload_id.clone()))?;
                Ok(json!({
                    "upload_id": upload.id,
                    "filename": upload.filename,
                    "mime_type": upload.mime_type,
                    "kind": upload.kind,
                    "size_bytes": upload.size_bytes,
                    "summary": upload.analysis.summary()
                }))
            }
            "analyze_image" => {
                let upload_id = string_argument(arguments, "upload_id")?;
                let upload = self
                    .uploads
                    .get(&upload_id)
                    .ok_or_else(|| ChatError::MissingUpload(upload_id.clone()))?;
                match &upload.analysis {
                    UploadAnalysis::Image {
                        width,
                        height,
                        mean_luma,
                        contrast,
                        entropy,
                        frame_hash,
                        feature_hash,
                        summary,
                    } => Ok(json!({
                        "upload_id": upload.id,
                        "filename": upload.filename,
                        "width": width,
                        "height": height,
                        "mean_luma": mean_luma,
                        "contrast": contrast,
                        "entropy": entropy,
                        "frame_hash": hex32(frame_hash),
                        "feature_hash": hex32(feature_hash),
                        "summary": summary
                    })),
                    _ => Err(ChatError::UploadIsNotImage(upload_id)),
                }
            }
            "training_control" => {
                let instruction = string_argument(arguments, "instruction")?;
                let message = ConversationMessage::new(
                    ConversationRole::Operator,
                    instruction.clone(),
                    unix_millis()?,
                )?;
                let turn = self.controller.process(message)?;
                Ok(json!({
                    "instruction": instruction,
                    "reply": turn.reply,
                    "actions": control_actions_json(&turn.actions),
                    "training_report": turn.training_report.as_ref().map(|report| json!({
                        "train_initial_loss": report.report.train.initial_loss,
                        "train_final_loss": report.report.train.final_loss,
                        "validation_mean_loss": report.report.validation.as_ref().map(|metrics| metrics.mean_loss),
                        "test_mean_loss": report.report.test.as_ref().map(|metrics| metrics.mean_loss)
                    }))
                }))
            }
            "resolve_huggingface_registers" => {
                let observed_at = unix_millis()?;
                let report = resolve_huggingface_registers(&self.node_public_key, observed_at)?;
                let registers = report
                    .registers
                    .iter()
                    .take(64)
                    .map(|register| {
                        json!({
                            "hostname": register.hostname,
                            "ip_address": register.ip_address,
                            "port": register.port,
                            "lane": format!("{:?}", register.lane),
                            "endpoint_kind": format!("{:?}", register.endpoint_kind),
                            "register_id": hex32(register.register_id.as_bytes())
                        })
                    })
                    .collect::<Vec<_>>();
                Ok(json!({
                    "observed_at_unix_millis": report.observed_at_unix_millis,
                    "target_count": report.target_count,
                    "resolved_register_count": report.registers.len(),
                    "ipv4_count": report.ipv4_count(),
                    "ipv6_count": report.ipv6_count(),
                    "failure_count": report.failures.len(),
                    "registers": registers
                }))
            }
            _ => Err(ChatError::UnknownTool(name.to_string())),
        }
    }

    fn search_uploads(&self, query: &str, limit: usize) -> Vec<Value> {
        let terms = query
            .to_ascii_lowercase()
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let mut matches = self
            .uploads
            .values()
            .filter_map(|upload| {
                let haystack = format!(
                    "{}\n{}\n{}",
                    upload.filename,
                    upload.mime_type,
                    upload.analysis.searchable_text()
                )
                .to_ascii_lowercase();
                let score = terms
                    .iter()
                    .filter(|term| haystack.contains(term.as_str()))
                    .count();
                (score > 0).then(|| (score, upload))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.filename.cmp(&right.1.filename))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|(score, upload)| {
                json!({
                    "upload_id": upload.id,
                    "filename": upload.filename,
                    "kind": upload.kind,
                    "score": score,
                    "summary": upload.analysis.summary()
                })
            })
            .collect()
    }

    fn record_action(
        &mut self,
        kind: impl Into<String>,
        status: ActionStatus,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Result<ActionEvent, ChatError> {
        let timestamp_ms = unix_millis()?;
        let id = self.next_stable_id("action", &[&timestamp_ms.to_le_bytes()]);
        let event = ActionEvent {
            id,
            kind: kind.into(),
            status,
            title: title.into(),
            detail: detail.into(),
            timestamp_ms,
        };
        self.actions.push(event.clone());
        Ok(event)
    }

    fn next_stable_id(&mut self, prefix: &str, parts: &[&[u8]]) -> String {
        self.sequence = self.sequence.saturating_add(1);
        let mut hasher = blake3::Hasher::new();
        hasher.update(prefix.as_bytes());
        hasher.update(&self.sequence.to_le_bytes());
        for part in parts {
            hasher.update(part);
        }
        let digest = hasher.finalize();
        format!("{prefix}-{}", hex_prefix(digest.as_bytes(), 16))
    }
}

impl ChatCompletionRequest {
    pub fn validate(&self) -> Result<(), ChatError> {
        if self.model.trim().is_empty() {
            return Err(ChatError::EmptyModel);
        }
        if self.messages.is_empty() {
            return Err(ChatError::EmptyMessages);
        }
        if self.max_tool_calls == 0 {
            return Err(ChatError::InvalidToolBudget);
        }
        for message in &self.messages {
            if message
                .content
                .text_fragments()
                .iter()
                .all(|text| text.trim().is_empty())
                && message.content.upload_refs().is_empty()
                && message.content.tool_calls().is_empty()
            {
                return Err(ChatError::EmptyMessage);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ChatState {
    engine: Arc<Mutex<ChatEngine>>,
}

impl ChatState {
    pub fn new(config: TrainingControlConfig) -> Result<Self, ChatError> {
        Ok(Self {
            engine: Arc::new(Mutex::new(ChatEngine::new(config)?)),
        })
    }

    pub fn from_engine(engine: ChatEngine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
        }
    }
}

pub fn create_router(state: ChatState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/uploads", post(upload))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/actions", get(actions))
        .route("/v1/actions/:id", get(action_by_id))
        .with_state(state)
        .layer(
            CorsLayer::permissive()
        )
}

pub async fn serve(addr: SocketAddr, state: ChatState) -> Result<(), ChatError> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, create_router(state)).await?;
    Ok(())
}

async fn health(State(state): State<ChatState>) -> Result<Json<HealthResponse>, ApiError> {
    let engine = state.engine.lock().await;
    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        uploads: engine.uploads.len(),
        actions: engine.actions.len(),
        built_in_tools: ChatEngine::built_in_tools(),
    }))
}

async fn upload(
    State(state): State<ChatState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let mut uploaded = Vec::new();
    while let Some(field) = multipart.next_field().await.map_err(ApiError::multipart)? {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field
            .file_name()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "upload.bin".to_string());
        let mime_type = field
            .content_type()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let bytes: Bytes = field.bytes().await.map_err(ApiError::multipart)?;
        let mut engine = state.engine.lock().await;
        uploaded.push(engine.upload_bytes(filename, mime_type, bytes.as_ref())?);
    }
    if uploaded.is_empty() {
        return Err(ApiError(ChatError::NoUploadFile));
    }
    Ok(Json(UploadResponse { uploads: uploaded }))
}

async fn chat_completions(
    State(state): State<ChatState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Response {
    if request.stream {
        let result = {
            let mut engine = state.engine.lock().await;
            engine.stream_events(request)
        };
        match result {
            Ok(events) => {
                let stream = tokio_stream::iter(events.into_iter().map(|event| {
                    let name = event.event_name();
                    match serde_json::to_string(&event) {
                        Ok(data) => {
                            Ok::<Event, Infallible>(Event::default().event(name).data(data))
                        }
                        Err(error) => {
                            let fallback = ChatStreamEvent::Error {
                                message: error.to_string(),
                            };
                            let data = serde_json::to_string(&fallback).unwrap_or_else(|_| {
                                "{\"type\":\"error\",\"message\":\"stream serialization failed\"}"
                                    .to_string()
                            });
                            Ok(Event::default().event("response.error").data(data))
                        }
                    }
                }));
                Sse::new(stream)
                    .keep_alive(KeepAlive::default())
                    .into_response()
            }
            Err(error) => ApiError(error).into_response(),
        }
    } else {
        let result = {
            let mut engine = state.engine.lock().await;
            engine.complete(request)
        };
        match result {
            Ok(response) => Json(response).into_response(),
            Err(error) => ApiError(error).into_response(),
        }
    }
}

async fn actions(State(state): State<ChatState>) -> Result<Json<Vec<ActionEvent>>, ApiError> {
    let engine = state.engine.lock().await;
    Ok(Json(engine.actions.clone()))
}

async fn action_by_id(
    State(state): State<ChatState>,
    Path(id): Path<String>,
) -> Result<Json<ActionEvent>, ApiError> {
    let engine = state.engine.lock().await;
    let action = engine
        .actions
        .iter()
        .find(|event| event.id == id)
        .cloned()
        .ok_or(ChatError::MissingAction(id))?;
    Ok(Json(action))
}

#[derive(Debug)]
pub struct ApiError(ChatError);

impl ApiError {
    fn multipart(error: axum::extract::multipart::MultipartError) -> Self {
        Self(ChatError::Multipart(error.to_string()))
    }
}

impl From<ChatError> for ApiError {
    fn from(error: ChatError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            ChatError::MissingUpload(_)
            | ChatError::MissingAction(_)
            | ChatError::UnknownTool(_)
            | ChatError::ToolNotAllowed(_) => StatusCode::NOT_FOUND,
            ChatError::UploadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            ChatError::Io(_) | ChatError::Server(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        let body = Json(json!({
            "error": {
                "message": self.0.to_string(),
                "type": "bqip_chat_error"
            }
        }));
        (status, body).into_response()
    }
}

fn analyze_upload(
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
    uploaded_at_ms: u64,
) -> Result<(UploadKind, UploadAnalysis), ChatError> {
    if is_image(filename, mime_type) {
        let image = image::load_from_memory(bytes)?;
        let rgb = image.to_rgb8();
        let (width, height) = rgb.dimensions();
        let envelope = phase_for_bytes(bytes);
        let frame = VisualFrame::new(
            width as usize,
            height as usize,
            PixelFormat::Rgb8,
            uploaded_at_ms,
            rgb.into_raw(),
        )?;
        let features = analyze_frame(&frame, envelope)?;
        let mut feature_hasher = blake3::Hasher::new();
        feature_hasher.update(features.frame_hash.as_slice());
        feature_hasher.update(&features.mean_luma.to_le_bytes());
        feature_hasher.update(&features.contrast.to_le_bytes());
        feature_hasher.update(&features.entropy.to_le_bytes());
        feature_hasher.update(&features.luma_grid);
        let feature_hash = *feature_hasher.finalize().as_bytes();
        let summary = format!(
            "Image {}x{} mean_luma={:.4} contrast={:.4} entropy={:.4}",
            width, height, features.mean_luma, features.contrast, features.entropy
        );
        return Ok((
            UploadKind::Image,
            UploadAnalysis::Image {
                width: width as usize,
                height: height as usize,
                mean_luma: features.mean_luma,
                contrast: features.contrast,
                entropy: features.entropy,
                frame_hash: features.frame_hash,
                feature_hash,
                summary,
            },
        ));
    }

    if is_document(filename, mime_type) {
        let text = extract_document_text(filename, mime_type, bytes)?;
        let text = truncate_chars(&normalize_ws(&text), MAX_DOCUMENT_TEXT_CHARS);
        let word_count = text.split_whitespace().count();
        let char_count = text.chars().count();
        let summary = summarize_text(&text);
        return Ok((
            UploadKind::Document,
            UploadAnalysis::Document {
                text,
                summary,
                word_count,
                char_count,
            },
        ));
    }

    Ok((
        UploadKind::Binary,
        UploadAnalysis::Binary {
            summary: format!(
                "Binary upload: {} bytes, blake3={}",
                bytes.len(),
                hex32(blake3::hash(bytes).as_bytes())
            ),
        },
    ))
}

fn is_image(filename: &str, mime_type: &str) -> bool {
    let lower_name = filename.to_ascii_lowercase();
    let lower_mime = mime_type.to_ascii_lowercase();
    lower_mime == "image/png"
        || lower_mime == "image/jpeg"
        || lower_name.ends_with(".png")
        || lower_name.ends_with(".jpg")
        || lower_name.ends_with(".jpeg")
}

fn is_document(filename: &str, mime_type: &str) -> bool {
    let lower_name = filename.to_ascii_lowercase();
    let lower_mime = mime_type.to_ascii_lowercase();
    lower_mime.starts_with("text/")
        || matches!(
            lower_mime.as_str(),
            "application/json"
                | "application/xml"
                | "application/pdf"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
        || lower_name.ends_with(".txt")
        || lower_name.ends_with(".md")
        || lower_name.ends_with(".json")
        || lower_name.ends_with(".csv")
        || lower_name.ends_with(".html")
        || lower_name.ends_with(".xml")
        || lower_name.ends_with(".pdf")
        || lower_name.ends_with(".docx")
}

fn extract_document_text(
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<String, ChatError> {
    let lower_name = filename.to_ascii_lowercase();
    let lower_mime = mime_type.to_ascii_lowercase();
    if lower_name.ends_with(".pdf") || lower_mime == "application/pdf" {
        return extract_pdf_text(bytes);
    }
    if lower_name.ends_with(".docx")
        || lower_mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    {
        return extract_docx_text(bytes);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ChatError::UnsupportedDocumentEncoding(filename.to_string()))?;
    if lower_name.ends_with(".html")
        || lower_mime == "text/html"
        || lower_mime == "application/xml"
        || lower_name.ends_with(".xml")
    {
        Ok(strip_markup(text))
    } else {
        Ok(text.to_string())
    }
}

fn extract_docx_text(bytes: &[u8]) -> Result<String, ChatError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut document = archive.by_name("word/document.xml")?;
    let mut xml = String::new();
    document.read_to_string(&mut xml)?;
    Ok(extract_word_xml_text(&xml))
}

fn extract_word_xml_text(xml: &str) -> String {
    let mut output = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<w:t") {
        rest = &rest[start + 4..];
        let Some(close) = rest.find('>') else {
            break;
        };
        rest = &rest[close + 1..];
        let Some(end) = rest.find("</w:t>") else {
            break;
        };
        let text = &rest[..end];
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(&xml_unescape(text));
        rest = &rest[end + 6..];
    }
    output
}

fn extract_pdf_text(bytes: &[u8]) -> Result<String, ChatError> {
    let document =
        lopdf::Document::load_mem(bytes).map_err(|error| ChatError::Pdf(error.to_string()))?;
    if document.is_encrypted() {
        return Err(ChatError::PdfEncrypted);
    }
    let mut text = String::new();
    for (_, page_id) in document.get_pages() {
        let content_data = document
            .get_page_content(page_id)
            .map_err(|error| ChatError::Pdf(error.to_string()))?;
        let content =
            Content::decode(&content_data).map_err(|error| ChatError::Pdf(error.to_string()))?;
        for operation in content.operations {
            match operation.operator.as_str() {
                "Tj" | "'" => {
                    if let Some(value) = operation.operands.first() {
                        append_pdf_text(value, &mut text);
                    }
                }
                "\"" => {
                    if let Some(value) = operation.operands.get(2) {
                        append_pdf_text(value, &mut text);
                    }
                }
                "TJ" => {
                    if let Some(Object::Array(items)) = operation.operands.first() {
                        for item in items {
                            append_pdf_text(item, &mut text);
                        }
                    }
                }
                _ => {}
            }
        }
        text.push('\n');
    }
    let normalized = normalize_ws(&text);
    if normalized.is_empty() {
        return Err(ChatError::PdfNoExtractableText);
    }
    Ok(normalized)
}

fn append_pdf_text(object: &Object, output: &mut String) {
    if let Object::String(bytes, _) = object {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(&String::from_utf8_lossy(bytes));
    }
}

fn strip_markup(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;
    for character in input.chars() {
        match character {
            '<' => {
                inside_tag = true;
                output.push(' ');
            }
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    xml_unescape(&output)
}

fn xml_unescape(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn normalize_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn summarize_text(input: &str) -> String {
    let normalized = normalize_ws(input);
    if normalized.is_empty() {
        return "Document contains no extractable text.".to_string();
    }
    let mut summary = normalized.chars().take(480).collect::<String>();
    if normalized.chars().count() > 480 {
        summary.push('…');
    }
    summary
}

fn phase_for_bytes(bytes: &[u8]) -> PhaseEnvelope {
    let hash = blake3::hash(bytes);
    let mut sig = [0u8; 8];
    sig.copy_from_slice(&hash.as_bytes()[..8]);
    PhaseEnvelope::balanced(u64::from_le_bytes(sig))
}

fn collect_prompt_text(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .flat_map(|message| message.content.text_fragments())
        .collect::<Vec<_>>()
        .join("\n")
}

fn response_lead(prompt_text: &str, upload_count: usize, tool_count: usize) -> String {
    let leading = prompt_text
        .split_whitespace()
        .take(32)
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "Processed request: `{}`. Attached uploads={}, requested_tools={}.",
        leading, upload_count, tool_count
    )
}

fn synthesis_items(uploads: &[UploadRecord], tools: &[ToolCallView]) -> Vec<String> {
    let mut items = Vec::new();
    if !uploads.is_empty() {
        items.push(format!(
            "Upload context grounded with {} file(s).",
            uploads.len()
        ));
    }
    let completed = tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::Completed)
        .count();
    let failed = tools.len().saturating_sub(completed);
    if !tools.is_empty() {
        items.push(format!(
            "Tool execution completed={} failed={}.",
            completed, failed
        ));
    }
    items
}

fn render_markdown(blocks: &[RichBlock]) -> Result<String, ChatError> {
    let mut out = String::new();
    for block in blocks {
        match block {
            RichBlock::Heading { level, text } => {
                let level = (*level).clamp(1, 6) as usize;
                out.push_str(&"#".repeat(level));
                out.push(' ');
                out.push_str(text);
                out.push_str("\n\n");
            }
            RichBlock::Paragraph { text } => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            RichBlock::Bullets { items } => {
                for item in items {
                    out.push_str("- ");
                    out.push_str(item);
                    out.push('\n');
                }
                out.push('\n');
            }
            RichBlock::CodeBlock { language, code } => {
                out.push_str("```");
                out.push_str(language);
                out.push('\n');
                out.push_str(code);
                out.push_str("\n```\n\n");
            }
            RichBlock::ToolCard { tool } => {
                out.push_str(&format!("### Tool: `{}`\n\n", tool.name));
                out.push_str(&format!("- status: `{:?}`\n", tool.status));
                out.push_str("- input:\n");
                out.push_str("```json\n");
                out.push_str(&serde_json::to_string_pretty(&tool.input)?);
                out.push_str("\n```\n");
                out.push_str("- output:\n");
                out.push_str("```json\n");
                out.push_str(&serde_json::to_string_pretty(&tool.output)?);
                out.push_str("\n```\n\n");
            }
            RichBlock::UploadCard { upload } => {
                out.push_str(&format!("### Upload: `{}`\n\n", upload.filename));
                out.push_str(&format!("- id: `{}`\n", upload.upload_id));
                out.push_str(&format!("- mime: `{}`\n", upload.mime_type));
                out.push_str(&format!("- kind: `{:?}`\n", upload.kind));
                out.push_str(&format!("- bytes: `{}`\n", upload.size_bytes));
                out.push_str(&format!("- summary: {}\n\n", upload.summary));
            }
            RichBlock::ActionTimeline { events } => {
                out.push_str("### Actions\n\n");
                for event in events {
                    out.push_str(&format!(
                        "- `{}` {:?}: {} — {}\n",
                        event.kind, event.status, event.title, event.detail
                    ));
                }
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn parse_inline_tools(prompt_text: &str) -> Result<Vec<ToolInvocation>, ChatError> {
    let mut invocations = Vec::new();
    for line in prompt_text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            continue;
        }
        let mut parts = trimmed[1..].splitn(2, char::is_whitespace);
        let Some(name) = parts.next() else {
            continue;
        };
        let input = parts.next().unwrap_or_default().trim();
        let arguments = match name {
            "calculator" => json!({ "expression": input }),
            "search_uploads" => json!({ "query": input }),
            "summarize_upload" | "analyze_image" => json!({ "upload_id": input }),
            "training_control" | "training" => json!({ "instruction": input }),
            "resolve_huggingface_registers" | "huggingface_registers" => json!({}),
            other => {
                return Err(ChatError::UnknownTool(other.to_string()));
            }
        };
        let normalized_name = match name {
            "training" => "training_control",
            "huggingface_registers" => "resolve_huggingface_registers",
            value => value,
        };
        invocations.push(ToolInvocation {
            name: normalized_name.to_string(),
            arguments,
        });
    }
    Ok(invocations)
}

fn allowed_tool_names(tools: &[ToolSpec]) -> HashSet<String> {
    tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect::<HashSet<_>>()
}

fn string_argument(arguments: &Value, key: &'static str) -> Result<String, ChatError> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(ChatError::MissingToolArgument(key))
}

fn usize_argument(arguments: &Value, key: &'static str) -> Option<usize> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn control_actions_json(actions: &[ControlAction]) -> Vec<Value> {
    actions
        .iter()
        .map(|action| match action {
            ControlAction::TextIngested {
                concept_label,
                token_count,
            } => json!({"type":"text_ingested","concept_label":concept_label,"token_count":token_count}),
            ControlAction::InferenceIngested {
                concept_label,
                provider,
                model,
            } => json!({"type":"inference_ingested","concept_label":concept_label,"provider":provider,"model":model}),
            ControlAction::TrainConfigUpdated { field, value } => {
                json!({"type":"train_config_updated","field":field,"value":value})
            }
            ControlAction::TrainingCompleted {
                initial_loss,
                final_loss,
                ledger_records,
            } => json!({"type":"training_completed","initial_loss":initial_loss,"final_loss":final_loss,"ledger_records":ledger_records}),
            ControlAction::EvaluationCompleted {
                mean_loss,
                accuracy,
            } => json!({"type":"evaluation_completed","mean_loss":mean_loss,"accuracy":accuracy}),
            ControlAction::StatusReported {
                documents,
                inferences,
                ledger_records,
                epochs,
                learning_rate,
                trainable_scope,
            } => json!({
                "type":"status_reported",
                "documents":documents,
                "inferences":inferences,
                "ledger_records":ledger_records,
                "epochs":epochs,
                "learning_rate":learning_rate,
                "trainable_scope":format!("{:?}", trainable_scope)
            }),
        })
        .collect()
}

fn compact_json(value: &Value) -> String {
    match serde_json::to_string(value) {
        Ok(serialized) => serialized.chars().take(640).collect(),
        Err(error) => error.to_string(),
    }
}

fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

fn unix_millis() -> Result<u64, ChatError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ChatError::ClockBeforeUnixEpoch)?;
    Ok(duration.as_millis().min(u128::from(u64::MAX)) as u64)
}

fn hex32(bytes: &[u8; REGISTER_BYTES]) -> String {
    hex_prefix(bytes, REGISTER_BYTES)
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    let mut out = String::with_capacity(len.saturating_mul(2));
    for byte in bytes.iter().take(len) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < 1.0e-12 {
        format!("{value:.0}")
    } else {
        format!("{value:.12}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

struct Calculator<'a> {
    input: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> Calculator<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<f64, ChatError> {
        let value = self.expression()?;
        self.skip_ws();
        if self.pos != self.chars.len() {
            return Err(ChatError::Calculator(format!(
                "unexpected token `{}` at position {}",
                self.chars[self.pos], self.pos
            )));
        }
        if !value.is_finite() {
            return Err(ChatError::Calculator("non-finite result".to_string()));
        }
        Ok(value)
    }

    fn expression(&mut self) -> Result<f64, ChatError> {
        let mut value = self.term()?;
        loop {
            self.skip_ws();
            if self.consume('+') {
                value += self.term()?;
            } else if self.consume('-') {
                value -= self.term()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn term(&mut self) -> Result<f64, ChatError> {
        let mut value = self.factor()?;
        loop {
            self.skip_ws();
            if self.consume('*') {
                value *= self.factor()?;
            } else if self.consume('/') {
                let denominator = self.factor()?;
                if denominator.abs() < f64::EPSILON {
                    return Err(ChatError::Calculator("division by zero".to_string()));
                }
                value /= denominator;
            } else {
                return Ok(value);
            }
        }
    }

    fn factor(&mut self) -> Result<f64, ChatError> {
        self.skip_ws();
        if self.consume('+') {
            return self.factor();
        }
        if self.consume('-') {
            return Ok(-self.factor()?);
        }
        if self.consume('(') {
            let value = self.expression()?;
            self.skip_ws();
            if !self.consume(')') {
                return Err(ChatError::Calculator(
                    "missing closing parenthesis".to_string(),
                ));
            }
            return Ok(value);
        }
        self.number()
    }

    fn number(&mut self) -> Result<f64, ChatError> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_ascii_digit() || self.chars[self.pos] == '.')
        {
            self.pos += 1;
        }
        if self.pos < self.chars.len() && matches!(self.chars[self.pos], 'e' | 'E') {
            self.pos += 1;
            if self.pos < self.chars.len() && matches!(self.chars[self.pos], '+' | '-') {
                self.pos += 1;
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        if start == self.pos {
            return Err(ChatError::Calculator(format!(
                "expected number at position {} in `{}`",
                self.pos, self.input
            )));
        }
        self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|_| ChatError::Calculator("invalid number".to_string()))
    }

    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn consume(&mut self, character: char) -> bool {
        if self.pos < self.chars.len() && self.chars[self.pos] == character {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("chat model must be nonempty")]
    EmptyModel,
    #[error("chat request must contain at least one message")]
    EmptyMessages,
    #[error("message content must contain text, an upload reference, or a tool call")]
    EmptyMessage,
    #[error("upload must contain at least one byte")]
    EmptyUpload,
    #[error("upload file field is required")]
    NoUploadFile,
    #[error("upload filename must be nonempty")]
    InvalidUploadName,
    #[error("upload exceeds byte limit: limit={limit}, actual={actual}")]
    UploadTooLarge { limit: usize, actual: usize },
    #[error("missing upload: {0}")]
    MissingUpload(String),
    #[error("missing action: {0}")]
    MissingAction(String),
    #[error("document encoding is not supported for {0}")]
    UnsupportedDocumentEncoding(String),
    #[error("PDF is encrypted")]
    PdfEncrypted,
    #[error("PDF contains no extractable text")]
    PdfNoExtractableText,
    #[error("PDF extraction failed: {0}")]
    Pdf(String),
    #[error("upload is not an image: {0}")]
    UploadIsNotImage(String),
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("tool is not allowed by request tools: {0}")]
    ToolNotAllowed(String),
    #[error("missing tool argument: {0}")]
    MissingToolArgument(&'static str),
    #[error("max_tool_calls must be at least 1")]
    InvalidToolBudget,
    #[error("calculator failed: {0}")]
    Calculator(String),
    #[error("system clock is before Unix epoch")]
    ClockBeforeUnixEpoch,
    #[error("multipart upload failed: {0}")]
    Multipart(String),
    #[error("server failed: {0}")]
    Server(String),
    #[error(transparent)]
    Control(#[from] bqip_control::ControlError),
    #[error(transparent)]
    Ingest(#[from] bqip_ingestor::IngestError),
    #[error(transparent)]
    Multimodal(#[from] bqip_multimodal::MultimodalError),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}
