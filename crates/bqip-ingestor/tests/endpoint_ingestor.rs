use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use bqip_core::RegisterLane;
use bqip_ingestor::{
    huggingface_endpoint_requests, huggingface_register_targets, resolve_register_targets,
    EndpointIngestor, EndpointRequest, IngestError, RegisterTarget, ResolvedEndpointRegister,
};
use bqip_knowledge_graph::EndpointKind;
use bqip_training::{ConceptTokenCompiler, ConceptTokenizerConfig};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn compiler() -> ConceptTokenCompiler {
    ConceptTokenCompiler::new(ConceptTokenizerConfig {
        vocab_size: 64,
        max_tokens: 32,
        ngram_min: 1,
        ngram_max: 2,
        include_byte_tokens: true,
    })
    .expect("tokenizer config should be valid")
}

fn spawn_http_server(body: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let address = listener.local_addr().expect("read local address");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one request");
        let mut request_bytes = [0u8; 1024];
        let _ = stream.read(&mut request_bytes).expect("read request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response headers");
        stream.write_all(body).expect("write response body");
    });
    format!("http://{address}/api/live")
}

#[test]
fn fetches_endpoint_and_builds_daemon_sense_event() {
    let url = spawn_http_server(br#"{"concept":"reactive graph","status":"live"}"#);
    let request = EndpointRequest::get(url, "Reactive API")
        .expect("endpoint request should parse")
        .with_limits(5_000, 16_384, 60_000)
        .expect("limits should validate");
    let ingestor = EndpointIngestor::new(compiler());
    let observation = ingestor.fetch(request).expect("fetch local endpoint");
    let event = observation
        .to_sense_event(ingestor.compiler())
        .expect("observation should become a sense event");

    assert_eq!(observation.status, 200);
    assert_eq!(event.concept_label, "Reactive API");
    assert_eq!(event.endpoint_lane, RegisterLane::PublicApi);
    assert_eq!(event.endpoint_kind, EndpointKind::PublicApi);
    assert!(!event.tokens.is_empty());
    assert_eq!(event.payload, observation.body);
}

#[test]
fn observations_compile_into_training_corpus() {
    let url = spawn_http_server(b"concept compiler builds grounded training windows");
    let request = EndpointRequest::get(url, "Compiler")
        .expect("endpoint request should parse")
        .with_limits(5_000, 16_384, 60_000)
        .expect("limits should validate");
    let ingestor = EndpointIngestor::new(compiler());
    let observation = ingestor.fetch(request).expect("fetch local endpoint");
    let corpus = ingestor
        .observations_to_corpus(64, 6, &[observation])
        .expect("observation should compile to corpus");

    assert_eq!(corpus.vocab_size, 64);
    assert!(!corpus.examples.is_empty());
    corpus.validate().expect("corpus validates");
}

#[test]
fn body_limit_is_enforced_before_observation_is_returned() {
    let url = spawn_http_server(b"body exceeds configured limit");
    let request = EndpointRequest::get(url, "Limited")
        .expect("endpoint request should parse")
        .with_limits(5_000, 4, 60_000)
        .expect("limits should validate");
    let ingestor = EndpointIngestor::new(compiler());
    let error = ingestor.fetch(request).expect_err("body limit must fail");

    assert!(matches!(error, IngestError::BodyTooLarge { .. }));
}

#[test]
fn resolved_endpoint_registers_preserve_ipv4_and_ipv6_lanes() {
    let key = [7u8; 32];
    let ipv4 = ResolvedEndpointRegister::new(
        "huggingface.co",
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)),
        443,
        EndpointKind::Website,
        &key,
        1_000,
    )
    .expect("ipv4 register should derive");
    let ipv6 = ResolvedEndpointRegister::new(
        "huggingface.co",
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
        443,
        EndpointKind::Website,
        &key,
        1_000,
    )
    .expect("ipv6 register should derive");

    assert_eq!(ipv4.lane, RegisterLane::Ipv4);
    assert_eq!(ipv6.lane, RegisterLane::Ipv6);
    assert_ne!(ipv4.register_id, ipv6.register_id);
    assert!(String::from_utf8(ipv4.canonical_address.clone())
        .expect("canonical ipv4")
        .starts_with("ipv4:"));
    assert!(String::from_utf8(ipv6.canonical_address.clone())
        .expect("canonical ipv6")
        .starts_with("ipv6:["));
}

#[test]
fn register_target_resolution_dedupes_localhost_addresses() {
    let key = [8u8; 32];
    let report = resolve_register_targets(
        &[RegisterTarget::new("localhost", 80, EndpointKind::Website)
            .expect("localhost target validates")],
        &key,
        2_000,
    )
    .expect("localhost resolves on macOS");

    assert_eq!(report.target_count, 1);
    assert!(!report.registers.is_empty());
    assert_eq!(report.registers.len(), {
        let mut unique = std::collections::HashSet::new();
        for register in &report.registers {
            unique.insert((
                register.ip_address.clone(),
                String::from_utf8_lossy(register.lane.tag()).into_owned(),
            ));
        }
        unique.len()
    });
    assert!(report.ipv4_count() + report.ipv6_count() == report.registers.len());
}

#[test]
fn huggingface_register_targets_cover_api_cdn_and_fetchable_requests() {
    let targets = huggingface_register_targets();
    let requests = huggingface_endpoint_requests().expect("static hf requests validate");

    assert!(targets
        .iter()
        .any(|target| target.hostname == "huggingface.co"
            && target.endpoint_kind == EndpointKind::Website));
    assert!(targets
        .iter()
        .any(|target| target.hostname == "api-inference.huggingface.co"
            && target.endpoint_kind == EndpointKind::PublicApi));
    assert!(targets
        .iter()
        .any(|target| target.hostname == "cdn-lfs.huggingface.co"
            && target.endpoint_kind == EndpointKind::PublicProxy));
    assert!(requests
        .iter()
        .any(|request| request.url.contains("/api/models")));
    assert!(requests
        .iter()
        .any(|request| request.url.contains("/api/datasets")));
}

#[test]
fn endpoint_observation_can_bind_to_resolved_ip_register() {
    let url = spawn_http_server(b"resolved register event payload");
    let request = EndpointRequest::get(url.clone(), "Resolved Localhost")
        .expect("request validates")
        .with_limits(5_000, 16_384, 60_000)
        .expect("limits validate");
    let ingestor = EndpointIngestor::new(compiler());
    let observation = ingestor.fetch(request).expect("local fetch succeeds");
    let host = url
        .strip_prefix("http://")
        .expect("local url prefix")
        .split('/')
        .next()
        .expect("host:port")
        .split(':')
        .next()
        .expect("host");
    let register = ResolvedEndpointRegister::new(
        host,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        80,
        EndpointKind::Website,
        &[9u8; 32],
        observation.fetched_at_unix_millis,
    )
    .expect("register derives");
    let event = observation
        .to_sense_event_for_register(ingestor.compiler(), &register)
        .expect("event binds to register");

    assert_eq!(event.endpoint_lane, RegisterLane::Ipv4);
    assert!(String::from_utf8(event.canonical_endpoint)
        .expect("canonical endpoint utf8")
        .starts_with("ipv4:127.0.0.1"));
    assert_eq!(event.payload, observation.body);
}
