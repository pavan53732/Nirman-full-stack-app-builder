#![deny(unsafe_op_in_unsafe_fn)]

use nirman_domain::{ProjectId, ProviderUsageRecord};
use nirman_storage::Ledger;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Provider protocol surfaces supported by the M3 foundation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderProtocol {
    #[serde(rename = "chat_completions")]
    ChatCompletions,
    #[serde(rename = "responses")]
    Responses,
    #[serde(rename = "messages")]
    Messages,
    #[serde(rename = "custom")]
    Custom,
}

impl ProviderProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
            Self::Messages => "messages",
            Self::Custom => "custom",
        }
    }
}

/// Opaque reference to a credential stored in the operating-system keychain.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CredentialReference(String);

impl CredentialReference {
    pub fn new(reference: impl Into<String>) -> Result<Self, ProviderConfigError> {
        let reference = reference.into();
        if reference.trim().is_empty()
            || !(reference.starts_with("keychain://") || reference.starts_with("credential://"))
        {
            return Err(ProviderConfigError::InvalidCredentialReference);
        }
        Ok(Self(reference))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Secret material obtained by a future OS-keychain adapter.
///
/// This type intentionally has no `Debug`, `Serialize`, or `Display` implementation.
#[derive(Clone)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn from_keychain(value: impl Into<String>) -> Result<Self, ProviderConfigError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ProviderConfigError::MissingCredential);
        }
        Ok(Self(value))
    }
}

/// Credential-resolution boundary. Implementations obtain material from an OS keychain or an equivalent secure vault.
pub trait CredentialResolver: Send + Sync {
    fn resolve(&self, reference: &CredentialReference) -> Result<SecretValue, ProviderConfigError>;
}

/// Windows host credential resolver backed by Windows Credential Manager.
///
/// M3 stores API keys as Generic credentials. The Credential Manager target is
/// the opaque `CredentialReference` URI itself; the credential blob is read
/// only inside this authority boundary and is never serialized or logged.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsCredentialResolver;

impl CredentialResolver for OsCredentialResolver {
    fn resolve(&self, reference: &CredentialReference) -> Result<SecretValue, ProviderConfigError> {
        CredentialReference::new(reference.as_str().to_owned())?;
        #[cfg(windows)]
        {
            return resolve_windows_credential(reference);
        }
        #[cfg(not(windows))]
        {
            let _ = reference;
            Err(ProviderConfigError::CredentialStoreUnavailable)
        }
    }
}

#[cfg(windows)]
fn resolve_windows_credential(
    reference: &CredentialReference,
) -> Result<SecretValue, ProviderConfigError> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let target: Vec<u16> = reference
        .as_str()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut credential: *mut CREDENTIALW = null_mut();
    // SAFETY: `target` is NUL-terminated for the duration of the call and
    // Windows allocates the output credential which is released with CredFree.
    let read = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if read == 0 || credential.is_null() {
        return Err(ProviderConfigError::MissingCredential);
    }

    // SAFETY: Credential Manager owns a valid credential until CredFree; the
    // blob size is supplied by that same structure and bounds the read.
    let result = unsafe {
        let credential_ref = &*credential;
        if credential_ref.CredentialBlob.is_null() || credential_ref.CredentialBlobSize == 0 {
            Err(ProviderConfigError::MissingCredential)
        } else {
            let bytes = std::slice::from_raw_parts(
                credential_ref.CredentialBlob,
                credential_ref.CredentialBlobSize as usize,
            );
            let value = std::str::from_utf8(bytes)
                .map_err(|_| ProviderConfigError::MissingCredential)
                .and_then(SecretValue::from_keychain);
            value
        }
    };
    // SAFETY: The pointer was returned by CredReadW and has not been freed.
    unsafe { CredFree(credential.cast()) };
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderConfigError {
    InvalidProviderId,
    InvalidDisplayName,
    InvalidBaseUrl,
    InvalidModelId,
    InvalidCredentialReference,
    MissingCredential,
    CredentialStoreUnavailable,
    InvalidTimeout,
    Disabled,
}

