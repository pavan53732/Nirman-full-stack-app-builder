use nirman_domain::ProjectId;
use nirman_providers::{
    build_authenticated_request, CancellationSignal, CredentialReference, HttpProviderTransport,
    ProviderError, ProviderErrorKind, ProviderExecution, ProviderProfile, ProviderProtocol,
    ProviderRequestInput, ProviderRuntime, ProviderRuntimeError, ProviderTransport,
    RawProviderResponse, SecretValue,
};
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn database_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nirman-m3-provider-{nonce}.sqlite3"))
}

fn profile() -> ProviderProfile {
    ProviderProfile {
        provider_id: "provider-m3".into(),
        display_name: "M3 fixture provider".into(),
        protocol: ProviderProtocol::ChatCompletions,
        base_url: "https://provider.example/v1/chat/completions".into(),
        api_key_secret_ref: CredentialReference::new("keychain://nirman/m3/provider")
            .expect("credential reference"),
        model_id: "model-m3".into(),
        timeout_ms: 250,
        enabled: true,
    }
}

struct FixtureResolver;

impl nirman_providers::CredentialResolver for FixtureResolver {
    fn resolve(
        &self,
        _reference: &CredentialReference,
    ) -> Result<SecretValue, nirman_providers::ProviderConfigError> {
        SecretValue::from_keychain("raw-api-key")
    }
}

struct FixtureTransport {
    response: Result<RawProviderResponse, ProviderError>,
}

struct CancellableTransport;

impl ProviderTransport for CancellableTransport {
    fn send(
        &self,
        request: &nirman_providers::AuthenticatedProviderRequest,
    ) -> Result<RawProviderResponse, ProviderError> {
        for _ in 0..100 {
            if request.cancellation().is_cancelled() {
                return Err(ProviderError::classified(
                    ProviderErrorKind::Cancellation,
                    None,
                ));
            }
            thread::sleep(Duration::from_millis(5));
        }
        Err(ProviderError::classified(
            ProviderErrorKind::Transport,
            None,
        ))
    }
}

