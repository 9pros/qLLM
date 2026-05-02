use std::io::{Cursor, Write};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bqip_chat::{
    create_router, ActionMode, ChatCompletionRequest, ChatEngine, ChatMessage, ChatRole, ChatState,
    ContentPart, MessageContent, ToolStatus, UploadAnalysis, UploadKind,
};
use bqip_control::TrainingControlConfig;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use tower::ServiceExt;
use zip::write::FileOptions;

fn engine() -> ChatEngine {
    ChatEngine::new(TrainingControlConfig::default()).expect("chat engine initializes")
}

fn request_with_text(text: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "bqip-hybrid-chat".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: MessageContent::Text(text.to_string()),
        }],
        stream: false,
        tools: Vec::new(),
        upload_ids: Vec::new(),
        action_mode: ActionMode::Inline,
        max_tool_calls: 16,
    }
}

#[test]
fn text_document_upload_is_searchable_and_summarized() {
    let mut engine = engine();
    let upload = engine
        .upload_bytes(
            "reactive-notes.md",
            "text/markdown",
            b"Reactive graphs update subscribers when memory blocks change. Concepts layer over memory blocks.",
        )
        .expect("text upload analyzes");
    assert_eq!(upload.kind, UploadKind::Document);
    match &upload.analysis {
        UploadAnalysis::Document {
            summary,
            word_count,
            ..
        } => {
            assert!(summary.contains("Reactive graphs"));
            assert!(*word_count >= 10);
        }
        other => panic!("unexpected analysis: {other:?}"),
    }

    let mut request = request_with_text("@search_uploads reactive graphs");
    request.upload_ids.push(upload.id.clone());
    let response = engine.complete(request).expect("chat completes");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].status, ToolStatus::Completed);
    assert!(response.choices[0]
        .message
        .content
        .contains("Tool: `search_uploads`"));
    assert!(response.choices[0]
        .message
        .content
        .contains("reactive-notes.md"));
}

#[test]
fn docx_upload_extracts_word_xml_text() {
    let mut zip_bytes = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut zip_bytes);
        writer
            .start_file(
                "word/document.xml",
                FileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .expect("docx xml starts");
        writer
            .write_all(
                br#"<w:document><w:body><w:p><w:r><w:t>DeltaNet memory</w:t></w:r><w:r><w:t>over concept layers</w:t></w:r></w:p></w:body></w:document>"#,
            )
            .expect("docx xml writes");
        writer.finish().expect("docx zip finishes");
    }
    let mut engine = engine();
    let upload = engine
        .upload_bytes(
            "notes.docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            &zip_bytes.into_inner(),
        )
        .expect("docx upload analyzes");
    assert_eq!(upload.kind, UploadKind::Document);
    assert!(upload.analysis.summary().contains("DeltaNet memory"));
}

#[test]
fn png_upload_runs_visual_feature_analysis() {
    let image = ImageBuffer::from_fn(4, 4, |x, y| {
        if (x + y) % 2 == 0 {
            Rgb([255u8, 0u8, 0u8])
        } else {
            Rgb([0u8, 0u8, 255u8])
        }
    });
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("png encodes");

    let mut engine = engine();
    let upload = engine
        .upload_bytes("checker.png", "image/png", bytes.get_ref())
        .expect("image upload analyzes");
    assert_eq!(upload.kind, UploadKind::Image);
    match upload.analysis {
        UploadAnalysis::Image {
            width,
            height,
            entropy,
            summary,
            ..
        } => {
            assert_eq!(width, 4);
            assert_eq!(height, 4);
            assert!(entropy > 0.0);
            assert!(summary.contains("Image 4x4"));
        }
        other => panic!("unexpected analysis: {other:?}"),
    }
}

#[test]
fn calculator_tool_renders_as_rich_tool_card() {
    let mut engine = engine();
    let response = engine
        .complete(request_with_text("@calculator 2 * (3 + 4)"))
        .expect("calculator response completes");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "calculator");
    assert_eq!(response.tool_calls[0].status, ToolStatus::Completed);
    assert_eq!(response.tool_calls[0].output["display"], "14");
    assert!(response.choices[0].message.content.contains("```json"));
}

#[test]
fn structured_tool_call_can_control_training_state() {
    let mut engine = engine();
    let request = ChatCompletionRequest {
        model: "bqip-hybrid-chat".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: MessageContent::Parts(vec![ContentPart::ToolCall {
                name: "training_control".to_string(),
                arguments: serde_json::json!({
                    "instruction": "learn concept endpoint-chat: rich chat endpoints stream tool cards and uploads"
                }),
            }]),
        }],
        stream: false,
        tools: Vec::new(),
        upload_ids: Vec::new(),
        action_mode: ActionMode::Inline,
        max_tool_calls: 4,
    };
    let response = engine.complete(request).expect("training control executes");
    assert_eq!(response.tool_calls[0].status, ToolStatus::Completed);
    assert!(response.tool_calls[0].output["reply"]
        .as_str()
        .expect("reply is string")
        .contains("ingested"));
}

#[test]
fn stream_events_preserve_display_order() {
    let mut engine = engine();
    let events = engine
        .stream_events(request_with_text("@calculator 7 + 5"))
        .expect("stream events build");
    assert!(matches!(
        events.first(),
        Some(bqip_chat::ChatStreamEvent::ResponseStarted { .. })
    ));
    assert!(matches!(
        events.last(),
        Some(bqip_chat::ChatStreamEvent::ResponseCompleted { .. })
    ));
    assert!(events
        .iter()
        .any(|event| matches!(event, bqip_chat::ChatStreamEvent::ToolCallDelta { .. })));
    assert!(events
        .iter()
        .any(|event| matches!(event, bqip_chat::ChatStreamEvent::ActionDelta { .. })));
}

#[tokio::test]
async fn router_exposes_health_endpoint() {
    let state = ChatState::new(TrainingControlConfig::default()).expect("state initializes");
    let app = create_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("health request succeeds");
    assert_eq!(response.status(), StatusCode::OK);
}