impl fmt::Display for ProviderConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidProviderId => "provider id is required",
            Self::InvalidDisplayName => "provider display name is required",
            Self::InvalidBaseUrl => "provider base URL must use HTTP or HTTPS",
            Self::InvalidModelId => "provider model id is required",
            Self::InvalidCredentialReference => "credential must be an OS-keychain reference",
            Self::MissingCredential => "provider credential is unavailable",
            Self::CredentialStoreUnavailable => "operating-system credential store is unavailable",
            Self::InvalidTimeout => "provider timeout must be between 1 ms and one hour",
            Self::Disabled => "provider profile is disabled",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ProviderConfigError {}

/// M3 provider profile. Sensitive values are represented only by references.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderProfile {
    pub provider_id: String,
    pub display_name: String,
    pub protocol: ProviderProtocol,
    pub base_url: String,
    pub api_key_secret_ref: CredentialReference,
    pub model_id: String,
    pub timeout_ms: u64,
    pub enabled: bool,
}

impl ProviderProfile {
    pub fn validate(&self) -> Result<(), ProviderConfigError> {
        if self.provider_id.trim().is_empty() {
            return Err(ProviderConfigError::InvalidProviderId);
        }
        if self.display_name.trim().is_empty() {
            return Err(ProviderConfigError::InvalidDisplayName);
        }
        let valid_scheme =
            self.base_url.starts_with("https://") || self.base_url.starts_with("http://");
        if !valid_scheme || self.base_url.trim_end_matches('/').len() <= 8 {
            return Err(ProviderConfigError::InvalidBaseUrl);
        }
        if self.model_id.trim().is_empty() {
            return Err(ProviderConfigError::InvalidModelId);
        }
        if !(1..=3_600_000).contains(&self.timeout_ms) {
            return Err(ProviderConfigError::InvalidTimeout);
        }
        if !self.enabled {
            return Err(ProviderConfigError::Disabled);
        }
        CredentialReference::new(self.api_key_secret_ref.as_str().to_owned())?;
        Ok(())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

/// Minimal canonical input for one M3 provider request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderRequestInput {
    pub prompt: String,
    pub max_output_tokens: Option<u64>,
}

impl ProviderRequestInput {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_output_tokens: None,
        }
    }
}

/// Cooperative cancellation signal shared by the provider runtime and transport.
#[derive(Clone, Default)]
pub struct CancellationSignal(Arc<AtomicBool>);

impl CancellationSignal {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Safe request metadata and body. Credential material is kept private.
pub struct AuthenticatedProviderRequest {
    profile: ProviderProfile,
    credential: SecretValue,
    request_id: String,
    correlation_id: String,
    body: Value,
    cancellation: CancellationSignal,
}

impl AuthenticatedProviderRequest {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn correlation_id(&self) -> &str {
        &self.correlation_id
    }

    pub fn provider_id(&self) -> &str {
        &self.profile.provider_id
    }

    pub fn model_id(&self) -> &str {
        &self.profile.model_id
    }

    pub fn protocol(&self) -> ProviderProtocol {
        self.profile.protocol
    }

    pub fn endpoint(&self) -> &str {
        &self.profile.base_url
    }

    pub fn timeout(&self) -> Duration {
        self.profile.timeout()
    }

    pub fn body(&self) -> &Value {
        &self.body
    }

    pub fn cancellation(&self) -> &CancellationSignal {
        &self.cancellation
    }

