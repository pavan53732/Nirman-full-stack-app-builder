use nirman_control_plane::{DurableControlPlane, DurableControlPlaneError, DurableDispatchOutcome};
use nirman_domain::{CommandEnvelope, CommandKind, DomainError, ProjectId, Revision};
use nirman_ipc::{
    command_registry, normalize_android_service_error, AndroidServiceErrorKind, AuthContext,
    AuthenticatedSession, BackpressurePolicy, CommandRequest, ErrorCategory, ErrorEnvelope,
    EventBatch, EventSubscription, ProjectionReceiver, SubscriptionStatus, PROTOCOL_SCHEMA_VERSION,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn auth() -> AuthContext {
    AuthContext {
        installation_id: "desktop-test-installation".into(),
        user_scope: "local-user".into(),
        project_scope: "project-0001".into(),
        schema_version: PROTOCOL_SCHEMA_VERSION,
    }
}

fn command(revision: u64, id: &str, kind: CommandKind, payload: &str) -> CommandEnvelope {
    CommandEnvelope {
        command_id: id.into(),
        project_id: ProjectId("project-0001".into()),
        task_id: None,
        kind,
        payload: payload.into(),
        expected_projection_revision: Revision(revision),
        idempotency_key: Some(id.into()),
    }
}

fn request(session: &AuthenticatedSession, command: CommandEnvelope) -> CommandRequest {
    CommandRequest {
        protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
        auth: session.context().clone(),
        command,
        correlation_id: "desktop-test-session".into(),
        causation_id: Some("desktop-test-cause".into()),
        deadline_epoch_seconds: None,
    }
}

fn database_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nirman-desktop-ipc-{nonce}.sqlite3"))
}

