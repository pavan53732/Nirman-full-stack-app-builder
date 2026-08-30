use nirman_domain::{CommandKind, ProjectId, Revision};
use nirman_ipc::{
    command_registry, normalize_android_service_error, AndroidServiceAdapter,
    AndroidServiceErrorKind, AndroidServiceIntegration, AuthContext, AuthenticatedSession,
    BackpressurePolicy, CommandRequest, ControlPlaneErrorCode, ErrorCategory, ErrorEnvelope,
    EventSubscription, SubscriptionStatus, PROTOCOL_SCHEMA_VERSION,
};

fn integration() -> AndroidServiceIntegration {
    AndroidServiceIntegration {
        request_schema_ref: "android.request.v1".into(),
        response_schema_ref: "android.response.v1".into(),
        error_schema_ref: "android.error.v1".into(),
        auth_state: "configured".into(),
        credential_reference: "credential-ref".into(),
        base_endpoint_identity: "service.example".into(),
        datastore_owner: "generated-android-service".into(),
        offline_policy: "declared-offline".into(),
        retry_policy: "bounded-retry".into(),
        timeout_policy: "bounded-timeout".into(),
        idempotency_policy: "request-key".into(),
        token_refresh_policy: "refresh-on-expiry".into(),
        privacy_policy: "declared".into(),
        network_policy: "declared".into(),
        functional_scenario_ids: vec!["scenario-1".into()],
    }
}

#[test]
fn m115_initial_command_registry_is_complete_and_executable() {
    let names: Vec<_> = command_registry()
        .into_iter()
        .map(|entry| entry.canonical_kind)
        .collect();
    let expected = [
        "project.open",
        "task.start",
        "task.cancel",
        "task.resume",
        "workspace.apply_patch",
        "preview.start",
        "preview.stop",
        "preview.promote",
        "validation.run",
        "artifact.build",
        "artifact.export",
        "provider.test",
        "settings.update_provider",
    ];
    assert_eq!(names[..expected.len()], expected);
    assert!(command_registry().iter().all(|entry| entry.supported));
}

#[test]
fn m115_authenticated_subscription_and_envelopes_round_trip() {
    let auth = AuthContext {
        installation_id: "install-1".into(),
        user_scope: "user-1".into(),
        project_scope: "project-1".into(),
        schema_version: PROTOCOL_SCHEMA_VERSION,
    };
    let session = AuthenticatedSession::issue(auth.clone(), "corr-1", 60);
    let request = CommandRequest {
        protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
        auth: auth.clone(),
        command: nirman_domain::CommandEnvelope {
            command_id: "cmd-1".into(),
            project_id: ProjectId("project-1".into()),
            task_id: None,
            kind: CommandKind::TaskStart,
            payload: "Build".into(),
            expected_projection_revision: Revision(0),
            idempotency_key: Some("idem-1".into()),
        },
        correlation_id: "corr-1".into(),
        causation_id: Some("cause-1".into()),
        deadline_epoch_seconds: None,
    };
    session.authorize(&request).expect("authenticated command");
    let subscription = EventSubscription {
        subscription_id: "sub-1".into(),
        connection_id: "connection-1".into(),
        auth: auth.clone(),
        project_id: "project-1".into(),
        task_id: None,
        from_event_sequence: 0,
        snapshot_revision: Some(Revision(0)),
        snapshot_projection_revision: None,
        last_projection_revision: None,
        requested_projection_kinds: vec!["task".into()],
        acknowledged_event_sequence: 0,
        heartbeat_interval_seconds: 15,
        max_batch_size: 64,
        backpressure_policy: BackpressurePolicy::RejectOverLimit,
        status: SubscriptionStatus::Requested,
        correlation_id: "corr-1".into(),
    };
    assert_eq!(
        serde_json::from_str::<EventSubscription>(&serde_json::to_string(&subscription).unwrap())
            .unwrap(),
        subscription
    );
    let error = ErrorEnvelope {
        error_id: "err-1".into(),
        command_id: Some("cmd-1".into()),
        correlation_id: "corr-1".into(),
        causation_id: Some("cause-1".into()),
        code: ControlPlaneErrorCode::Timeout,
        category: ErrorCategory::Timeout,
        safe_message: "safe error".into(),
        retryable: false,
        retry_after_seconds: None,
        recovery_action: None,
        diagnostic_ref: None,
        authority_decision_ref: "local".into(),
        sensitive_data_omitted: true,
        created_at_epoch_seconds: 1,
    };
    assert_eq!(
        serde_json::from_str::<ErrorEnvelope>(&serde_json::to_string(&error).unwrap()).unwrap(),
        error
    );
}

#[test]
fn m115_android_adapter_normalizes_failures_without_nirman_ipc_coupling() {
    let adapter = AndroidServiceAdapter::new(integration()).expect("valid integration");
    let error = adapter.normalize_error(
        AndroidServiceErrorKind::Timeout,
        "service timeout",
        true,
        Some("idem-1".into()),
    );
    assert_eq!(error.kind, AndroidServiceErrorKind::Timeout);
    assert!(error.retryable);
    assert_eq!(error.idempotency_key.as_deref(), Some("idem-1"));
    let direct =
        normalize_android_service_error(AndroidServiceErrorKind::Offline, "offline", true, None);
    assert_eq!(direct.kind, AndroidServiceErrorKind::Offline);
}