    fn authorization_header(&self) -> String {
        format!("Bearer {}", self.credential.0)
    }
}

fn request_body(profile: &ProviderProfile, input: &ProviderRequestInput) -> Value {
    let content = json!([{"role": "user", "content": input.prompt}]);
    let mut body = match profile.protocol {
        ProviderProtocol::ChatCompletions | ProviderProtocol::Messages => {
            json!({"model": profile.model_id, "messages": content})
        }
        ProviderProtocol::Responses => json!({"model": profile.model_id, "input": input.prompt}),
        ProviderProtocol::Custom => json!({"model": profile.model_id, "prompt": input.prompt}),
    };
    if let Some(max_output_tokens) = input.max_output_tokens {
        if let Some(object) = body.as_object_mut() {
            object.insert("max_output_tokens".into(), json!(max_output_tokens));
        }
    }
    body
}

pub fn build_authenticated_request(
    profile: &ProviderProfile,
    credential: SecretValue,
    input: ProviderRequestInput,
    request_id: impl Into<String>,
    correlation_id: impl Into<String>,
    cancellation: CancellationSignal,
) -> Result<AuthenticatedProviderRequest, ProviderConfigError> {
    profile.validate()?;
    Ok(AuthenticatedProviderRequest {
        profile: profile.clone(),
        credential,
        request_id: request_id.into(),
        correlation_id: correlation_id.into(),
        body: request_body(profile, &input),
        cancellation,
    })
}

/// Raw provider response returned by a transport adapter.
pub struct RawProviderResponse {
    pub status_code: u16,
    pub body: String,
    pub provider_request_id: Option<String>,
}

/// Normalized token usage when the provider reports it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct NormalizedUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// M3 normalized successful provider response.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NormalizedProviderResponse {
    pub request_id: String,
    pub correlation_id: String,
    pub provider_request_id: Option<String>,
    pub model_id: String,
    pub text: String,
    pub finish_reason: Option<String>,
    pub usage: NormalizedUsage,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderErrorKind {
    #[serde(rename = "authentication")]
    Authentication,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "cancellation")]
    Cancellation,
    #[serde(rename = "rate_limited")]
    RateLimited,
    #[serde(rename = "transport")]
    Transport,
    #[serde(rename = "invalid_response")]
    InvalidResponse,
    #[serde(rename = "remote")]
    Remote,
}

/// Normalized provider failure. It contains safe text only and never raw response bodies.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderError {
    kind: ProviderErrorKind,
    safe_message: String,
    retryable: bool,
    status_code: Option<u16>,
}

impl ProviderError {
    /// Construct a fixed-message error for external transport adapters.
    pub fn classified(kind: ProviderErrorKind, status_code: Option<u16>) -> Self {
        let (message, retryable) = match kind {
            ProviderErrorKind::Authentication => ("provider authentication was rejected", false),
            ProviderErrorKind::Timeout => ("provider request timed out", true),
            ProviderErrorKind::Cancellation => ("provider request was cancelled", false),
            ProviderErrorKind::RateLimited => ("provider rate limit was reached", true),
            ProviderErrorKind::Transport => ("provider service is unavailable", true),
            ProviderErrorKind::InvalidResponse => ("provider returned an invalid response", false),
            ProviderErrorKind::Remote => ("provider rejected the request", false),
        };
        Self::new(kind, message, retryable, status_code)
    }

    pub fn kind(&self) -> ProviderErrorKind {
        self.kind
    }

    pub fn safe_message(&self) -> &str {
        &self.safe_message
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }

    pub fn status_code(&self) -> Option<u16> {
        self.status_code
    }

    fn new(
        kind: ProviderErrorKind,
        message: &'static str,
        retryable: bool,
        status_code: Option<u16>,
    ) -> Self {
        Self {
            kind,
            safe_message: message.into(),
            retryable,
            status_code,
        }
    }

    pub fn is_timeout(&self) -> bool {
        self.kind == ProviderErrorKind::Timeout
    }
}

pub trait ProviderTransport: Send + Sync {
    fn send(
        &self,
        request: &AuthenticatedProviderRequest,
    ) -> Result<RawProviderResponse, ProviderError>;
}

