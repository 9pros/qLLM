use std::collections::HashSet;
use std::io::{self, Read};
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bqip_core::{derive_register_id, InterfaceKind, RegisterId, RegisterLane, REGISTER_BYTES};
use bqip_daemon::{EventSource, SenseEvent};
use bqip_knowledge_graph::EndpointKind;
use bqip_training::{ConceptTokenCompiler, TokenizedDocument, TrainingCorpus, TrainingError};
use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_FRESHNESS_HALF_LIFE_MS: u64 = 86_400_000;
const HTTPS_PORT: u16 = 443;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

impl HeaderPair {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, IngestError> {
        let header = Self {
            name: name.into(),
            value: value.into(),
        };
        header.validate()?;
        Ok(header)
    }

    fn validate(&self) -> Result<(), IngestError> {
        if self.name.trim().is_empty() {
            return Err(IngestError::InvalidHeader("header name is empty"));
        }
        if self.name.bytes().any(|byte| byte <= 31 || byte == 127) {
            return Err(IngestError::InvalidHeader(
                "header name contains a control byte",
            ));
        }
        if self
            .value
            .bytes()
            .any(|byte| byte == b'\r' || byte == b'\n')
        {
            return Err(IngestError::InvalidHeader(
                "header value contains a newline",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndpointRequest {
    pub url: String,
    pub concept_label: String,
    pub lane: RegisterLane,
    pub interface_kind: InterfaceKind,
    pub endpoint_kind: EndpointKind,
    pub slot_index: u64,
    pub headers: Vec<HeaderPair>,
    pub timeout_ms: u64,
    pub max_body_bytes: usize,
    pub freshness_half_life_millis: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterTarget {
    pub hostname: String,
    pub port: u16,
    pub endpoint_kind: EndpointKind,
}

impl RegisterTarget {
    pub fn new(
        hostname: impl Into<String>,
        port: u16,
        endpoint_kind: EndpointKind,
    ) -> Result<Self, IngestError> {
        let target = Self {
            hostname: hostname.into(),
            port,
            endpoint_kind,
        };
        target.validate()?;
        Ok(target)
    }

    pub fn validate(&self) -> Result<(), IngestError> {
        if self.hostname.trim().is_empty() {
            return Err(IngestError::EmptyHostname);
        }
        if self.hostname.contains('/')
            || self.hostname.contains(':') && !self.hostname.contains('.')
        {
            return Err(IngestError::InvalidHostname);
        }
        if self.port == 0 {
            return Err(IngestError::InvalidLimit("port"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedEndpointRegister {
    pub hostname: String,
    pub ip_address: String,
    pub port: u16,
    pub lane: RegisterLane,
    pub interface_kind: InterfaceKind,
    pub endpoint_kind: EndpointKind,
    pub slot_index: u64,
    pub canonical_address: Vec<u8>,
    pub register_id: RegisterId,
    pub observed_at_unix_millis: u64,
}

impl ResolvedEndpointRegister {
    pub fn new(
        hostname: impl Into<String>,
        ip: IpAddr,
        port: u16,
        endpoint_kind: EndpointKind,
        node_public_key: &[u8; REGISTER_BYTES],
        observed_at_unix_millis: u64,
    ) -> Result<Self, IngestError> {
        if port == 0 {
            return Err(IngestError::InvalidLimit("port"));
        }
        let hostname = hostname.into();
        if hostname.trim().is_empty() {
            return Err(IngestError::EmptyHostname);
        }
        let lane = match ip {
            IpAddr::V4(_) => RegisterLane::Ipv4,
            IpAddr::V6(_) => RegisterLane::Ipv6,
        };
        let ip_address = ip.to_string();
        let canonical_address = match ip {
            IpAddr::V4(_) => format!("ipv4:{ip_address}:{port}").into_bytes(),
            IpAddr::V6(_) => format!("ipv6:[{ip_address}]:{port}").into_bytes(),
        };
        let slot_index = register_slot_index(&hostname, &canonical_address);
        let register_id = derive_register_id(
            lane,
            slot_index,
            &canonical_address,
            InterfaceKind::Application,
            node_public_key,
        );
        Ok(Self {
            hostname,
            ip_address,
            port,
            lane,
            interface_kind: InterfaceKind::Application,
            endpoint_kind,
            slot_index,
            canonical_address,
            register_id,
            observed_at_unix_millis,
        })
    }

    pub fn validate(&self) -> Result<(), IngestError> {
        if self.hostname.trim().is_empty() {
            return Err(IngestError::EmptyHostname);
        }
        if self.canonical_address.is_empty() {
            return Err(IngestError::EmptyCanonicalRegisterAddress);
        }
        let parsed_ip = self
            .ip_address
            .parse::<IpAddr>()
            .map_err(|_| IngestError::InvalidResolvedIp)?;
        let expected_lane = match parsed_ip {
            IpAddr::V4(_) => RegisterLane::Ipv4,
            IpAddr::V6(_) => RegisterLane::Ipv6,
        };
        if self.lane != expected_lane {
            return Err(IngestError::RegisterLaneIpMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionFailure {
    pub hostname: String,
    pub port: u16,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterIngestReport {
    pub observed_at_unix_millis: u64,
    pub target_count: usize,
    pub registers: Vec<ResolvedEndpointRegister>,
    pub failures: Vec<ResolutionFailure>,
}

impl RegisterIngestReport {
    pub fn validate(&self) -> Result<(), IngestError> {
        if self.target_count == 0 {
            return Err(IngestError::EmptyRegisterTargetSet);
        }
        if self.registers.is_empty() {
            return Err(IngestError::EmptyResolvedRegisterSet);
        }
        for register in &self.registers {
            register.validate()?;
        }
        Ok(())
    }

    pub fn ipv4_count(&self) -> usize {
        self.registers
            .iter()
            .filter(|register| register.lane == RegisterLane::Ipv4)
            .count()
    }

    pub fn ipv6_count(&self) -> usize {
        self.registers
            .iter()
            .filter(|register| register.lane == RegisterLane::Ipv6)
            .count()
    }
}

impl EndpointRequest {
    pub fn get(
        url: impl Into<String>,
        concept_label: impl Into<String>,
    ) -> Result<Self, IngestError> {
        let url = url.into();
        let parsed = Url::parse(&url)?;
        let concept_label = concept_label.into();
        let (lane, endpoint_kind) = classify_url(&parsed);
        let request = Self {
            slot_index: url_slot_index(&url),
            url,
            concept_label,
            lane,
            interface_kind: InterfaceKind::Application,
            endpoint_kind,
            headers: Vec::new(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            freshness_half_life_millis: DEFAULT_FRESHNESS_HALF_LIFE_MS,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, IngestError> {
        self.headers.push(HeaderPair::new(name, value)?);
        self.validate()?;
        Ok(self)
    }

    pub fn with_limits(
        mut self,
        timeout_ms: u64,
        max_body_bytes: usize,
        freshness_half_life_millis: u64,
    ) -> Result<Self, IngestError> {
        self.timeout_ms = timeout_ms;
        self.max_body_bytes = max_body_bytes;
        self.freshness_half_life_millis = freshness_half_life_millis;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), IngestError> {
        let parsed = Url::parse(&self.url)?;
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => return Err(IngestError::UnsupportedScheme(scheme.to_string())),
        }
        if self.concept_label.trim().is_empty() {
            return Err(IngestError::EmptyConceptLabel);
        }
        if parsed.host_str().is_none() {
            return Err(IngestError::MissingHost);
        }
        if self.timeout_ms == 0 {
            return Err(IngestError::InvalidLimit("timeout_ms"));
        }
        if self.max_body_bytes == 0 {
            return Err(IngestError::InvalidLimit("max_body_bytes"));
        }
        if self.freshness_half_life_millis == 0 {
            return Err(IngestError::InvalidLimit("freshness_half_life_millis"));
        }
        for header in &self.headers {
            header.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndpointObservation {
    pub request: EndpointRequest,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<HeaderPair>,
    pub headers_hash: [u8; REGISTER_BYTES],
    pub body: Vec<u8>,
    pub body_hash: [u8; REGISTER_BYTES],
    pub fetched_at_unix_millis: u64,
    pub duration_millis: u64,
}

impl EndpointObservation {
    pub fn validate(&self) -> Result<(), IngestError> {
        self.request.validate()?;
        if self.status == 0 {
            return Err(IngestError::InvalidStatus);
        }
        if self.body.is_empty() {
            return Err(IngestError::EmptyBody);
        }
        if self.body.len() > self.request.max_body_bytes {
            return Err(IngestError::BodyTooLarge {
                limit: self.request.max_body_bytes,
                actual: self.body.len(),
            });
        }
        if self.body_hash != *blake3::hash(&self.body).as_bytes() {
            return Err(IngestError::BodyHashMismatch);
        }
        if self.headers_hash != hash_headers(&self.headers) {
            return Err(IngestError::HeaderHashMismatch);
        }
        Ok(())
    }

    pub fn tokenized(
        &self,
        compiler: &ConceptTokenCompiler,
    ) -> Result<TokenizedDocument, IngestError> {
        Ok(compiler.compile(self.request.concept_label.clone(), &self.body)?)
    }

    pub fn to_sense_event(
        &self,
        compiler: &ConceptTokenCompiler,
    ) -> Result<SenseEvent, IngestError> {
        self.validate()?;
        let tokenized = self.tokenized(compiler)?;
        Ok(SenseEvent::new(
            EventSource::Tool,
            tokenized.tokens,
            self.fetched_at_unix_millis,
        )
        .with_endpoint_observation(
            self.request.concept_label.clone(),
            self.request.lane,
            self.request.interface_kind,
            self.request.endpoint_kind.clone(),
            self.request.slot_index,
            self.request.url.as_bytes().to_vec(),
            self.body.clone(),
            self.request.freshness_half_life_millis,
        ))
    }

    pub fn to_sense_event_for_register(
        &self,
        compiler: &ConceptTokenCompiler,
        register: &ResolvedEndpointRegister,
    ) -> Result<SenseEvent, IngestError> {
        self.validate()?;
        register.validate()?;
        if register.hostname
            != Url::parse(&self.request.url)?
                .host_str()
                .unwrap_or_default()
        {
            return Err(IngestError::ResolvedRegisterHostMismatch);
        }
        let tokenized = self.tokenized(compiler)?;
        Ok(SenseEvent::new(
            EventSource::Tool,
            tokenized.tokens,
            self.fetched_at_unix_millis,
        )
        .with_endpoint_observation(
            self.request.concept_label.clone(),
            register.lane,
            register.interface_kind,
            register.endpoint_kind.clone(),
            register.slot_index,
            register.canonical_address.clone(),
            self.body.clone(),
            self.request.freshness_half_life_millis,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct EndpointIngestor {
    compiler: ConceptTokenCompiler,
}

impl EndpointIngestor {
    pub fn new(compiler: ConceptTokenCompiler) -> Self {
        Self { compiler }
    }

    pub fn compiler(&self) -> &ConceptTokenCompiler {
        &self.compiler
    }

    pub fn fetch(&self, request: EndpointRequest) -> Result<EndpointObservation, IngestError> {
        request.validate()?;
        let started = Instant::now();
        let fetched_at_unix_millis = unix_millis()?;
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_millis(request.timeout_ms))
            .timeout_read(Duration::from_millis(request.timeout_ms))
            .timeout_write(Duration::from_millis(request.timeout_ms))
            .build();
        let mut call = agent
            .get(&request.url)
            .set("User-Agent", "bqip-ingestor/0.1");
        for header in &request.headers {
            call = call.set(&header.name, &header.value);
        }
        let response = match call.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(error) => return Err(IngestError::Http(error.to_string())),
        };
        let status = response.status();
        let status_text = response.status_text().to_string();
        let mut headers = response
            .headers_names()
            .into_iter()
            .filter_map(|name| {
                response.header(&name).map(|value| HeaderPair {
                    name,
                    value: value.to_string(),
                })
            })
            .collect::<Vec<_>>();
        headers.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.value.cmp(&right.value))
        });
        let mut body = Vec::new();
        let mut reader = response
            .into_reader()
            .take(request.max_body_bytes as u64 + 1);
        reader.read_to_end(&mut body)?;
        if body.len() > request.max_body_bytes {
            return Err(IngestError::BodyTooLarge {
                limit: request.max_body_bytes,
                actual: body.len(),
            });
        }
        let observation = EndpointObservation {
            headers_hash: hash_headers(&headers),
            body_hash: *blake3::hash(&body).as_bytes(),
            request,
            status,
            status_text,
            headers,
            body,
            fetched_at_unix_millis,
            duration_millis: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        };
        observation.validate()?;
        Ok(observation)
    }

    pub fn fetch_to_sense_event(
        &self,
        request: EndpointRequest,
    ) -> Result<SenseEvent, IngestError> {
        self.fetch(request)?.to_sense_event(&self.compiler)
    }

    pub fn fetch_to_resolved_register_events(
        &self,
        request: EndpointRequest,
        node_public_key: &[u8; REGISTER_BYTES],
    ) -> Result<(EndpointObservation, RegisterIngestReport, Vec<SenseEvent>), IngestError> {
        let observed_at = unix_millis()?;
        let report = resolve_endpoint_request_registers(&request, node_public_key, observed_at)?;
        let observation = self.fetch(request)?;
        let mut events = Vec::with_capacity(report.registers.len());
        for register in &report.registers {
            events.push(observation.to_sense_event_for_register(&self.compiler, register)?);
        }
        Ok((observation, report, events))
    }

    pub fn observations_to_corpus(
        &self,
        vocab_size: usize,
        max_context: usize,
        observations: &[EndpointObservation],
    ) -> Result<TrainingCorpus, IngestError> {
        if observations.is_empty() {
            return Err(IngestError::EmptyObservationSet);
        }
        let mut documents = Vec::with_capacity(observations.len());
        for observation in observations {
            documents.push(observation.tokenized(&self.compiler)?);
        }
        Ok(TrainingCorpus::from_tokenized_documents(
            vocab_size,
            max_context,
            &documents,
        )?)
    }
}

pub fn huggingface_register_targets() -> Vec<RegisterTarget> {
    [
        ("huggingface.co", EndpointKind::Website),
        ("huggingface.co", EndpointKind::PublicApi),
        ("api-inference.huggingface.co", EndpointKind::PublicApi),
        ("router.huggingface.co", EndpointKind::PublicApi),
        ("datasets-server.huggingface.co", EndpointKind::PublicApi),
        ("cdn-lfs.huggingface.co", EndpointKind::PublicProxy),
        ("cdn-lfs-us-1.huggingface.co", EndpointKind::PublicProxy),
        ("cdn-lfs-eu-1.huggingface.co", EndpointKind::PublicProxy),
    ]
    .into_iter()
    .map(|(hostname, endpoint_kind)| {
        RegisterTarget::new(hostname, HTTPS_PORT, endpoint_kind)
            .expect("static Hugging Face register target must validate")
    })
    .collect()
}

pub fn huggingface_endpoint_requests() -> Result<Vec<EndpointRequest>, IngestError> {
    Ok(vec![
        EndpointRequest::get("https://huggingface.co/", "Hugging Face Hub")?,
        EndpointRequest::get(
            "https://huggingface.co/api/models?limit=1",
            "Hugging Face Models API",
        )?,
        EndpointRequest::get(
            "https://huggingface.co/api/datasets?limit=1",
            "Hugging Face Datasets API",
        )?,
        EndpointRequest::get(
            "https://huggingface.co/api/spaces?limit=1",
            "Hugging Face Spaces API",
        )?,
    ])
}

pub fn resolve_huggingface_registers(
    node_public_key: &[u8; REGISTER_BYTES],
    observed_at_unix_millis: u64,
) -> Result<RegisterIngestReport, IngestError> {
    resolve_register_targets(
        &huggingface_register_targets(),
        node_public_key,
        observed_at_unix_millis,
    )
}

pub fn resolve_endpoint_request_registers(
    request: &EndpointRequest,
    node_public_key: &[u8; REGISTER_BYTES],
    observed_at_unix_millis: u64,
) -> Result<RegisterIngestReport, IngestError> {
    request.validate()?;
    let parsed = Url::parse(&request.url)?;
    let hostname = parsed.host_str().ok_or(IngestError::MissingHost)?;
    let port = parsed
        .port_or_known_default()
        .ok_or(IngestError::InvalidLimit("port"))?;
    resolve_register_targets(
        &[RegisterTarget::new(
            hostname,
            port,
            request.endpoint_kind.clone(),
        )?],
        node_public_key,
        observed_at_unix_millis,
    )
}

pub fn resolve_register_targets(
    targets: &[RegisterTarget],
    node_public_key: &[u8; REGISTER_BYTES],
    observed_at_unix_millis: u64,
) -> Result<RegisterIngestReport, IngestError> {
    if targets.is_empty() {
        return Err(IngestError::EmptyRegisterTargetSet);
    }
    let mut registers = Vec::new();
    let mut failures = Vec::new();
    let mut seen = HashSet::new();

    for target in targets {
        target.validate()?;
        match (target.hostname.as_str(), target.port).to_socket_addrs() {
            Ok(addresses) => {
                for address in addresses {
                    let register = ResolvedEndpointRegister::new(
                        target.hostname.clone(),
                        address.ip(),
                        target.port,
                        target.endpoint_kind.clone(),
                        node_public_key,
                        observed_at_unix_millis,
                    )?;
                    let key = (
                        register.hostname.clone(),
                        register.ip_address.clone(),
                        register.port,
                        String::from_utf8_lossy(register.endpoint_kind.tag()).into_owned(),
                    );
                    if seen.insert(key) {
                        registers.push(register);
                    }
                }
            }
            Err(error) => failures.push(ResolutionFailure {
                hostname: target.hostname.clone(),
                port: target.port,
                error: error.to_string(),
            }),
        }
    }

    registers.sort_by(|left, right| {
        left.hostname
            .cmp(&right.hostname)
            .then_with(|| left.ip_address.cmp(&right.ip_address))
            .then_with(|| left.endpoint_kind.tag().cmp(right.endpoint_kind.tag()))
    });
    let report = RegisterIngestReport {
        observed_at_unix_millis,
        target_count: targets.len(),
        registers,
        failures,
    };
    report.validate()?;
    Ok(report)
}

fn classify_url(url: &Url) -> (RegisterLane, EndpointKind) {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();
    if host.starts_with("api.") || path.contains("/api/") || path.ends_with("/api") {
        (RegisterLane::PublicApi, EndpointKind::PublicApi)
    } else if host.contains("cdn") || host.contains("proxy") {
        (RegisterLane::PublicProxy, EndpointKind::PublicProxy)
    } else if path.ends_with(".json") || path.ends_with(".xml") || path.ends_with(".rss") {
        (RegisterLane::GenericEndpoint, EndpointKind::Feed)
    } else {
        (RegisterLane::GenericEndpoint, EndpointKind::Website)
    }
}

fn url_slot_index(url: &str) -> u64 {
    let hash = blake3::hash(url.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

fn register_slot_index(hostname: &str, canonical_address: &[u8]) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-resolved-endpoint-register");
    hasher.update(hostname.as_bytes());
    hasher.update(canonical_address);
    let hash = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

fn hash_headers(headers: &[HeaderPair]) -> [u8; REGISTER_BYTES] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"bqip-ingestor-headers");
    for header in headers {
        hasher.update(header.name.to_ascii_lowercase().as_bytes());
        hasher.update(&[0]);
        hasher.update(header.value.as_bytes());
        hasher.update(&[0xff]);
    }
    *hasher.finalize().as_bytes()
}

fn unix_millis() -> Result<u64, IngestError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| IngestError::ClockBeforeUnixEpoch)?;
    Ok(duration.as_millis().min(u128::from(u64::MAX)) as u64)
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("endpoint URL parse failed: {0}")]
    Url(#[from] url::ParseError),
    #[error("unsupported endpoint URL scheme: {0}")]
    UnsupportedScheme(String),
    #[error("endpoint URL must contain a host")]
    MissingHost,
    #[error("concept label must be nonempty")]
    EmptyConceptLabel,
    #[error("register target hostname must be nonempty")]
    EmptyHostname,
    #[error("register target hostname is invalid")]
    InvalidHostname,
    #[error("register canonical address must be nonempty")]
    EmptyCanonicalRegisterAddress,
    #[error("resolved IP address is invalid")]
    InvalidResolvedIp,
    #[error("register lane does not match resolved IP address family")]
    RegisterLaneIpMismatch,
    #[error("register target set must be nonempty")]
    EmptyRegisterTargetSet,
    #[error("resolved register set must contain at least one IPv4 or IPv6 address")]
    EmptyResolvedRegisterSet,
    #[error("resolved register hostname does not match endpoint observation hostname")]
    ResolvedRegisterHostMismatch,
    #[error("invalid request limit: {0}")]
    InvalidLimit(&'static str),
    #[error("invalid HTTP header: {0}")]
    InvalidHeader(&'static str),
    #[error("HTTP fetch failed: {0}")]
    Http(String),
    #[error("HTTP response status must be nonzero")]
    InvalidStatus,
    #[error("HTTP response body is empty")]
    EmptyBody,
    #[error("HTTP body exceeds limit: limit={limit}, actual={actual}")]
    BodyTooLarge { limit: usize, actual: usize },
    #[error("HTTP response body hash mismatch")]
    BodyHashMismatch,
    #[error("HTTP response header hash mismatch")]
    HeaderHashMismatch,
    #[error("endpoint observation set must be nonempty")]
    EmptyObservationSet,
    #[error("system clock is before Unix epoch")]
    ClockBeforeUnixEpoch,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Training(#[from] TrainingError),
}