impl ProviderTransport for FixtureTransport {
    fn send(
        &self,
        _request: &nirman_providers::AuthenticatedProviderRequest,
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

fn success_transport() -> FixtureTransport {
    FixtureTransport {
        response: Ok(RawProviderResponse {
            status_code: 200,
            body: json!({
                "id": "provider-request-m3",
                "model": "model-m3",
                "choices": [{
                    "message": {"content": "normalized provider response"},
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 7,
                    "completion_tokens": 11,
                    "total_tokens": 18
                }
            })
            .to_string(),
            provider_request_id: Some("provider-request-m3".into()),
        }),
    }
}

fn assert_success(execution: &ProviderExecution) {
    assert_eq!(execution.response.text, "normalized provider response");
    assert_eq!(execution.response.request_id, "request-m3-success");
    assert_eq!(execution.response.correlation_id, "correlation-m3-success");
    assert_eq!(execution.response.usage.total_tokens, Some(18));
    assert_eq!(execution.usage.outcome, "completed");
}

#[test]
fn m3_provider_runtime_acceptance_is_file_backed_and_observation_derived(
) -> Result<(), Box<dyn std::error::Error>> {
    let database = database_path();
    let provider = profile();
    let profile_validated = provider.validate().is_ok();
    assert!(profile_validated);
    let credential_reference_only = CredentialReference::new("raw-api-key").is_err();
    assert!(credential_reference_only);

    let request = build_authenticated_request(
        &provider,
        SecretValue::from_keychain("raw-api-key")?,
        ProviderRequestInput::new("Build an Android notes app"),
        "request-m3-construction",
        "correlation-m3-construction",
        CancellationSignal::default(),
    )?;
    assert_eq!(request.request_id(), "request-m3-construction");
    assert_eq!(request.correlation_id(), "correlation-m3-construction");
    assert_eq!(request.endpoint(), provider.base_url);
    assert_eq!(request.protocol(), ProviderProtocol::ChatCompletions);
    let authenticated_request_constructed = request.endpoint() == provider.base_url
        && request.protocol() == ProviderProtocol::ChatCompletions
        && request.request_id() == "request-m3-construction"
        && request.correlation_id() == "correlation-m3-construction";
    assert!(authenticated_request_constructed);
    let secret_redaction_observed = !request.body().to_string().contains("raw-api-key")
        && !serde_json::to_string(&provider)?.contains("raw-api-key");
    assert!(secret_redaction_observed);

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/v1/chat/completions", listener.local_addr()?);
    let server = thread::spawn(move || -> Result<bool, String> {
        let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let mut buffer = [0_u8; 16 * 1024];
        let bytes_read = stream
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_ascii_lowercase();
        if !request_text.contains("authorization: bearer raw-api-key")
            || !request_text.contains("\"model\":\"model-m3\"")
        {
            return Err("authenticated request headers or body were not observed".into());
        }
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "Connection: close\r\n\r\n",
            "{\"model\":\"model-m3\",\"choices\":[{\"message\":{\"content\":\"http ok\"}}],\"usage\":{\"total_tokens\":2}}"
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(true)
    });
    let http_profile = ProviderProfile {
        base_url: endpoint,
        ..provider.clone()
    };
    let http_runtime = ProviderRuntime::in_memory()?;
    let http_execution = http_runtime.execute(
        &http_profile,
        ProviderRequestInput::new("Build an Android notes app"),
        "request-m3-http",
        "correlation-m3-http",
        ProjectId("m3-project".into()),
        CancellationSignal::default(),
        &FixtureResolver,
        &HttpProviderTransport::new().map_err(|error| format!("{error:?}"))?,
    )?;
    let authenticated_http_request_observed = server
        .join()
        .map_err(|_| "HTTP fixture thread panicked")?
        .map_err(|error| error.to_string())?;
    assert!(authenticated_http_request_observed && http_execution.response.text == "http ok");

    let timeout_listener = TcpListener::bind("127.0.0.1:0")?;
    let timeout_endpoint = format!(
        "http://{}/v1/chat/completions",
        timeout_listener.local_addr()?
    );
    let timeout_server = thread::spawn(move || -> Result<(), String> {
        let (_stream, _) = timeout_listener
            .accept()
            .map_err(|error| error.to_string())?;
        thread::sleep(Duration::from_millis(100));
        Ok(())
    });
    let http_timeout_profile = ProviderProfile {
        base_url: timeout_endpoint,
        timeout_ms: 20,
        ..provider.clone()
    };
    let http_timeout_runtime = ProviderRuntime::in_memory()?;
    let http_timeout_result = http_timeout_runtime.execute(
        &http_timeout_profile,
        ProviderRequestInput::new("timeout"),
        "request-m3-http-timeout",
        "correlation-m3-http-timeout",
        ProjectId("m3-project".into()),
        CancellationSignal::default(),
        &FixtureResolver,
        &HttpProviderTransport::new().map_err(|error| format!("{error:?}"))?,
    );
    let http_timeout_observed = matches!(
        &http_timeout_result,
        Err(ProviderRuntimeError::Provider(error)) if error.kind() == ProviderErrorKind::Timeout
    );
    assert!(http_timeout_observed);
    timeout_server
        .join()
        .map_err(|_| "HTTP timeout fixture thread panicked")??;

    {
        let runtime = ProviderRuntime::open(&database)?;
        let success = runtime.execute(
            &provider,
            ProviderRequestInput::new("Build an Android notes app"),
            "request-m3-success",
            "correlation-m3-success",
            ProjectId("m3-project".into()),
            CancellationSignal::default(),
            &FixtureResolver,
            &success_transport(),
        )?;
        assert_success(&success);
        let usage = runtime
            .usage_record("request-m3-success")?
            .expect("durable success usage");
        assert_eq!(usage.project_id, ProjectId("m3-project".into()));
        assert_eq!(usage.provider_id, "provider-m3");
        assert_eq!(usage.model_id, "model-m3");
        assert_eq!(usage.correlation_id, "correlation-m3-success");
        assert_eq!(usage.total_tokens, Some(18));
        assert!(!serde_json::to_string(&usage)?.contains("raw-api-key"));
    }

    let reopened = ProviderRuntime::open(&database)?;
    let restored_usage = reopened
        .usage_record("request-m3-success")?
        .expect("usage survives restart");
    assert_eq!(restored_usage.outcome, "completed");
    assert_eq!(restored_usage.total_tokens, Some(18));
    let normalized_response_observed = restored_usage.total_tokens == Some(18)
        && restored_usage.outcome == "completed"
        && restored_usage.provider_id == "provider-m3";
    assert!(normalized_response_observed);

    let timeout = reopened.execute(
        &provider,
        ProviderRequestInput::new("timeout"),
        "request-m3-timeout",
        "correlation-m3-timeout",
        ProjectId("m3-project".into()),
        CancellationSignal::default(),
        &FixtureResolver,
        &FixtureTransport {
            response: Err(ProviderError::classified(ProviderErrorKind::Timeout, None)),
        },
    );
    assert!(
        matches!(&timeout, Err(ProviderRuntimeError::Provider(error)) if error.kind() == ProviderErrorKind::Timeout)
    );
    assert_eq!(
        reopened
            .usage_record("request-m3-timeout")?
            .expect("durable timeout usage")
            .outcome,
        "timed_out"
    );

    let cancellation = CancellationSignal::default();
    cancellation.cancel();
    let cancelled = reopened.execute(
        &provider,
        ProviderRequestInput::new("cancel"),
        "request-m3-cancelled",
        "correlation-m3-cancelled",
        ProjectId("m3-project".into()),
        cancellation,
        &FixtureResolver,
        &success_transport(),
    );
    assert!(
        matches!(&cancelled, Err(ProviderRuntimeError::Provider(error)) if error.kind() == ProviderErrorKind::Cancellation)
    );
    assert_eq!(
        reopened
            .usage_record("request-m3-cancelled")?
            .expect("durable cancellation usage")
            .outcome,
        "cancelled"
    );

    let in_flight_cancellation = CancellationSignal::default();
    let cancellation_trigger = in_flight_cancellation.clone();
    let cancellation_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        cancellation_trigger.cancel();
    });
    let in_flight_result = reopened.execute(
        &provider,
        ProviderRequestInput::new("cancel during generation"),
        "request-m3-cancel-in-flight",
        "correlation-m3-cancel-in-flight",
        ProjectId("m3-project".into()),
        in_flight_cancellation,
        &FixtureResolver,
        &CancellableTransport,
    );
    cancellation_thread
        .join()
        .map_err(|_| "cancellation fixture thread panicked")?;
    let cancellation_during_request_observed = matches!(
        &in_flight_result,
        Err(ProviderRuntimeError::Provider(error)) if error.kind() == ProviderErrorKind::Cancellation
    ) && reopened
        .usage_record("request-m3-cancel-in-flight")?
        .as_ref()
        .map(|record| record.outcome == "cancelled")
        .unwrap_or(false);
    assert!(cancellation_during_request_observed);

    let safe_error = ProviderError::classified(ProviderErrorKind::Authentication, Some(401));
    assert_eq!(safe_error.kind(), ProviderErrorKind::Authentication);
    assert!(!safe_error.safe_message().contains("raw-api-key"));
    assert!(!format!("{safe_error:?}").contains("raw-api-key"));

    let evidence = json!({
        "schema": "nirman.m3.provider_runtime.v1",
        "profileValidated": profile_validated,
        "credentialReferenceOnly": credential_reference_only,
        "authenticatedRequestConstructed": authenticated_request_constructed,
        "authenticatedHttpRequestObserved": authenticated_http_request_observed,
        "requestCorrelationIdentity": request.request_id() == "request-m3-construction" && request.correlation_id() == "correlation-m3-construction",
        "normalizedResponseObserved": normalized_response_observed,
        "timeoutObserved": timeout.is_err() && reopened.usage_record("request-m3-timeout")?.as_ref().map(|record| record.outcome == "timed_out").unwrap_or(false),
        "httpTimeoutObserved": http_timeout_observed,
        "cancellationObserved": cancellation_during_request_observed && cancelled.is_err() && reopened.usage_record("request-m3-cancelled")?.as_ref().map(|record| record.outcome == "cancelled").unwrap_or(false),
        "normalizedFailureObserved": safe_error.kind() == ProviderErrorKind::Authentication && !safe_error.safe_message().contains("raw-api-key"),
        "durableUsageRecorded": restored_usage.request_id == "request-m3-success" && restored_usage.total_tokens == Some(18),
        "usageRestoredAfterRestart": restored_usage.outcome == "completed",
        "secretRedactionObserved": secret_redaction_observed,
        "providerTransport": "fixture_transport",
        "runtimeStatus": "M3_FOUNDATION_FIXTURE_ONLY"
    });
    let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m3_provider_runtime.json");
    fs::create_dir_all(evidence_path.parent().expect("evidence directory"))?;
    fs::write(evidence_path, serde_json::to_vec_pretty(&evidence)?)?;

    let _ = fs::remove_file(database);
    Ok(())
}