/// Real HTTP transport used by the future M22 gateway and Android construction pipeline.
pub struct HttpProviderTransport {
    client: Client,
}

impl HttpProviderTransport {
    pub fn new() -> Result<Self, ProviderError> {
        Client::builder()
            .build()
            .map(|client| Self { client })
            .map_err(|_| {
                ProviderError::new(
                    ProviderErrorKind::Transport,
                    "provider HTTP client could not be initialized",
                    true,
                    None,
                )
            })
    }
}

impl ProviderTransport for HttpProviderTransport {
    fn send(
        &self,
        request: &AuthenticatedProviderRequest,
    ) -> Result<RawProviderResponse, ProviderError> {
        if request.cancellation().is_cancelled() {
            return Err(ProviderError::new(
                ProviderErrorKind::Cancellation,
                "provider request was cancelled before transport",
                false,
                None,
            ));
        }
        let response = self
            .client
            .post(request.endpoint())
            .timeout(request.timeout())
            .header(
                reqwest::header::AUTHORIZATION,
                request.authorization_header(),
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(request.body())
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    ProviderError::new(
                        ProviderErrorKind::Timeout,
                        "provider request exceeded its timeout",
                        true,
                        None,
                    )
                } else {
                    ProviderError::new(
                        ProviderErrorKind::Transport,
                        "provider transport failed",
                        true,
                        None,
                    )
                }
            })?;
        let status_code = response.status().as_u16();
        let provider_request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response.text().map_err(|_| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "provider response body could not be read",
                false,
                Some(status_code),
            )
        })?;
        Ok(RawProviderResponse {
            status_code,
            body,
            provider_request_id,
        })
    }
}

fn classify_status(status_code: u16) -> Option<ProviderError> {
    match status_code {
        401 | 403 => Some(ProviderError::new(
            ProviderErrorKind::Authentication,
            "provider authentication was rejected",
            false,
            Some(status_code),
        )),
        408 | 504 => Some(ProviderError::new(
            ProviderErrorKind::Timeout,
            "provider request timed out",
            true,
            Some(status_code),
        )),
        429 => Some(ProviderError::new(
            ProviderErrorKind::RateLimited,
            "provider rate limit was reached",
            true,
            Some(status_code),
        )),
        400..=499 => Some(ProviderError::new(
            ProviderErrorKind::Remote,
            "provider rejected the request",
            false,
            Some(status_code),
        )),
        500..=599 => Some(ProviderError::new(
            ProviderErrorKind::Transport,
            "provider service is unavailable",
            true,
            Some(status_code),
        )),
        _ => None,
    }
}

fn value_u64(value: &Value, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_u64))
}

fn normalize_usage(value: Option<&Value>) -> NormalizedUsage {
    let Some(value) = value else {
        return NormalizedUsage::default();
    };
    NormalizedUsage {
        input_tokens: value_u64(value, &["prompt_tokens", "input_tokens"]),
        output_tokens: value_u64(value, &["completion_tokens", "output_tokens"]),
        total_tokens: value_u64(value, &["total_tokens"]),
    }
}

fn response_text(body: &Value) -> Option<String> {
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .or_else(|| body.get("output_text").and_then(Value::as_str))
        .or_else(|| {
            body.pointer("/output/0/content/0/text")
                .and_then(Value::as_str)
        })
        .or_else(|| body.pointer("/message/content").and_then(Value::as_str))
        .map(str::to_owned)
}