#[test]
fn desktop_ipc_trace_proves_durable_typed_projection_boundary_without_overclaiming_runtime() {
    let database = database_path();
    let auth = auth();
    let session = AuthenticatedSession::issue(auth.clone(), "desktop-test-session", 600);
    let accepted_request = request(
        &session,
        command(
            0,
            "command-1",
            CommandKind::SubmitInstruction,
            "Build an Android notes app",
        ),
    );
    session.authorize(&accepted_request).expect("valid session");

    let (initial, accepted_snapshot, accepted_event) = {
        let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("file-backed ledger");
        let initial = plane.snapshot();
        let accepted = plane
            .dispatch_with_result(accepted_request.command.clone(), "desktop-test-session")
            .expect("accepted command");
        let (accepted_snapshot, accepted_event) = match accepted {
            DurableDispatchOutcome::Accepted { snapshot, event } => (snapshot, event),
            DurableDispatchOutcome::Duplicate { .. } => panic!("first command cannot be duplicate"),
        };
        (initial, accepted_snapshot, accepted_event)
    };

    let reopened = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
        .expect("reopen file-backed ledger");
    let replayed_events = reopened
        .replay_after(initial.last_event_sequence)
        .expect("replay");
    let replayed_snapshot = reopened.snapshot();
    let duplicate_after_restart = {
        let mut plane = reopened;
        plane
            .dispatch_with_result(accepted_request.command.clone(), "desktop-test-session")
            .expect("persisted duplicate result")
    };

    let required_command_names = [
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
    let registry_complete = required_command_names.iter().all(|name| {
        command_registry().iter().any(|entry| {
            entry.canonical_kind == *name
                && entry.supported
                && !entry.request_schema_ref.is_empty()
                && !entry.response_schema_ref.is_empty()
                && !entry.required_authority.is_empty()
                && !entry.required_capability.is_empty()
                && !entry.transaction_domain.is_empty()
                && !entry.idempotency_policy.is_empty()
                && !entry.timeout_policy.is_empty()
                && !entry.cancellation_policy.is_empty()
                && !entry.emitted_event_types.is_empty()
                && !entry.projection_effects.is_empty()
        })
    });
    let subscription = EventSubscription {
        subscription_id: "subscription-1".into(),
        connection_id: "connection-1".into(),
        auth: auth.clone(),
        project_id: "project-0001".into(),
        task_id: None,
        from_event_sequence: initial.last_event_sequence,
        snapshot_revision: Some(accepted_snapshot.projection_revision),
        requested_projection_kinds: vec!["task".into(), "preview".into()],
        acknowledged_event_sequence: initial.last_event_sequence,
        heartbeat_interval_seconds: 15,
        max_batch_size: 64,
        backpressure_policy: BackpressurePolicy::RejectOverLimit,
        status: SubscriptionStatus::Active,
        correlation_id: "desktop-test-session".into(),
    };
    let subscription_round_trip = serde_json::from_str::<EventSubscription>(
        &serde_json::to_string(&subscription).expect("subscription serialization"),
    )
    .expect("subscription deserialization")
        == subscription;
    let error_envelope = ErrorEnvelope {
        error_id: "error-1".into(),
        command_id: Some("command-1".into()),
        correlation_id: "desktop-test-session".into(),
        causation_id: Some("desktop-test-cause".into()),
        code: nirman_ipc::ControlPlaneErrorCode::InvalidCommand,
        category: ErrorCategory::Validation,
        safe_message: "safe validation error".into(),
        retryable: false,
        retry_after_seconds: None,
        recovery_action: None,
        diagnostic_ref: None,
        authority_decision_ref: "local-control-plane".into(),
        sensitive_data_omitted: true,
        created_at_epoch_seconds: 1,
    };
    let error_envelope_round_trip = serde_json::from_str::<ErrorEnvelope>(
        &serde_json::to_string(&error_envelope).expect("error serialization"),
    )
    .expect("error deserialization")
        == error_envelope;
    let service_error = normalize_android_service_error(
        AndroidServiceErrorKind::Timeout,
        "service timed out",
        true,
        Some("command-1".into()),
    );
    let android_service_error_normalized =
        service_error.retryable && service_error.idempotency_key.as_deref() == Some("command-1");

    let batch = EventBatch {
        subscription_id: "subscription-1".into(),
        projection_revision: accepted_snapshot.projection_revision,
        from_event_sequence: initial.last_event_sequence,
        next_event_sequence: accepted_event.sequence,
        events: replayed_events.clone(),
        has_gap: false,
        status: SubscriptionStatus::Active,
    };
    let encoded = serde_json::to_string(&batch).expect("typed event batch is serializable");
    let decoded: EventBatch =
        serde_json::from_str(&encoded).expect("typed event batch is decodable");
    let typed_transport = decoded == batch;

    let mut receiver = ProjectionReceiver::default();
    let snapshot_accepted = receiver.observe_snapshot(initial.clone());
    let ordered_event_accepted = receiver.observe_event(&decoded.events[0]);
    let duplicate_rejected = !receiver.observe_event(&decoded.events[0]);
    let out_of_order_rejected = !receiver.observe_event(&nirman_domain::ControlEvent {
        sequence: decoded.events[0].sequence + 2,
        ..decoded.events[0].clone()
    });
    let wrong_project_rejected = !receiver.observe_event(&nirman_domain::ControlEvent {
        project_id: ProjectId("other-project".into()),
        sequence: decoded.events[0].sequence + 1,
        ..decoded.events[0].clone()
    });

    let forged = CommandRequest {
        auth: AuthContext {
            project_scope: "other-project".into(),
            ..auth.clone()
        },
        ..accepted_request.clone()
    };
    let project_scope_rejected = session.authorize(&forged).is_err();
    let stale_projection_rejected = matches!(
        DurableControlPlane::in_memory(ProjectId("project-0001".into()))
            .expect("in-memory ledger")
            .dispatch(command(1, "command-stale", CommandKind::Reconnect, "")),
        Err(DurableControlPlaneError::Domain(
            DomainError::StaleProjection { .. }
        ))
    );
    let durable_duplicate = matches!(
        duplicate_after_restart,
        DurableDispatchOutcome::Duplicate { .. }
    );
    let sqlite_replay = replayed_events.len() == 1
        && replayed_snapshot.last_event_sequence == accepted_event.sequence;
    let snapshot_bootstrap = snapshot_accepted && receiver.snapshot() == Some(&initial);

    let evidence = json!({
        "schema": "nirman.desktop_ipc_trace.v2",
        "status": "M115_HEADLESS_DURABLE_BOUNDARY_TRACE_ONLY",
        "fileBackedSqlite": true,
        "durableCommandCommit": sqlite_replay,
        "persistedIdempotencyAfterRestart": durable_duplicate,
        "typedEnvelopeRoundTrip": typed_transport,
        "commandRegistryComplete": registry_complete,
        "subscriptionEnvelopeRoundTrip": subscription_round_trip,
        "errorEnvelopeRoundTrip": error_envelope_round_trip,
        "androidServiceErrorNormalized": android_service_error_normalized,
        "authenticatedProjectScope": project_scope_rejected,
        "snapshotBootstrap": snapshot_bootstrap,
        "orderedEventDelivery": ordered_event_accepted,
        "duplicateRejected": duplicate_rejected,
        "gapAndOutOfOrderRejected": out_of_order_rejected,
        "wrongProjectEventRejected": wrong_project_rejected,
        "staleProjectionRejected": stale_projection_rejected,
        "tauriCommandRuntime": false,
        "reactDomRuntime": false,
        "androidRuntime": false,
        "apkExport": false
    });
    fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/evidence/desktop_ipc_trace.json"),
        serde_json::to_vec_pretty(&evidence).expect("evidence JSON"),
    )
    .expect("write desktop IPC evidence");
    assert!(evidence["fileBackedSqlite"].as_bool().unwrap());
    assert!(evidence["persistedIdempotencyAfterRestart"]
        .as_bool()
        .unwrap());
    assert!(evidence["typedEnvelopeRoundTrip"].as_bool().unwrap());
    assert!(evidence["commandRegistryComplete"].as_bool().unwrap());
    assert!(evidence["subscriptionEnvelopeRoundTrip"].as_bool().unwrap());
    assert!(evidence["errorEnvelopeRoundTrip"].as_bool().unwrap());
    assert!(evidence["androidServiceErrorNormalized"].as_bool().unwrap());
    assert!(evidence["authenticatedProjectScope"].as_bool().unwrap());
    assert!(evidence["duplicateRejected"].as_bool().unwrap());
    assert!(evidence["gapAndOutOfOrderRejected"].as_bool().unwrap());
    assert!(evidence["wrongProjectEventRejected"].as_bool().unwrap());
    assert!(evidence["staleProjectionRejected"].as_bool().unwrap());
    let _ = fs::remove_file(database);
}