fn normalize_response(
    request: &AuthenticatedProviderRequest,
    raw: RawProviderResponse,
) -> Result<NormalizedProviderResponse, ProviderError> {
    if let Some(error) = classify_status(raw.status_code) {
        return Err(error);
    }
    let body: Value = serde_json::from_str(&raw.body).map_err(|_| {
        ProviderError::new(
            ProviderErrorKind::InvalidResponse,
            "provider returned invalid JSON",
            false,
            Some(raw.status_code),
        )
    })?;
    let text = response_text(&body)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "provider response did not contain assistant text",
                false,
                Some(raw.status_code),
            )
        })?;
    Ok(NormalizedProviderResponse {
        request_id: request.request_id.clone(),
        correlation_id: request.correlation_id.clone(),
        provider_request_id: raw.provider_request_id,
        model_id: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(request.model_id())
            .to_owned(),
        text,
        finish_reason: body
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .map(str::to_owned),
        usage: normalize_usage(body.get("usage")),
    })
}

#[derive(Debug)]
pub enum ProviderRuntimeError {
    Configuration(ProviderConfigError),
    Provider(ProviderError),
    UsagePersistence,
}

impl fmt::Display for ProviderRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => error.fmt(f),
            Self::Provider(error) => f.write_str(&error.safe_message),
            Self::UsagePersistence => f.write_str("provider usage could not be recorded durably"),
        }
    }
}

impl std::error::Error for ProviderRuntimeError {}

/// Provider result plus the durable, secret-free usage record produced for it.
#[derive(Debug)]
pub struct ProviderExecution {
    pub response: NormalizedProviderResponse,
    pub usage: ProviderUsageRecord,
}

/// M3 runtime facade. It owns durable usage writes but not M22 orchestration.
pub struct ProviderRuntime {
    ledger: Ledger,
}

impl ProviderRuntime {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            ledger: Ledger::open(path)?,
        })
    }

    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        Ok(Self {
            ledger: Ledger::open_in_memory()?,
        })
    }

    pub fn usage_record(
        &self,
        request_id: &str,
    ) -> Result<Option<ProviderUsageRecord>, rusqlite::Error> {
        self.ledger.provider_usage(request_id)
    }

    pub fn execute<T: ProviderTransport + ?Sized, R: CredentialResolver + ?Sized>(
        &self,
        profile: &ProviderProfile,
        input: ProviderRequestInput,
        request_id: impl Into<String>,
        correlation_id: impl Into<String>,
        project_id: ProjectId,
        cancellation: CancellationSignal,
        resolver: &R,
        transport: &T,
    ) -> Result<ProviderExecution, ProviderRuntimeError> {
        let request_id = request_id.into();
        let correlation_id = correlation_id.into();
        let started_at_epoch_seconds = now_epoch_seconds();
        let started = Instant::now();
        let credential = resolver
            .resolve(&profile.api_key_secret_ref)
            .map_err(ProviderRuntimeError::Configuration)?;
        let request = match build_authenticated_request(
            profile,
            credential,
            input,
            request_id.clone(),
            correlation_id.clone(),
            cancellation.clone(),
        ) {
            Ok(request) => request,
            Err(error) => return Err(ProviderRuntimeError::Configuration(error)),
        };
        let result = if cancellation.is_cancelled() {
            Err(ProviderError::new(
                ProviderErrorKind::Cancellation,
                "provider request was cancelled",
                false,
                None,
            ))
        } else {
            transport.send(&request).and_then(|raw| {
                if cancellation.is_cancelled() {
                    Err(ProviderError::new(
                        ProviderErrorKind::Cancellation,
                        "provider request was cancelled",
                        false,
                        None,
                    ))
                } else {
                    normalize_response(&request, raw)
                }
            })
        };
        let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let (outcome, usage) = match &result {
            Ok(response) => ("completed", response.usage.clone()),
            Err(error) => {
                let outcome = match error.kind {
                    ProviderErrorKind::Authentication => "authentication_failed",
                    ProviderErrorKind::Timeout => "timed_out",
                    ProviderErrorKind::Cancellation => "cancelled",
                    ProviderErrorKind::RateLimited => "rate_limited",
                    ProviderErrorKind::Transport => "transport_failed",
                    ProviderErrorKind::InvalidResponse => "invalid_response",
                    ProviderErrorKind::Remote => "provider_rejected",
                };
                (outcome, NormalizedUsage::default())
            }
        };
        let usage_record = ProviderUsageRecord {
            request_id,
            correlation_id,
            project_id,
            provider_id: profile.provider_id.clone(),
            model_id: profile.model_id.clone(),
            started_at_epoch_seconds,
            duration_ms,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            outcome: outcome.into(),
        };
        self.ledger
            .record_provider_usage(&usage_record)
            .map_err(|_| ProviderRuntimeError::UsagePersistence)?;
        result
            .map(|response| ProviderExecution {
                response,
                usage: usage_record,
            })
            .map_err(ProviderRuntimeError::Provider)
    }
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_profile_requires_keychain_reference_and_valid_endpoint() {
        let reference =
            CredentialReference::new("keychain://nirman/provider/openai").expect("reference");
        let profile = ProviderProfile {
            provider_id: "provider-1".into(),
            display_name: "Local test provider".into(),
            protocol: ProviderProtocol::ChatCompletions,
            base_url: "https://provider.example/v1/chat/completions".into(),
            api_key_secret_ref: reference,
            model_id: "model-1".into(),
            timeout_ms: 5000,
            enabled: true,
        };
        profile.validate().expect("valid profile");
        assert!(CredentialReference::new("sk-secret-value").is_err());
        assert!(serde_json::to_string(&profile)
            .expect("profile serialization")
            .contains("keychain://"));
    }

    #[test]
    fn authenticated_request_contains_identity_and_never_serializes_credential() {
        let profile = ProviderProfile {
            provider_id: "provider-1".into(),
            display_name: "Test".into(),
            protocol: ProviderProtocol::Responses,
            base_url: "https://provider.example/responses".into(),
            api_key_secret_ref: CredentialReference::new("keychain://secret").expect("ref"),
            model_id: "model-1".into(),
            timeout_ms: 5000,
            enabled: true,
        };
        let request = build_authenticated_request(
            &profile,
            SecretValue::from_keychain("raw-secret").expect("secret"),
            ProviderRequestInput::new("Build an Android app"),
            "request-1",
            "correlation-1",
            CancellationSignal::default(),
        )
        .expect("request");
        assert_eq!(request.request_id(), "request-1");
        assert_eq!(request.correlation_id(), "correlation-1");
        let body = request.body().to_string();
        assert!(!body.contains("raw-secret"));
        assert!(!body.contains("api_key"));
        assert!(!request.authorization_header().is_empty());
    }

    #[test]
    fn normalized_responses_and_failures_are_safe_and_typed() {
        let profile = ProviderProfile {
            provider_id: "provider-1".into(),
            display_name: "Test".into(),
            protocol: ProviderProtocol::ChatCompletions,
            base_url: "https://provider.example/chat".into(),
            api_key_secret_ref: CredentialReference::new("keychain://secret").expect("ref"),
            model_id: "model-1".into(),
            timeout_ms: 5000,
            enabled: true,
        };
        let request = build_authenticated_request(
            &profile,
            SecretValue::from_keychain("raw-secret").expect("secret"),
            ProviderRequestInput::new("hello"),
            "request-1",
            "correlation-1",
            CancellationSignal::default(),
        )
        .expect("request");
        let response = normalize_response(
            &request,
            RawProviderResponse {
                status_code: 200,
                body: json!({
                    "id": "provider-request-1",
                    "model": "model-1",
                    "choices": [{"message": {"content": "hello"}, "finish_reason": "stop"}],
                    "usage": {"prompt_tokens": 3, "completion_tokens": 5, "total_tokens": 8}
                })
                .to_string(),
                provider_request_id: Some("provider-request-1".into()),
            },
        )
        .expect("normalized");
        assert_eq!(response.text, "hello");
        assert_eq!(response.usage.total_tokens, Some(8));
        assert_eq!(response.request_id, "request-1");
        for (status, kind) in [
            (401, ProviderErrorKind::Authentication),
            (408, ProviderErrorKind::Timeout),
            (429, ProviderErrorKind::RateLimited),
            (503, ProviderErrorKind::Transport),
        ] {
            let error = normalize_response(
                &request,
                RawProviderResponse {
                    status_code: status,
                    body: "{\"secret\":\"do-not-forward\"}".into(),
                    provider_request_id: None,
                },
            )
            .expect_err("typed failure");
            assert_eq!(error.kind, kind);
            assert!(!error.safe_message.contains("do-not-forward"));
            assert!(!error.safe_message.contains("secret"));
        }
    }

    struct FixtureTransport {
        response: Result<RawProviderResponse, ProviderError>,
    }

    struct FixtureResolver;

    impl CredentialResolver for FixtureResolver {
        fn resolve(
            &self,
            _reference: &CredentialReference,
        ) -> Result<SecretValue, ProviderConfigError> {
            SecretValue::from_keychain("raw-secret")
        }
    }

    impl ProviderTransport for FixtureTransport {
        fn send(
            &self,
            _request: &AuthenticatedProviderRequest,
        ) -> Result<RawProviderResponse, ProviderError> {
            match &self.response {
                Ok(response) => Ok(RawProviderResponse {
                    status_code: response.status_code,
                    body: response.body.clone(),
                    provider_request_id: response.provider_request_id.clone(),
                }),
                Err(error) => Err(error.clone()),
            }
        }
    }

    fn test_profile() -> ProviderProfile {
        ProviderProfile {
            provider_id: "provider-1".into(),
            display_name: "Test".into(),
            protocol: ProviderProtocol::ChatCompletions,
            base_url: "https://provider.example/chat".into(),
            api_key_secret_ref: CredentialReference::new("keychain://secret").expect("ref"),
            model_id: "model-1".into(),
            timeout_ms: 50,
            enabled: true,
        }
    }

    fn success_transport() -> FixtureTransport {
        FixtureTransport {
            response: Ok(RawProviderResponse {
                status_code: 200,
                body: json!({"model":"model-1","choices":[{"message":{"content":"ok"}}],"usage":{"total_tokens":2}}).to_string(),
                provider_request_id: Some("provider-request-1".into()),
            }),
        }
    }

    #[test]
    fn os_credential_resolver_rejects_invalid_and_missing_references_without_secret_leakage() {
        assert_eq!(
            CredentialReference::new("raw-api-key").expect_err("raw value must be rejected"),
            ProviderConfigError::InvalidCredentialReference
        );
        let missing = CredentialReference::new("credential://nirman/tests/missing").expect("ref");
        let result = OsCredentialResolver.resolve(&missing);
        #[cfg(windows)]
        assert!(matches!(
            result,
            Err(ProviderConfigError::MissingCredential)
        ));
        #[cfg(not(windows))]
        assert!(matches!(
            result,
            Err(ProviderConfigError::CredentialStoreUnavailable)
        ));
        let safe_error = match result {
            Err(error) => error.to_string(),
            Ok(_) => panic!("missing credential must fail"),
        };
        assert!(!safe_error.contains("raw-api-key"));
        assert!(!safe_error.contains("credential://nirman/tests/missing"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_credential_manager_resolves_generic_credential_without_serializing_secret() {
        use std::ptr::null_mut;
        use windows_sys::Win32::Security::Credentials::{
            CredDeleteW, CredWriteW, CREDENTIALW, CRED_PERSIST_SESSION, CRED_TYPE_GENERIC,
        };

        let target_text = format!(
            "credential://nirman/tests/provider/{}/{}",
            std::process::id(),
            now_epoch_seconds()
        );
        let reference = CredentialReference::new(target_text.clone()).expect("ref");
        let secret = "windows-only-test-secret";
        let mut target: Vec<u16> = target_text
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut blob = secret.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: null_mut(),
            LastWritten: windows_sys::Win32::Foundation::FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_SESSION,
            AttributeCount: 0,
            Attributes: null_mut(),
            TargetAlias: null_mut(),
            UserName: null_mut(),
        };
        // SAFETY: all pointers refer to NUL-terminated or length-bounded data
        // owned by this test for the duration of the Windows API call.
        let wrote = unsafe { CredWriteW(&credential, 0) };
        assert_ne!(wrote, 0, "Credential Manager test write failed");

        let resolved = OsCredentialResolver
            .resolve(&reference)
            .expect("Windows Credential Manager lookup");
        assert_eq!(resolved.0, secret);
        let profile_json = serde_json::to_string(&ProviderProfile {
            provider_id: "provider-test".into(),
            display_name: "Windows credential test".into(),
            protocol: ProviderProtocol::ChatCompletions,
            base_url: "https://provider.example/v1".into(),
            api_key_secret_ref: reference,
            model_id: "model-test".into(),
            timeout_ms: 1000,
            enabled: true,
        })
        .expect("profile serialization");
        assert!(!profile_json.contains(secret));
        assert!(!profile_json.contains("api_key"));

        // SAFETY: the target remains NUL-terminated and the credential is
        // deleted under the same Generic target/type used for the test write.
        let deleted = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        assert_ne!(deleted, 0, "Credential Manager test cleanup failed");
    }

    #[test]
    fn runtime_records_durable_success_usage_with_request_and_correlation_identity() {
        let runtime = ProviderRuntime::in_memory().expect("runtime");
        let execution = runtime
            .execute(
                &test_profile(),
                ProviderRequestInput::new("hello"),
                "request-1",
                "correlation-1",
                ProjectId("project-1".into()),
                CancellationSignal::default(),
                &FixtureResolver,
                &success_transport(),
            )
            .expect("execution");
        assert_eq!(execution.response.text, "ok");
        let usage = runtime
            .usage_record("request-1")
            .expect("usage")
            .expect("record");
        assert_eq!(usage.correlation_id, "correlation-1");
        assert_eq!(usage.outcome, "completed");
        assert_eq!(usage.total_tokens, Some(2));
        let serialized = serde_json::to_string(&usage).expect("usage json");
        assert!(!serialized.contains("raw-secret"));
    }

    #[test]
    fn runtime_records_timeout_and_cancellation_without_leaking_provider_details() {
        let runtime = ProviderRuntime::in_memory().expect("runtime");
        let timeout = FixtureTransport {
            response: Err(ProviderError::new(
                ProviderErrorKind::Timeout,
                "provider request exceeded its timeout",
                true,
                None,
            )),
        };
        let timeout_result = runtime.execute(
            &test_profile(),
            ProviderRequestInput::new("hello"),
            "request-timeout",
            "correlation-timeout",
            ProjectId("project-1".into()),
            CancellationSignal::default(),
            &FixtureResolver,
            &timeout,
        );
        assert!(
            matches!(timeout_result, Err(ProviderRuntimeError::Provider(error)) if error.is_timeout())
        );
        assert_eq!(
            runtime
                .usage_record("request-timeout")
                .expect("usage")
                .expect("record")
                .outcome,
            "timed_out"
        );

        let cancellation = CancellationSignal::default();
        cancellation.cancel();
        let cancellation_result = runtime.execute(
            &test_profile(),
            ProviderRequestInput::new("hello"),
            "request-cancelled",
            "correlation-cancelled",
            ProjectId("project-1".into()),
            cancellation,
            &FixtureResolver,
            &success_transport(),
        );
        assert!(
            matches!(cancellation_result, Err(ProviderRuntimeError::Provider(error)) if error.kind == ProviderErrorKind::Cancellation)
        );
        assert_eq!(
            runtime
                .usage_record("request-cancelled")
                .expect("usage")
                .expect("record")
                .outcome,
            "cancelled"
        );
    }
}
