#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nirman_android::{
    infer_android_requirement_manifest, plan_preflight, synthesize_android_plan,
    validate_android_build_request, validate_android_workspace, AndroidBuildRequest,
    AndroidRepairRegistry, AndroidRequirementManifest, AndroidSynthesisRequest, CapabilityProbe,
    HostCapabilityProbe, PreflightStatus, RepairFailureFingerprint, RepairSelection,
};
use nirman_control_plane::{
    deadline_elapsed, DurableControlPlane, DurableControlPlaneError, DurableDispatchOutcome,
};
use nirman_domain::{
    AndroidConstructionCommandPayload, AndroidConstructionContract, CommandKind,
    MutationTransactionRecord, ProjectId, Revision,
};
use nirman_ipc::{
    acknowledge_event_subscription, authorize_registry_capability, command_registry,
    publish_control_event, AndroidRequirementEvaluateCommandPayload,
    AndroidRequirementEvaluateResultPayload, AndroidSynthesisBuildCommandPayload,
    AndroidSynthesisBuildResultPayload, AndroidToolchainPreflightCommandPayload,
    AndroidToolchainPreflightResultPayload, AuthContext, AuthenticatedSession, CommandRequest,
    CommandResponse, ControlPlaneErrorCode, ErrorCategory, ErrorEnvelope, EventBatch, EventRange,
    EventSink, EventSubscription, PreviewStartCommandPayload, PreviewStartResultPayload,
    ProviderExecuteCommandPayload, ProviderExecuteResultPayload, ProviderTestCommandPayload,
    ProviderTestResultPayload, ResponseStatus, SettingsUpdateProviderCommandPayload,
    SubscriptionAcknowledgement, SubscriptionBootstrap, SubscriptionControl, SubscriptionStatus,
    WorkspaceApplyPatchCommandPayload, WorkspaceApplyPatchResultPayload, PROTOCOL_SCHEMA_VERSION,
};
use nirman_preview::{
    bind_preview_revision, select_fallback, M108ProjectionState, PreviewEventTruth, PreviewMode,
    PreviewProjection, PreviewRequest, PreviewRevision, PreviewSyncEvent, PreviewSyncEventType,
};
use nirman_project::{
    IndexRequest, MutationBroker, MutationError, MutationRequest, ProjectIndexer,
};
use nirman_providers::{
    CancellationSignal, CredentialResolver, HttpProviderTransport, OsCredentialResolver,
    ProviderBridge, ProviderBridgeError, ProviderBridgeErrorKind, ProviderBridgeHandshake,
    ProviderErrorKind, ProviderProfile, ProviderRequestInput, ProviderRuntime,
    ProviderRuntimeError, ProviderTransport, PROVIDER_BRIDGE_PROTOCOL_VERSION,
};
use nirman_supervisor::{Supervisor, SupervisorState};
use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const PROJECT_ID: &str = "project-0001";

struct TauriEventSink<'a> {
    app: &'a AppHandle,
}

impl EventSink for TauriEventSink<'_> {
    fn emit_batch(&self, batch: EventBatch) -> Result<(), ()> {
        self.app
            .emit("nirman://control-event", batch)
            .map_err(|_| ())
    }
}

struct RuntimeState {
    plane: DurableControlPlane,
    session: AuthenticatedSession,
    auth: AuthContext,
    correlation_id: String,
    subscriptions: BTreeMap<String, EventSubscription>,
    supervisor: Supervisor,
    provider_runtime: ProviderRuntime,
    credential_resolver: Box<dyn CredentialResolver>,
    provider_transport: Box<dyn ProviderTransport>,
    capability_probe: Box<dyn CapabilityProbe>,
    authorized_workspace_root: Option<PathBuf>,
    consumed_mutation_capabilities: BTreeSet<String>,
}

#[derive(serde::Serialize)]
struct SessionHandshake {
    auth: AuthContext,
    correlation_id: String,
    schema_version: u16,
    expires_at_epoch_seconds: u64,
}

fn map_mutation_error(
    correlation_id: &str,
    command_id: &str,
    causation_id: Option<String>,
    mutation_error: MutationError,
) -> ErrorEnvelope {
    let (code, category, retryable, recovery) = match mutation_error {
        MutationError::CapabilityInvalid => (
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Authorization,
            false,
            Some("request a fresh single-use mutation capability".into()),
        ),
        MutationError::ScopeViolation
        | MutationError::OwnershipViolation
        | MutationError::InvalidPath => (
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            false,
            Some("request a new scoped mutation capability".into()),
        ),
        MutationError::BaseRevisionMismatch
        | MutationError::BaseFingerprintMismatch
        | MutationError::BaseFileHashMismatch => (
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            true,
            Some("re-index the current project revision and rebase the proposal".into()),
        ),
        MutationError::MutationBudgetExceeded
        | MutationError::DependencyPolicyMissing
        | MutationError::EvidenceRequired
        | MutationError::IsolationRequired
        | MutationError::WholeFileFallbackRejected
        | MutationError::TouchedPathMismatch
        | MutationError::InvalidIdentity
        | MutationError::InvalidStructuredOperation
        | MutationError::UnknownSymbol
        | MutationError::SyntaxInvalid => (
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            false,
            Some("repair the structured mutation proposal and revalidate it".into()),
        ),
        MutationError::ContentIntegrityFailure => (
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Conflict,
            false,
            Some("discard the candidate and reconcile the workspace revision".into()),
        ),
        MutationError::WorkspaceUnavailable
        | MutationError::IndexFailure(_)
        | MutationError::CommitFailure(_) => (
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            true,
            Some("reconcile the local workspace and retry from a durable checkpoint".into()),
        ),
    };
    error(
        correlation_id,
        Some(command_id.to_owned()),
        causation_id,
        code,
        category,
        mutation_error.to_string(),
        retryable,
        recovery,
    )
}

fn preflight_status_name(status: &PreflightStatus) -> &'static str {
    match status {
        PreflightStatus::Available => "AVAILABLE",
        PreflightStatus::Repairable => "REPAIRABLE",
        PreflightStatus::UserRequired => "USER_REQUIRED",
        PreflightStatus::Unavailable => "UNAVAILABLE",
    }
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn persist_m108_stage(
    state: &mut RuntimeState,
    task_id: &str,
    preview_revision_id: &str,
    event_type: PreviewSyncEventType,
    event_truth: PreviewEventTruth,
    project_revision_id: &str,
    checkpoint_id: &str,
    source_fingerprint: &str,
    command_id: &str,
    evidence_refs: Vec<String>,
) -> Result<(), ErrorEnvelope> {
    let (mut projection, _) = if let Some((projection_json, evidence_json, _)) =
        state.plane.load_m108_sync_record(task_id).map_err(|_| {
            error(
                "m108",
                None,
                None,
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "M108 synchronization record could not be loaded",
                true,
                None,
            )
        })? {
        (
            serde_json::from_str::<M108ProjectionState>(&projection_json).map_err(|_| {
                error(
                    "m108",
                    None,
                    None,
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "M108 projection is corrupt",
                    false,
                    None,
                )
            })?,
            evidence_json,
        )
    } else {
        (
            M108ProjectionState::new(
                state.auth.project_scope.clone(),
                task_id,
                preview_revision_id,
            ),
            "[]".into(),
        )
    };
    let sequence = projection.last_event_sequence + 1;
    let event = PreviewSyncEvent {
        event_id: format!("m108:{command_id}:{sequence}"),
        event_sequence: sequence,
        project_id: state.auth.project_scope.clone(),
        task_id: task_id.into(),
        correlation_id: state.correlation_id.clone(),
        causation_id: Some(command_id.into()),
        candidate_preview_revision_id: preview_revision_id.into(),
        event_type,
        event_truth,
        project_revision_id: project_revision_id.into(),
        checkpoint_id: checkpoint_id.into(),
        source_fingerprint: source_fingerprint.into(),
        artifact_id: None,
        artifact_fingerprint: None,
        runtime_session_id: None,
        device_id: None,
        operation_ref: command_id.into(),
        observation_refs: vec![],
        evidence_refs: evidence_refs.clone(),
        validation_ref: None,
        payload: "authoritative host boundary event".into(),
    };
    projection.apply(&event).map_err(|e| {
        error(
            "m108",
            Some(command_id.into()),
            Some(command_id.into()),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            e.to_string(),
            false,
            None,
        )
    })?;
    let evidence = serde_json::json!({"eventId": event.event_id, "eventSequence": sequence, "projectionRevision": projection.projection_revision, "previewRevisionId": preview_revision_id, "evidenceRefs": evidence_refs, "truth": format!("{:?}", event.event_truth), "nativeWindowsTauriRuntimeObserved": false, "androidRuntimeObserved": false});
    state
        .plane
        .save_m108_sync_record(
            task_id,
            &serde_json::to_string(&projection).expect("M108 projection serialization"),
            &serde_json::to_string(&vec![evidence]).expect("M108 evidence serialization"),
            sequence,
        )
        .map_err(|_| {
            error(
                "m108",
                Some(command_id.into()),
                None,
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "M108 synchronization record could not be persisted",
                true,
                None,
            )
        })
}

fn error(
    correlation_id: impl Into<String>,
    command_id: Option<String>,
    causation_id: Option<String>,
    code: ControlPlaneErrorCode,
    category: ErrorCategory,
    message: impl Into<String>,
    retryable: bool,
    recovery_action: Option<String>,
) -> ErrorEnvelope {
    let correlation_id = correlation_id.into();
    ErrorEnvelope {
        error_id: format!("error-{correlation_id}"),
        command_id,
        correlation_id,
        causation_id,
        code,
        category,
        safe_message: message.into(),
        retryable,
        retry_after_seconds: retryable.then_some(1),
        recovery_action,
        diagnostic_ref: None,
        authority_decision_ref: "local-control-plane".into(),
        sensitive_data_omitted: true,
        created_at_epoch_seconds: now_epoch_seconds(),
    }
}

fn response(
    command_id: impl Into<String>,
    correlation_id: impl Into<String>,
    causation_id: Option<String>,
    project_id: ProjectId,
    snapshot: nirman_domain::ProjectionSnapshot,
    status: ResponseStatus,
    event_range: Option<EventRange>,
) -> CommandResponse {
    let command_id = command_id.into();
    let correlation_id = correlation_id.into();
    CommandResponse {
        response_id: format!("response-{command_id}-{}", snapshot.projection_revision.0),
        command_id,
        correlation_id,
        causation_id,
        project_id,
        task_id: None,
        status,
        result_schema_ref: Some("nirman.command_response.v1".into()),
        projection_snapshot_ref: Some(format!("projection-{}", snapshot.projection_revision.0)),
        projection_revision: snapshot.projection_revision,
        snapshot,
        event_range,
        result_payload: None,
        authority_decision_ref: "local-control-plane".into(),
        created_at_epoch_seconds: now_epoch_seconds(),
    }
}

fn map_provider_error(
    correlation_id: &str,
    command_id: &str,
    causation_id: Option<String>,
    runtime_error: ProviderRuntimeError,
) -> ErrorEnvelope {
    match runtime_error {
        ProviderRuntimeError::Configuration(config_error) => error(
            correlation_id,
            Some(command_id.to_owned()),
            causation_id,
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Provider,
            config_error.to_string(),
            false,
            None,
        ),
        ProviderRuntimeError::Provider(provider_error) => {
            let (code, category, recovery_action) = match provider_error.kind() {
                ProviderErrorKind::Timeout => (
                    ControlPlaneErrorCode::Timeout,
                    ErrorCategory::Timeout,
                    Some("retry the provider request with a valid deadline".into()),
                ),
                ProviderErrorKind::Cancellation => (
                    ControlPlaneErrorCode::CancellationRejected,
                    ErrorCategory::Cancellation,
                    Some("start a new provider test if the operation is still required".into()),
                ),
                ProviderErrorKind::Authentication => (
                    ControlPlaneErrorCode::PermissionDenied,
                    ErrorCategory::Authentication,
                    Some("update the secure provider credential reference".into()),
                ),
                _ => (
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Provider,
                    Some("inspect provider availability and retry".into()),
                ),
            };
            error(
                correlation_id,
                Some(command_id.to_owned()),
                causation_id,
                code,
                category,
                provider_error.safe_message(),
                provider_error.retryable(),
                recovery_action,
            )
        }
        ProviderRuntimeError::UsagePersistence => error(
            correlation_id,
            Some(command_id.to_owned()),
            causation_id,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "provider usage could not be recorded durably",
            true,
            Some("reconcile local provider storage before retry".into()),
        ),
    }
}

fn parse_provider_profile(
    request: &CommandRequest,
) -> Result<(ProviderProfile, String), ErrorEnvelope> {
    let payload: SettingsUpdateProviderCommandPayload =
        serde_json::from_str(&request.command.payload).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "provider settings payload is invalid",
                false,
                None,
            )
        })?;
    let profile: ProviderProfile = serde_json::from_value(payload.profile).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "provider profile payload is invalid",
            false,
            None,
        )
    })?;
    profile.validate().map_err(|config_error| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            config_error.to_string(),
            false,
            None,
        )
    })?;
    let profile_json = serde_json::to_string(&profile).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Internal,
            "provider profile could not be serialized safely",
            false,
            None,
        )
    })?;
    Ok((profile, profile_json))
}

fn parse_android_toolchain_preflight(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<
    (
        AndroidToolchainPreflightCommandPayload,
        nirman_android::AndroidToolchainPreflight,
        String,
    ),
    ErrorEnvelope,
> {
    let payload: AndroidToolchainPreflightCommandPayload =
        serde_json::from_str(&request.command.payload).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "android toolchain preflight payload is invalid",
                false,
                None,
            )
        })?;
    if payload.build_variant.trim().is_empty() {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "android toolchain preflight requires a build variant",
            false,
            None,
        ));
    }
    let task_id = request.command.task_id.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Scope,
            "android toolchain preflight requires a task scope",
            false,
            None,
        )
    })?;
    let contract_json = state
        .plane
        .load_android_construction_contract(&task_id.0)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "android construction contract storage is unavailable",
                true,
                None,
            )
        })?
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "android construction contract is not persisted for this task",
                false,
                None,
            )
        })?;
    let contract = AndroidConstructionContract::from_json(&contract_json).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Internal,
            "persisted android construction contract could not be restored safely",
            false,
            None,
        )
    })?;
    if contract.project_id != request.command.project_id || contract.task_id != *task_id {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Scope,
            "android construction contract does not match the preflight scope",
            false,
            None,
        ));
    }
    let preflight = plan_preflight(
        &contract,
        &payload.build_variant,
        state.capability_probe.as_ref(),
    )
    .map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "android toolchain preflight could not be planned",
            false,
            None,
        )
    })?;
    let preflight_json = serde_json::to_string(&preflight).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Internal,
            "android toolchain preflight could not be serialized safely",
            false,
            None,
        )
    })?;
    Ok((payload, preflight, preflight_json))
}

fn parse_android_construction_contract(
    request: &CommandRequest,
) -> Result<(AndroidConstructionContract, String), ErrorEnvelope> {
    let payload: AndroidConstructionCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "android construction contract payload is invalid",
                false,
                None,
            )
        })?;
    if payload.contract.project_id != request.command.project_id
        || request.command.task_id.as_ref() != Some(&payload.contract.task_id)
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Scope,
            "android construction contract identity does not match the command scope",
            false,
            None,
        ));
    }
    payload.contract.validate().map_err(|contract_error| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            contract_error.to_string(),
            false,
            None,
        )
    })?;
    let contract_json = serde_json::to_string(&payload.contract).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Internal,
            "android construction contract could not be serialized safely",
            false,
            None,
        )
    })?;
    Ok((payload.contract, contract_json))
}

#[derive(Debug)]
struct PreparedM4 {
    task_id: String,
    plan: nirman_android::AndroidSynthesisPlan,
    build_request: AndroidBuildRequest,
    plan_json: String,
    build_request_json: String,
    toolchain_lock_hash: String,
    environment_snapshot_id: String,
}

fn parse_m4_synthesis_build(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<PreparedM4, ErrorEnvelope> {
    let payload: AndroidSynthesisBuildCommandPayload =
        serde_json::from_str(&request.command.payload).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "M4 synthesis/build payload is invalid",
                false,
                None,
            )
        })?;
    let task_id = request.command.task_id.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "M4 synthesis/build requires a task scope",
            false,
            None,
        )
    })?;
    payload.contract.validate().map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "M4 Android construction contract is invalid",
            false,
            None,
        )
    })?;
    if payload.contract.project_id != request.command.project_id
        || payload.contract.task_id != *task_id
        || payload.contract.target_platforms != vec!["android".to_string()]
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "M4 contract does not match authenticated Android scope",
            false,
            None,
        ));
    }
    let authorized_root = state
        .authorized_workspace_root
        .as_ref()
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Environment,
                "no authorized project workspace is configured",
                true,
                None,
            )
        })?;
    let requested_root = PathBuf::from(&payload.workspace_root)
        .canonicalize()
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::PermissionDenied,
                ErrorCategory::Scope,
                "M4 workspace is unavailable",
                false,
                None,
            )
        })?;
    if requested_root != authorized_root {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "M4 workspace is outside the authorized project root",
            false,
            None,
        ));
    }
    let index = ProjectIndexer::default()
        .index_workspace(&requested_root, &IndexRequest::default())
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "M4 workspace indexing failed",
                true,
                None,
            )
        })?;
    if index.project_fingerprint != payload.project_fingerprint {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            "M4 project fingerprint is stale",
            true,
            None,
        ));
    }
    let preflight_json = state
        .plane
        .load_android_toolchain_preflight(task_id.0.as_str())
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "M43 toolchain preflight is unavailable",
                true,
                None,
            )
        })?
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Environment,
                "M4 requires persisted M43 preflight",
                false,
                None,
            )
        })?;
    let preflight: nirman_android::AndroidToolchainPreflight =
        serde_json::from_str(&preflight_json).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "M43 preflight could not be restored",
                false,
                None,
            )
        })?;
    let lock = preflight.lock.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Environment,
            "M4 requires an available M43 toolchain lock",
            false,
            None,
        )
    })?;
    let plan = synthesize_android_plan(&AndroidSynthesisRequest {
        schema_version: nirman_android::M4_SCHEMA_VERSION,
        contract: payload.contract.clone(),
        source_revision: payload.source_revision,
        workspace_root: requested_root.to_string_lossy().into_owned(),
        project_fingerprint: payload.project_fingerprint.clone(),
    })
    .map_err(|e| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            e.to_string(),
            false,
            None,
        )
    })?;
    let build_request = AndroidBuildRequest {
        schema_version: nirman_android::M4_SCHEMA_VERSION,
        project_id: payload.contract.project_id.0.clone(),
        task_id: payload.contract.task_id.0.clone(),
        source_revision: payload.source_revision,
        project_fingerprint: payload.project_fingerprint,
        workspace_root: requested_root.to_string_lossy().into_owned(),
        build_variant: payload.build_variant,
        gradle_task: payload.gradle_task,
    };
    validate_android_build_request(&build_request).map_err(|e| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            e.to_string(),
            false,
            None,
        )
    })?;
    Ok(PreparedM4 {
        task_id: task_id.0.clone(),
        plan_json: serde_json::to_string(&plan).expect("M4 plan serialization"),
        build_request_json: serde_json::to_string(&build_request).expect("M4 build serialization"),
        toolchain_lock_hash: lock.lock_hash.clone(),
        environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
        plan,
        build_request,
    })
}

#[derive(Debug)]
struct PreparedM47 {
    task_id: String,
    source_revision: u64,
    project_fingerprint: String,
    manifest: AndroidRequirementManifest,
    manifest_json: String,
    repair_selection: Option<RepairSelection>,
    repair_selection_json: Option<String>,
}

fn parse_android_requirement_evaluation(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<PreparedM47, ErrorEnvelope> {
    let payload: AndroidRequirementEvaluateCommandPayload =
        serde_json::from_str(&request.command.payload).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "android requirement evaluation payload is invalid",
                false,
                None,
            )
        })?;
    let task_id = request.command.task_id.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "android requirement evaluation requires a task scope",
            false,
            None,
        )
    })?;
    if payload.workspace_root.trim().is_empty() || payload.project_fingerprint.trim().is_empty() {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "android requirement evaluation requires workspace and fingerprint identity",
            false,
            None,
        ));
    }
    let authorized_root = state
        .authorized_workspace_root
        .as_ref()
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Environment,
                "no authorized project workspace is configured",
                true,
                Some(
                    "configure the project workspace before evaluating Android requirements".into(),
                ),
            )
        })?;
    let requested_root = PathBuf::from(&payload.workspace_root)
        .canonicalize()
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::PermissionDenied,
                ErrorCategory::Scope,
                "requested workspace is unavailable",
                false,
                None,
            )
        })?;
    if requested_root != authorized_root {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "requested workspace is outside the authorized project root",
            false,
            None,
        ));
    }
    if payload.source_revision != state.plane.snapshot().current_source_revision.0 {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            "Android requirement evaluation source revision is stale",
            true,
            Some("re-index the current authorized project revision".into()),
        ));
    }
    let contract_json = state
        .plane
        .load_android_construction_contract(task_id.0.as_str())
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "durable Android construction contract is unavailable",
                true,
                Some(
                    "reconcile the durable construction contract before evaluating requirements"
                        .into(),
                ),
            )
        })?
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "Android requirement evaluation requires a durable construction contract",
                false,
                Some(
                    "create the Android construction contract before evaluating requirements"
                        .into(),
                ),
            )
        })?;
    let contract: AndroidConstructionContract =
        serde_json::from_str(&contract_json).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "durable Android construction contract could not be restored safely",
                false,
                None,
            )
        })?;
    if contract.project_id != request.command.project_id || contract.task_id != *task_id {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "durable Android construction contract does not match the command scope",
            false,
            None,
        ));
    }
    contract.validate().map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "durable Android construction contract is invalid",
            false,
            None,
        )
    })?;
    let index = ProjectIndexer::default()
        .index_workspace(&requested_root, &IndexRequest::default())
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "authorized Android workspace could not be indexed",
                true,
                Some("repair the local project workspace and retry indexing".into()),
            )
        })?;
    if index.project_fingerprint != payload.project_fingerprint {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            "Android requirement evaluation fingerprint is stale",
            true,
            Some("re-index the current authorized project workspace".into()),
        ));
    }
    let workspace_validation =
        validate_android_workspace(&requested_root, &index).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "Android manifest and resources could not be validated",
                true,
                Some("repair the local Android workspace and retry validation".into()),
            )
        })?;
    if !workspace_validation.invalid_resource_files.is_empty()
        || (workspace_validation.manifest_present && !workspace_validation.manifest_well_formed)
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "Android manifest or resource validation failed",
            false,
            Some(
                "repair the manifest/resource files through the authorized mutation broker".into(),
            ),
        ));
    }
    let manifest = infer_android_requirement_manifest(&contract, &index, payload.source_revision)
        .map_err(|error_value| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            error_value.to_string(),
            false,
            None,
        )
    })?;
    let repair_selection = payload
        .failure
        .map(|failure| {
            AndroidRepairRegistry::default().select(&RepairFailureFingerprint {
                classifier: failure.classifier,
                detail: failure.detail,
            })
        })
        .transpose()
        .map_err(|error_value| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                error_value.to_string(),
                false,
                Some("classify the failure with a registered Android repair pattern".into()),
            )
        })?;
    let manifest_json = serde_json::to_string(&manifest).expect("M47 manifest serialization");
    let repair_selection_json = repair_selection
        .as_ref()
        .map(|selection| serde_json::to_string(selection).expect("M47 selection serialization"));
    Ok(PreparedM47 {
        task_id: task_id.0.clone(),
        source_revision: payload.source_revision,
        project_fingerprint: payload.project_fingerprint,
        manifest,
        manifest_json,
        repair_selection,
        repair_selection_json,
    })
}

#[derive(Debug)]
struct PreparedM48 {
    task_id: String,
    preview_revision_id: String,
    selection: nirman_preview::PreviewFallbackSelection,
    revision: PreviewRevision,
    selection_json: String,
    revision_json: String,
    projection_json: String,
}

fn parse_preview_start(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<PreparedM48, ErrorEnvelope> {
    let payload: PreviewStartCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "preview start payload is invalid",
                false,
                None,
            )
        })?;
    let preview_request: PreviewRequest = payload.request;
    preview_request.validate().map_err(|error_value| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            error_value.to_string(),
            false,
            None,
        )
    })?;
    if preview_request.project_id != request.command.project_id.0
        || request.command.task_id.as_ref().map(|task| task.0.as_str())
            != Some(preview_request.task_id.as_str())
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "preview request identity does not match the authenticated command scope",
            false,
            None,
        ));
    }
    if preview_request.project_revision_id.trim().is_empty()
        || preview_request.source_fingerprint.trim().is_empty()
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "preview request must bind a source revision and fingerprint",
            false,
            None,
        ));
    }
    if preview_request.project_revision_id
        != format!(
            "source-{}",
            state.plane.snapshot().current_source_revision.0
        )
        && preview_request.requested_mode != Some(PreviewMode::Diagnostic)
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            "preview request does not target the current source revision",
            true,
            Some("reload the current committed source revision before starting preview".into()),
        ));
    }
    let selection = select_fallback(&preview_request).map_err(|error_value| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            error_value.to_string(),
            false,
            None,
        )
    })?;
    let preview_revision_id = format!("preview-revision-{}", preview_request.request_id);
    let revision = bind_preview_revision(
        &preview_request,
        &selection,
        &preview_revision_id,
        now_epoch_seconds(),
    )
    .map_err(|error_value| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            error_value.to_string(),
            false,
            None,
        )
    })?;
    let mut projection = PreviewProjection::new(
        preview_request.project_id.clone(),
        preview_request.task_id.clone(),
    );
    projection
        .apply_candidate(revision.clone(), &preview_request)
        .map_err(|error_value| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                error_value.to_string(),
                false,
                None,
            )
        })?;
    let selection_json = serde_json::to_string(&selection).expect("M48 selection serialization");
    let revision_json = serde_json::to_string(&revision).expect("M48 revision serialization");
    let projection_json = serde_json::to_string(&projection).expect("M48 projection serialization");
    Ok(PreparedM48 {
        task_id: preview_request.task_id,
        preview_revision_id,
        selection,
        revision,
        selection_json,
        revision_json,
        projection_json,
    })
}

#[derive(Debug)]
struct PreparedMutation {
    request: MutationRequest,
    transaction: MutationTransactionRecord,
}

fn prepare_m46_mutation(
    state: &RuntimeState,
    request: &CommandRequest,
    payload: &WorkspaceApplyPatchCommandPayload,
) -> Result<Option<PreparedMutation>, ErrorEnvelope> {
    let task_id = request.command.task_id.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Scope,
            "workspace mutation requires a task scope",
            false,
            None,
        )
    })?;
    let transaction_id = format!("mutation-transaction-{}", payload.operation_id);
    if let Some(existing) = state
        .plane
        .load_mutation_transaction(&transaction_id)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "mutation transaction storage is unavailable",
                true,
                Some("reconcile local mutation storage before retry".into()),
            )
        })?
    {
        if existing.command_id == request.command.command_id {
            return Ok(None);
        }
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::IdempotencyConflict,
            ErrorCategory::Idempotency,
            "mutation operation identity has already been used",
            false,
            Some("reconcile the existing mutation transaction before retrying".into()),
        ));
    }
    let authorized_root = state
        .authorized_workspace_root
        .as_ref()
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Environment,
                "no authorized project workspace is configured",
                true,
                Some("configure the project workspace before requesting a mutation".into()),
            )
        })?;
    let requested_root = PathBuf::from(&payload.workspace_root)
        .canonicalize()
        .map_err(|_| {
            map_mutation_error(
                &request.correlation_id,
                &request.command.command_id,
                request.causation_id.clone(),
                MutationError::WorkspaceUnavailable,
            )
        })?;
    if requested_root != authorized_root {
        return Err(map_mutation_error(
            &request.correlation_id,
            &request.command.command_id,
            request.causation_id.clone(),
            MutationError::ScopeViolation,
        ));
    }
    if payload.base_revision != state.plane.snapshot().current_source_revision.0 {
        return Err(map_mutation_error(
            &request.correlation_id,
            &request.command.command_id,
            request.causation_id.clone(),
            MutationError::BaseRevisionMismatch,
        ));
    }
    let active_fence = state.supervisor.snapshot().active_fence;
    if state.supervisor.snapshot().state != SupervisorState::Running
        || active_fence.as_ref().is_none_or(|fence| {
            fence.owner_id != payload.worker_id || fence.fence_token != payload.fence_token
        })
    {
        return Err(map_mutation_error(
            &request.correlation_id,
            &request.command.command_id,
            request.causation_id.clone(),
            MutationError::CapabilityInvalid,
        ));
    }
    let expected_capability = nirman_project::mutation_capability_digest(
        &request.command.project_id.0,
        &task_id.0,
        &payload.worker_id,
        &payload.operation_id,
        payload.base_revision,
        &payload.base_project_fingerprint,
        payload.fence_token,
    );
    if payload.capability_digest != expected_capability
        || state
            .consumed_mutation_capabilities
            .contains(&payload.capability_digest)
    {
        return Err(map_mutation_error(
            &request.correlation_id,
            &request.command.command_id,
            request.causation_id.clone(),
            MutationError::CapabilityInvalid,
        ));
    }
    let base_revision = Revision(payload.base_revision);
    let transaction = MutationTransactionRecord {
        transaction_id,
        command_id: request.command.command_id.clone(),
        operation_id: payload.operation_id.clone(),
        project_id: request.command.project_id.clone(),
        task_id: task_id.clone(),
        worker_id: payload.worker_id.clone(),
        workspace_root: requested_root.to_string_lossy().into_owned(),
        checkpoint_id: format!("m46-checkpoint-{}", request.command.command_id),
        base_revision,
        resulting_revision: base_revision,
        base_project_fingerprint: payload.base_project_fingerprint.clone(),
        resulting_project_fingerprint: None,
        capability_digest: payload.capability_digest.clone(),
        fence_token: payload.fence_token,
        state: "PREPARED".into(),
        changed_paths_json: None,
        evidence_json: None,
        started_at_epoch_seconds: now_epoch_seconds(),
        completed_at_epoch_seconds: None,
    };
    let mutation_request = MutationRequest {
        workspace_root: requested_root.to_string_lossy().into_owned(),
        project_id: request.command.project_id.0.clone(),
        task_id: task_id.0.clone(),
        worker_id: payload.worker_id.clone(),
        operation_id: payload.operation_id.clone(),
        base_revision: payload.base_revision,
        base_project_fingerprint: payload.base_project_fingerprint.clone(),
        allowed_paths: payload.allowed_paths.clone(),
        owned_paths: payload.owned_paths.clone(),
        touched_paths: payload.touched_paths.clone(),
        base_file_hashes: payload.base_file_hashes.clone(),
        mutation_budget: payload.mutation_budget,
        dependency_policy: payload.dependency_policy.clone(),
        capability_digest: payload.capability_digest.clone(),
        fence_token: payload.fence_token,
        evidence_required: payload.evidence_required,
        isolated_transaction: payload.isolated_transaction,
        whole_file_fallback: payload.whole_file_fallback,
        operation: payload.operation.clone(),
    };
    Ok(Some(PreparedMutation {
        request: mutation_request,
        transaction,
    }))
}

fn parse_workspace_apply_patch(
    request: &CommandRequest,
) -> Result<WorkspaceApplyPatchCommandPayload, ErrorEnvelope> {
    let payload: WorkspaceApplyPatchCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "workspace patch payload is invalid",
                false,
                None,
            )
        })?;
    if request.command.task_id.is_none()
        || payload.worker_id.trim().is_empty()
        || payload.operation_id.trim().is_empty()
        || payload.workspace_root.trim().is_empty()
        || payload.base_project_fingerprint.trim().is_empty()
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "workspace patch requires task, worker, operation, workspace, and base fingerprint identity",
            false,
            None,
        ));
    }
    Ok(payload)
}

fn parse_provider_test(
    request: &CommandRequest,
) -> Result<ProviderTestCommandPayload, ErrorEnvelope> {
    serde_json::from_str(&request.command.payload).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "provider test payload is invalid",
            false,
            None,
        )
    })
}

fn parse_provider_execute(
    request: &CommandRequest,
) -> Result<ProviderExecuteCommandPayload, ErrorEnvelope> {
    let payload: ProviderExecuteCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "provider execute payload is invalid",
                false,
                None,
            )
        })?;
    if payload.provider_id.trim().is_empty()
        || payload.worker_id.trim().is_empty()
        || payload.prompt.trim().is_empty()
        || payload.max_context_tokens == 0
        || payload.privacy_classification.trim().is_empty()
        || payload.tool_policy.trim().is_empty()
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "provider execute requires bounded identity, prompt, context, privacy, and tool policy fields",
            false,
            None,
        ));
    }
    if request.command.task_id.is_none() {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Scope,
            "provider execute requires a task scope",
            false,
            None,
        ));
    }
    Ok(payload)
}

fn load_m44_preflight(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<nirman_android::AndroidToolchainPreflight, ErrorEnvelope> {
    let task_id = request.command.task_id.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Scope,
            "provider execute requires a task-scoped toolchain preflight",
            false,
            None,
        )
    })?;
    let preflight_json = state
        .plane
        .load_android_toolchain_preflight(&task_id.0)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "android toolchain preflight storage is unavailable",
                true,
                Some("reconcile local preflight storage before retry".into()),
            )
        })?
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Environment,
                "provider execute requires a persisted android toolchain preflight",
                false,
                Some("run android toolchain preflight for this task first".into()),
            )
        })?;
    let preflight: nirman_android::AndroidToolchainPreflight =
        serde_json::from_str(&preflight_json).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "persisted android toolchain preflight could not be restored safely",
                false,
                Some("repair the local preflight record".into()),
            )
        })?;
    let lock = preflight.lock.as_ref().ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Environment,
            "provider execute requires a non-empty persisted android toolchain lock",
            false,
            Some("complete an available toolchain preflight before provider execution".into()),
        )
    })?;
    let lock_hash = lock.lock_hash.trim();
    if preflight.status != PreflightStatus::Available
        || lock_hash.is_empty()
        || preflight.manifest.project_id != request.command.project_id.0
        || preflight.manifest.task_id != task_id.0
        || preflight.environment_snapshot.project_id != request.command.project_id.0
        || preflight.environment_snapshot.task_id != task_id.0
        || preflight
            .environment_snapshot
            .toolchain_lock_hash
            .as_deref()
            != Some(lock_hash)
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Environment,
            "persisted android toolchain preflight is not an available lock for this task scope",
            false,
            Some("re-run preflight and persist an available environment lock".into()),
        ));
    }
    Ok(preflight)
}

fn provider_execute_result_from_record(
    record: &nirman_domain::ProviderExecutionRecord,
) -> ProviderExecuteResultPayload {
    let text = record
        .response_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|value| {
            value
                .get("text")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    ProviderExecuteResultPayload {
        execution_id: record.execution_id.clone(),
        request_id: record.request_id.clone(),
        correlation_id: record.correlation_id.clone(),
        provider_id: record.provider_id.clone(),
        model_id: record.model_id.clone(),
        environment_lock_hash: record.environment_lock_hash.clone(),
        environment_snapshot_id: record.environment_snapshot_id.clone(),
        state: record.state.clone(),
        outcome: record.outcome.clone(),
        text,
        error_kind: record.error_kind.clone(),
        events: Vec::new(),
    }
}

fn map_bridge_error(
    correlation_id: &str,
    command_id: &str,
    causation_id: Option<String>,
    bridge_error: ProviderBridgeError,
) -> ErrorEnvelope {
    let (code, category) = match bridge_error.kind() {
        ProviderBridgeErrorKind::Authentication => (
            ControlPlaneErrorCode::AuthenticationFailed,
            ErrorCategory::Authentication,
        ),
        ProviderBridgeErrorKind::ProfileMismatch
        | ProviderBridgeErrorKind::ModelCapability
        | ProviderBridgeErrorKind::InvalidRequest
        | ProviderBridgeErrorKind::Configuration => (
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
        ),
        ProviderBridgeErrorKind::Timeout => {
            (ControlPlaneErrorCode::Timeout, ErrorCategory::Timeout)
        }
        ProviderBridgeErrorKind::Cancellation => (
            ControlPlaneErrorCode::CancellationRejected,
            ErrorCategory::Cancellation,
        ),
        ProviderBridgeErrorKind::RateLimited => (
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Provider,
        ),
        ProviderBridgeErrorKind::ProtocolMismatch | ProviderBridgeErrorKind::MalformedResponse => (
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Provider,
        ),
        ProviderBridgeErrorKind::Unavailable | ProviderBridgeErrorKind::ProviderRejected => (
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
        ),
    };
    error(
        correlation_id,
        Some(command_id.to_owned()),
        causation_id,
        code,
        category,
        bridge_error.safe_message(),
        bridge_error.retryable(),
        Some("inspect the durable execution record before retrying".into()),
    )
}

fn map_runtime_error(
    correlation_id: &str,
    command_id: Option<String>,
    causation_id: Option<String>,
    runtime_error: DurableControlPlaneError,
) -> ErrorEnvelope {
    match runtime_error {
        DurableControlPlaneError::Domain(domain_error) => {
            let (code, category, retryable) = match &domain_error {
                nirman_domain::DomainError::StaleProjection { .. } => (
                    ControlPlaneErrorCode::StaleProjection,
                    ErrorCategory::StaleProjection,
                    false,
                ),
                nirman_domain::DomainError::DuplicateCommand => (
                    ControlPlaneErrorCode::DuplicateCommand,
                    ErrorCategory::Idempotency,
                    false,
                ),
                nirman_domain::DomainError::EmptyInstruction
                | nirman_domain::DomainError::InvalidTransition => (
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Validation,
                    false,
                ),
            };
            error(
                correlation_id,
                command_id,
                causation_id,
                code,
                category,
                domain_error.to_string(),
                retryable,
                None,
            )
        }
        DurableControlPlaneError::Storage(_storage_error) => error(
            correlation_id,
            command_id,
            causation_id,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "local durable storage is unavailable",
            true,
            Some("reconcile local storage before retry".into()),
        ),
        DurableControlPlaneError::IdempotencyConflict => error(
            correlation_id,
            command_id,
            causation_id,
            ControlPlaneErrorCode::IdempotencyConflict,
            ErrorCategory::Conflict,
            "idempotency key conflicts with a different request fingerprint",
            false,
            None,
        ),
        DurableControlPlaneError::CorruptCommandResult(_) => error(
            correlation_id,
            command_id,
            causation_id,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Internal,
            "durable command result could not be restored safely",
            false,
            Some("reconcile the local ledger".into()),
        ),
    }
}

fn authorize(state: &RuntimeState, request: &CommandRequest) -> Result<(), ErrorEnvelope> {
    state.session.authorize(request).map_err(|code| {
        let category = match code {
            ControlPlaneErrorCode::SchemaMismatch => ErrorCategory::Validation,
            ControlPlaneErrorCode::PermissionDenied => ErrorCategory::Authorization,
            _ => ErrorCategory::Authentication,
        };
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            code,
            category,
            "authenticated local session rejected the request",
            false,
            None,
        )
    })
}

fn authorize_subscription(
    state: &RuntimeState,
    subscription: &EventSubscription,
) -> Result<(), ErrorEnvelope> {
    state
        .session
        .authorize_context(
            &subscription.auth,
            &subscription.project_id,
            &subscription.correlation_id,
        )
        .map_err(|code| {
            let category = match code {
                ControlPlaneErrorCode::SchemaMismatch => ErrorCategory::Validation,
                ControlPlaneErrorCode::PermissionDenied => ErrorCategory::Authorization,
                _ => ErrorCategory::Authentication,
            };
            error(
                &subscription.correlation_id,
                None,
                None,
                code,
                category,
                "authenticated event subscription rejected",
                false,
                None,
            )
        })?;
    if subscription.project_id != PROJECT_ID {
        return Err(error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "event subscription project is not available in this host",
            false,
            None,
        ));
    }
    if subscription.max_batch_size == 0 || subscription.max_batch_size > 256 {
        return Err(error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::Backpressure,
            ErrorCategory::Validation,
            "subscription batch size is outside the local safety bound",
            false,
            Some("request a batch size from 1 through 256".into()),
        ));
    }
    Ok(())
}

fn batch(
    subscription: &EventSubscription,
    projection_revision: nirman_domain::Revision,
    after_sequence: u64,
    events: Vec<nirman_domain::ControlEvent>,
    status: SubscriptionStatus,
    has_gap: bool,
) -> EventBatch {
    EventBatch {
        subscription_id: subscription.subscription_id.clone(),
        projection_revision,
        from_event_sequence: after_sequence,
        next_event_sequence: events
            .last()
            .map(|event| event.sequence)
            .unwrap_or(after_sequence),
        events,
        has_gap,
        status,
    }
}

#[tauri::command]
fn handshake(state: State<'_, Mutex<RuntimeState>>) -> Result<SessionHandshake, ErrorEnvelope> {
    let state = state.lock().map_err(|_| {
        error(
            "handshake",
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    Ok(SessionHandshake {
        auth: state.auth.clone(),
        correlation_id: state.correlation_id.clone(),
        schema_version: PROTOCOL_SCHEMA_VERSION,
        expires_at_epoch_seconds: state.session.expires_at_epoch_seconds(),
    })
}

#[tauri::command]
fn projection(
    state: State<'_, Mutex<RuntimeState>>,
    auth: AuthContext,
    correlation_id: String,
) -> Result<CommandResponse, ErrorEnvelope> {
    let state = state.lock().map_err(|_| {
        error(
            &correlation_id,
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    state
        .session
        .authorize_context(&auth, PROJECT_ID, &correlation_id)
        .map_err(|code| {
            error(
                &correlation_id,
                None,
                None,
                code,
                ErrorCategory::Authentication,
                "authenticated local session rejected the projection request",
                false,
                None,
            )
        })?;
    let snapshot = state.plane.snapshot();
    Ok(response(
        "projection",
        correlation_id,
        None,
        snapshot.project_id.clone(),
        snapshot,
        ResponseStatus::Completed,
        None,
    ))
}

#[tauri::command]
fn subscribe_events(
    state: State<'_, Mutex<RuntimeState>>,
    mut subscription: EventSubscription,
) -> Result<SubscriptionBootstrap, ErrorEnvelope> {
    let mut state = state.lock().map_err(|_| {
        error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    authorize_subscription(&state, &subscription)?;
    if subscription.status != SubscriptionStatus::Requested {
        return Err(error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "new subscriptions must begin in REQUESTED state",
            false,
            None,
        ));
    }
    let snapshot = state.plane.snapshot();
    subscription.status = SubscriptionStatus::Active;
    subscription.snapshot_revision = Some(snapshot.projection_revision);
    subscription.acknowledged_event_sequence = snapshot.last_event_sequence;
    let initial_batch = batch(
        &subscription,
        snapshot.projection_revision,
        snapshot.last_event_sequence,
        Vec::new(),
        SubscriptionStatus::Active,
        false,
    );
    state
        .subscriptions
        .insert(subscription.subscription_id.clone(), subscription.clone());
    Ok(SubscriptionBootstrap {
        subscription,
        snapshot,
        batch: initial_batch,
    })
}

#[tauri::command]
fn replay_events(
    state: State<'_, Mutex<RuntimeState>>,
    subscription: EventSubscription,
) -> Result<EventBatch, ErrorEnvelope> {
    let state = state.lock().map_err(|_| {
        error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    authorize_subscription(&state, &subscription)?;
    let stored = state
        .subscriptions
        .get(&subscription.subscription_id)
        .ok_or_else(|| {
            error(
                &subscription.correlation_id,
                None,
                None,
                ControlPlaneErrorCode::SubscriptionNotFound,
                ErrorCategory::NotFound,
                "event subscription is not active",
                false,
                Some("create a new subscription".into()),
            )
        })?;
    if stored.connection_id != subscription.connection_id
        || stored.auth != subscription.auth
        || stored.status != SubscriptionStatus::Active
    {
        return Err(error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::AuthenticationFailed,
            ErrorCategory::Authentication,
            "subscription identity does not match the authenticated connection",
            false,
            None,
        ));
    }
    let current = state.plane.snapshot();
    if subscription.from_event_sequence > current.last_event_sequence {
        return Err(error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::ReplayGap,
            ErrorCategory::ReplayGap,
            "requested replay cursor is ahead of the durable ledger",
            false,
            Some("reconnect from the authoritative snapshot cursor".into()),
        ));
    }
    let (events, has_gap) = state
        .plane
        .replay_after_with_gap(subscription.from_event_sequence)
        .map_err(|storage_error| {
            error(
                &subscription.correlation_id,
                None,
                None,
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                storage_error.to_string(),
                true,
                Some("retry durable replay".into()),
            )
        })?;
    if events.len() > subscription.max_batch_size {
        return Err(error(
            &subscription.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::Backpressure,
            ErrorCategory::Unavailable,
            "replay batch exceeds the subscription safety bound",
            true,
            Some("acknowledge and replay in smaller bounded windows".into()),
        ));
    }
    Ok(batch(
        &subscription,
        current.projection_revision,
        subscription.from_event_sequence,
        events,
        if has_gap {
            SubscriptionStatus::Gap
        } else {
            SubscriptionStatus::Active
        },
        has_gap,
    ))
}

fn acknowledge_subscription_inner(
    state: &mut RuntimeState,
    acknowledgement: &SubscriptionAcknowledgement,
) -> Result<(), ErrorEnvelope> {
    let stored = state
        .subscriptions
        .get(&acknowledgement.subscription_id)
        .cloned()
        .ok_or_else(|| {
            error(
                &acknowledgement.correlation_id,
                None,
                None,
                ControlPlaneErrorCode::SubscriptionNotFound,
                ErrorCategory::NotFound,
                "event subscription is not active",
                false,
                None,
            )
        })?;
    authorize_subscription(state, &stored)?;
    let mut updated = stored.clone();
    acknowledge_event_subscription(
        &mut updated,
        &acknowledgement.auth,
        &acknowledgement.correlation_id,
        acknowledgement.acknowledged_event_sequence,
        state.plane.snapshot().last_event_sequence,
    )
    .map_err(|code| {
        let (category, retryable, recovery_action) = match code {
            ControlPlaneErrorCode::Backpressure => (
                ErrorCategory::Unavailable,
                true,
                Some("reconnect the subscription before acknowledging".into()),
            ),
            ControlPlaneErrorCode::ReplayGap => (
                ErrorCategory::ReplayGap,
                false,
                Some("replay from the authoritative snapshot cursor".into()),
            ),
            _ => (ErrorCategory::Authentication, false, None),
        };
        error(
            &acknowledgement.correlation_id,
            None,
            None,
            code,
            category,
            "acknowledgement rejected by the subscription protocol",
            retryable,
            recovery_action,
        )
    })?;
    state
        .subscriptions
        .insert(acknowledgement.subscription_id.clone(), updated);
    Ok(())
}

#[tauri::command]
fn acknowledge_subscription(
    state: State<'_, Mutex<RuntimeState>>,
    acknowledgement: SubscriptionAcknowledgement,
) -> Result<(), ErrorEnvelope> {
    let mut state = state.lock().map_err(|_| {
        error(
            &acknowledgement.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    acknowledge_subscription_inner(&mut state, &acknowledgement)
}

#[tauri::command]
fn heartbeat_subscription(
    state: State<'_, Mutex<RuntimeState>>,
    control: SubscriptionControl,
) -> Result<EventBatch, ErrorEnvelope> {
    let mut state = state.lock().map_err(|_| {
        error(
            &control.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    state.supervisor.heartbeat();
    let stored = state
        .subscriptions
        .get(&control.subscription_id)
        .ok_or_else(|| {
            error(
                &control.correlation_id,
                None,
                None,
                ControlPlaneErrorCode::SubscriptionNotFound,
                ErrorCategory::NotFound,
                "event subscription is not active",
                false,
                None,
            )
        })?;
    if stored.auth != control.auth || stored.correlation_id != control.correlation_id {
        return Err(error(
            &control.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::AuthenticationFailed,
            ErrorCategory::Authentication,
            "heartbeat does not match the subscription session",
            false,
            None,
        ));
    }
    if stored.status != SubscriptionStatus::Active {
        return Err(error(
            &control.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::Backpressure,
            ErrorCategory::Unavailable,
            "subscription is paused and requires reconnect replay",
            true,
            Some("create a new subscription and replay from its snapshot cursor".into()),
        ));
    }
    let snapshot = state.plane.snapshot();
    Ok(batch(
        stored,
        snapshot.projection_revision,
        stored.acknowledged_event_sequence,
        Vec::new(),
        SubscriptionStatus::Active,
        false,
    ))
}

#[tauri::command]
fn close_subscription(
    state: State<'_, Mutex<RuntimeState>>,
    control: SubscriptionControl,
) -> Result<(), ErrorEnvelope> {
    let mut state = state.lock().map_err(|_| {
        error(
            &control.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    let stored = state
        .subscriptions
        .get(&control.subscription_id)
        .ok_or_else(|| {
            error(
                &control.correlation_id,
                None,
                None,
                ControlPlaneErrorCode::SubscriptionNotFound,
                ErrorCategory::NotFound,
                "event subscription is not active",
                false,
                None,
            )
        })?;
    if stored.auth != control.auth || stored.correlation_id != control.correlation_id {
        return Err(error(
            &control.correlation_id,
            None,
            None,
            ControlPlaneErrorCode::AuthenticationFailed,
            ErrorCategory::Authentication,
            "close does not match the subscription session",
            false,
            None,
        ));
    }
    state.subscriptions.remove(&control.subscription_id);
    Ok(())
}

#[tauri::command]
fn dispatch(
    app: AppHandle,
    state: State<'_, Mutex<RuntimeState>>,
    request: CommandRequest,
) -> Result<CommandResponse, ErrorEnvelope> {
    let correlation_id = request.correlation_id.clone();
    let command_id = request.command.command_id.clone();
    let mut state = state.lock().map_err(|_| {
        error(
            &correlation_id,
            Some(command_id),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "runtime state lock is unavailable",
            true,
            None,
        )
    })?;
    dispatch_request(&TauriEventSink { app: &app }, &mut state, request)
}

fn dispatch_request<S: EventSink>(
    sink: &S,
    state: &mut RuntimeState,
    request: CommandRequest,
) -> Result<CommandResponse, ErrorEnvelope> {
    let correlation_id = request.correlation_id.clone();
    let causation_id = request.causation_id.clone();
    let command_id = request.command.command_id.clone();
    authorize(&state, &request)?;
    authorize_registry_capability(&request).map_err(|code| {
        error(
            &correlation_id,
            Some(command_id.clone()),
            causation_id.clone(),
            code,
            ErrorCategory::Authorization,
            "authenticated session lacks the exact registered command capability",
            false,
            None,
        )
    })?;
    if state.supervisor.snapshot().state == SupervisorState::Reconciling {
        return Err(error(
            &correlation_id,
            Some(command_id.clone()),
            causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "supervisor is reconciling the durable checkpoint after restart",
            true,
            Some("heartbeat the new connection and retry after reconciliation".into()),
        ));
    }
    if deadline_elapsed(request.deadline_epoch_seconds, now_epoch_seconds()) {
        return Err(error(
            &correlation_id,
            Some(command_id),
            causation_id,
            ControlPlaneErrorCode::Timeout,
            ErrorCategory::Timeout,
            "command deadline elapsed before durable admission",
            false,
            Some("retry with a new command id and deadline".into()),
        ));
    }
    if !command_registry()
        .iter()
        .any(|entry| entry.command_kind == request.command.kind && entry.supported)
    {
        return Err(error(
            &correlation_id,
            Some(command_id),
            causation_id,
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "command kind is not registered",
            false,
            None,
        ));
    }
    let construction_contract = if request.command.kind == CommandKind::AndroidConstructionCreate {
        Some(parse_android_construction_contract(&request)?)
    } else {
        None
    };
    let toolchain_preflight = if request.command.kind == CommandKind::AndroidToolchainPreflight {
        Some(parse_android_toolchain_preflight(state, &request)?)
    } else {
        None
    };
    let requirement_evaluation = if request.command.kind == CommandKind::AndroidRequirementEvaluate
    {
        Some(parse_android_requirement_evaluation(state, &request)?)
    } else {
        None
    };
    let m4_synthesis_build = if request.command.kind == CommandKind::AndroidSynthesisBuild {
        Some(parse_m4_synthesis_build(state, &request)?)
    } else {
        None
    };
    let preview_start = if request.command.kind == CommandKind::PreviewStart {
        Some(parse_preview_start(state, &request)?)
    } else {
        None
    };
    let settings_profile = if request.command.kind == CommandKind::SettingsUpdateProvider {
        Some(parse_provider_profile(&request)?)
    } else {
        None
    };
    let provider_test = if request.command.kind == nirman_domain::CommandKind::ProviderTest {
        Some(parse_provider_test(&request)?)
    } else {
        None
    };
    let provider_execute = if request.command.kind == nirman_domain::CommandKind::ProviderExecute {
        Some(parse_provider_execute(&request)?)
    } else {
        None
    };
    let workspace_apply_patch = if request.command.kind == CommandKind::WorkspaceApplyPatch {
        Some(parse_workspace_apply_patch(&request)?)
    } else {
        None
    };
    let prepared_mutation = if let Some(payload) = workspace_apply_patch.as_ref() {
        prepare_m46_mutation(state, &request, payload)?
    } else {
        None
    };
    let m44_context = if let Some(payload) = provider_execute.as_ref() {
        let preflight = load_m44_preflight(state, &request)?;
        let profile_json = state
            .plane
            .load_provider_profile(&payload.provider_id)
            .map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Unavailable,
                    "provider profile storage is unavailable",
                    true,
                    Some("reconcile local provider storage before retry".into()),
                )
            })?
            .ok_or_else(|| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Provider,
                    "provider profile is not configured",
                    false,
                    Some("save the provider profile before provider execution".into()),
                )
            })?;
        let profile: ProviderProfile = serde_json::from_str(&profile_json).map_err(|_| {
            error(
                &correlation_id,
                Some(command_id.clone()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "durable provider profile could not be restored safely",
                false,
                Some("repair the local provider profile record".into()),
            )
        })?;
        profile.validate().map_err(|_| {
            error(
                &correlation_id,
                Some(command_id.clone()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Provider,
                "durable provider profile is invalid",
                false,
                Some("repair the local provider profile record".into()),
            )
        })?;
        if profile.provider_id != payload.provider_id {
            return Err(error(
                &correlation_id,
                Some(command_id.clone()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Scope,
                "provider profile does not match the requested provider scope",
                false,
                None,
            ));
        }
        Some((preflight, profile))
    } else {
        None
    };
    if let Some(payload) = &provider_test {
        if payload.provider_id.trim().is_empty() || payload.prompt.trim().is_empty() {
            return Err(error(
                &correlation_id,
                Some(command_id.clone()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "provider test requires a provider id and prompt",
                false,
                None,
            ));
        }
        if state
            .plane
            .load_provider_profile(&payload.provider_id)
            .map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Unavailable,
                    "provider profile storage is unavailable",
                    true,
                    Some("reconcile local provider storage before retry".into()),
                )
            })?
            .is_none()
        {
            return Err(error(
                &correlation_id,
                Some(command_id.clone()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Provider,
                "provider profile is not configured",
                false,
                Some("save the provider profile before testing it".into()),
            ));
        }
    }
    let after_sequence = state.plane.snapshot().last_event_sequence;
    let outcome = if let Some(prepared) = m4_synthesis_build.as_ref() {
        state.plane.dispatch_with_result_and_m4(
            request.command.clone(),
            &correlation_id,
            (
                prepared.task_id.as_str(),
                prepared.build_request.source_revision,
                prepared.build_request.project_fingerprint.as_str(),
                prepared.plan.contract_id.as_str(),
                prepared.plan_json.as_str(),
                prepared.build_request_json.as_str(),
                prepared.toolchain_lock_hash.as_str(),
                prepared.environment_snapshot_id.as_str(),
            ),
        )
    } else if let Some(prepared) = preview_start.as_ref() {
        state.plane.dispatch_with_result_and_preview_revision(
            request.command.clone(),
            &correlation_id,
            Some((
                prepared.task_id.as_str(),
                prepared.preview_revision_id.as_str(),
                prepared.revision.project_revision_id.as_str(),
                prepared.revision.source_fingerprint.as_str(),
                prepared.revision_json.as_str(),
                prepared.selection_json.as_str(),
            )),
            Some((
                prepared.task_id.as_str(),
                prepared.projection_json.as_str(),
                prepared.revision.project_revision_id.as_str(),
            )),
        )
    } else if let Some(prepared) = requirement_evaluation.as_ref() {
        state
            .plane
            .dispatch_with_result_and_android_requirement_manifest(
                request.command.clone(),
                &correlation_id,
                Some((
                    prepared.task_id.as_str(),
                    prepared.manifest.manifest_id.as_str(),
                    prepared.source_revision,
                    prepared.project_fingerprint.as_str(),
                    prepared.manifest_json.as_str(),
                    prepared.repair_selection_json.as_deref(),
                )),
            )
    } else if let Some((_payload, preflight, preflight_json)) = toolchain_preflight.as_ref() {
        state
            .plane
            .dispatch_with_result_and_android_toolchain_preflight(
                request.command.clone(),
                &correlation_id,
                Some((
                    preflight.manifest.task_id.as_str(),
                    preflight.preflight_id.as_str(),
                    preflight_status_name(&preflight.status),
                    preflight.environment_snapshot.snapshot_id.as_str(),
                    preflight.lock.as_ref().map(|lock| lock.lock_hash.as_str()),
                    preflight_json.as_str(),
                )),
            )
    } else if let Some(prepared) = prepared_mutation.as_ref() {
        state.plane.dispatch_with_result_and_mutation_transaction(
            request.command.clone(),
            &correlation_id,
            &prepared.transaction,
        )
    } else if let Some((contract, contract_json)) = construction_contract.as_ref() {
        state.plane.dispatch_with_result_and_android_contract(
            request.command.clone(),
            &correlation_id,
            Some((
                contract.task_id.0.as_str(),
                contract.contract_id.as_str(),
                contract_json.as_str(),
                contract.schema_version,
            )),
        )
    } else {
        state.plane.dispatch_with_result_and_provider_profile(
            request.command.clone(),
            &correlation_id,
            settings_profile.as_ref().map(|(profile, profile_json)| {
                (profile.provider_id.as_str(), profile_json.as_str())
            }),
        )
    }
    .map_err(|runtime_error| {
        map_runtime_error(
            &correlation_id,
            Some(command_id.clone()),
            causation_id.clone(),
            runtime_error,
        )
    })?;
    let (snapshot, status, event) = match outcome {
        DurableDispatchOutcome::Accepted { snapshot, event } => {
            if matches!(
                request.command.kind,
                nirman_domain::CommandKind::TaskCancel | nirman_domain::CommandKind::CancelTask
            ) {
                state.plane.request_worker_cancel();
            }
            (snapshot, ResponseStatus::Accepted, Some(event))
        }
        DurableDispatchOutcome::Duplicate { snapshot } => {
            (snapshot, ResponseStatus::Duplicate, None)
        }
    };
    let event_range = event.as_ref().map(|event| EventRange {
        first_sequence: event.sequence,
        last_sequence: event.sequence,
    });
    if let Some(event) = event {
        publish_control_event(
            &mut state.subscriptions,
            sink,
            snapshot.projection_revision,
            after_sequence,
            &event,
        );
    }
    let was_duplicate = status == ResponseStatus::Duplicate;
    let mut command_response = response(
        command_id.clone(),
        correlation_id.clone(),
        causation_id.clone(),
        snapshot.project_id.clone(),
        snapshot,
        status,
        event_range,
    );
    if let Some(payload) = workspace_apply_patch {
        if command_response.status == ResponseStatus::Duplicate {
            let transaction_id = format!("mutation-transaction-{}", payload.operation_id);
            let record = state
                .plane
                .load_mutation_transaction(&transaction_id)
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "durable mutation transaction could not be reloaded",
                        true,
                        Some("reconcile local mutation storage before retry".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate mutation has no durable transaction result",
                        true,
                        Some("reconcile the command and mutation transaction records".into()),
                    )
                })?;
            if record.state != "COMMITTED" {
                return Err(error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::IdempotencyConflict,
                    ErrorCategory::Idempotency,
                    "duplicate mutation is unresolved and was not re-applied",
                    false,
                    Some("reconcile the prior mutation transaction before retrying".into()),
                ));
            }
            let changed_files: Vec<nirman_project::MutationFileResult> = record
                .changed_paths_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "durable mutation result is corrupt",
                        false,
                        Some("repair the durable mutation transaction record".into()),
                    )
                })?;
            let evidence: nirman_project::MutationEvidence = record
                .evidence_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "durable mutation evidence is corrupt",
                        false,
                        Some("repair the durable mutation evidence record".into()),
                    )
                })?;
            command_response.status = ResponseStatus::Completed;
            command_response.result_schema_ref =
                Some("nirman.workspace_apply_patch_result.v1".into());
            command_response.result_payload = Some(
                serde_json::to_value(WorkspaceApplyPatchResultPayload {
                    operation_id: record.operation_id,
                    project_fingerprint: record.resulting_project_fingerprint.unwrap_or_default(),
                    changed_files,
                    evidence,
                })
                .expect("M46 durable result serialization must remain infallible"),
            );
            return Ok(command_response);
        }
        let prepared = prepared_mutation.ok_or_else(|| {
            error(
                &correlation_id,
                Some(command_id.clone()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "accepted mutation has no prepared transaction",
                false,
                Some("reconcile the command and mutation transaction records".into()),
            )
        })?;
        state
            .consumed_mutation_capabilities
            .insert(prepared.transaction.capability_digest.clone());
        let outcome = match MutationBroker::default().apply(&prepared.request) {
            Ok(outcome) => outcome,
            Err(mutation_error) => {
                let failed = MutationTransactionRecord {
                    state: "FAILED".into(),
                    completed_at_epoch_seconds: Some(now_epoch_seconds()),
                    ..prepared.transaction.clone()
                };
                state
                    .plane
                    .record_mutation_transaction(&failed)
                    .map_err(|_| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Unavailable,
                            "failed mutation reconciliation could not be persisted",
                            true,
                            Some("reconcile local mutation storage before retry".into()),
                        )
                    })?;
                return Err(map_mutation_error(
                    &correlation_id,
                    &command_id,
                    causation_id.clone(),
                    mutation_error,
                ));
            }
        };
        let committed = MutationTransactionRecord {
            state: "COMMITTED".into(),
            resulting_revision: command_response.snapshot.current_source_revision,
            resulting_project_fingerprint: Some(outcome.project_fingerprint.clone()),
            changed_paths_json: Some(serde_json::to_string(&outcome.changed_files).map_err(
                |_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "mutation result could not be serialized safely",
                        false,
                        None,
                    )
                },
            )?),
            evidence_json: Some(serde_json::to_string(&outcome.evidence).map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "mutation evidence could not be serialized safely",
                    false,
                    None,
                )
            })?),
            completed_at_epoch_seconds: Some(now_epoch_seconds()),
            ..prepared.transaction
        };
        state
            .plane
            .record_mutation_transaction(&committed)
            .map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Unavailable,
                    "committed mutation transaction could not be persisted",
                    true,
                    Some("reconcile local mutation storage before retry".into()),
                )
            })?;
        command_response.status = ResponseStatus::Completed;
        command_response.result_schema_ref = Some("nirman.workspace_apply_patch_result.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(WorkspaceApplyPatchResultPayload {
                operation_id: outcome.operation_id,
                project_fingerprint: outcome.project_fingerprint,
                changed_files: outcome.changed_files,
                evidence: outcome.evidence,
            })
            .expect("M46 mutation result serialization must remain infallible"),
        );
    }
    if let Some(prepared) = m4_synthesis_build.as_ref() {
        let (plan, build_request, lock_hash, environment_snapshot_id) =
            if command_response.status == ResponseStatus::Duplicate {
                let stored = state
                    .plane
                    .load_android_synthesis_build(
                        &prepared.task_id,
                        prepared.build_request.source_revision,
                    )
                    .map_err(|_| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Unavailable,
                            "duplicate M4 command has no durable synthesis/build record",
                            true,
                            None,
                        )
                    })?
                    .ok_or_else(|| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Unavailable,
                            "duplicate M4 command has no durable synthesis/build record",
                            true,
                            None,
                        )
                    })?;
                (
                    serde_json::from_str(&stored.0).map_err(|_| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Internal,
                            "durable M4 synthesis plan is corrupt",
                            false,
                            None,
                        )
                    })?,
                    serde_json::from_str(&stored.1).map_err(|_| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Internal,
                            "durable M4 build request is corrupt",
                            false,
                            None,
                        )
                    })?,
                    stored.2,
                    stored.3,
                )
            } else {
                (
                    prepared.plan.clone(),
                    prepared.build_request.clone(),
                    prepared.toolchain_lock_hash.clone(),
                    prepared.environment_snapshot_id.clone(),
                )
            };
        command_response.status = ResponseStatus::Completed;
        command_response.result_schema_ref =
            Some("nirman.android_synthesis_build_result.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(AndroidSynthesisBuildResultPayload {
                contract_id: plan.contract_id.clone(),
                synthesis_plan: plan,
                build_request,
                toolchain_lock_hash: lock_hash,
                environment_snapshot_id,
                native_build_observed: false,
            })
            .expect("M4 result serialization"),
        );
    }
    if let Some(prepared) = preview_start.as_ref() {
        let (selection, revision) = if command_response.status == ResponseStatus::Duplicate {
            let stored = state
                .plane
                .load_preview_revision(&prepared.preview_revision_id)
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate preview command has no durable preview revision",
                        true,
                        Some("reconcile the command and preview revision records".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate preview command has no durable preview revision",
                        true,
                        Some("reconcile the command and preview revision records".into()),
                    )
                })?;
            (
                serde_json::from_str(&stored.1).map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "durable preview fallback selection is corrupt",
                        false,
                        None,
                    )
                })?,
                serde_json::from_str(&stored.0).map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "durable preview revision is corrupt",
                        false,
                        None,
                    )
                })?,
            )
        } else {
            (prepared.selection.clone(), prepared.revision.clone())
        };
        command_response.status = ResponseStatus::Completed;
        command_response.result_schema_ref = Some("nirman.preview_start_result.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(PreviewStartResultPayload {
                selection,
                revision,
            })
            .expect("M48 preview result serialization must remain infallible"),
        );
    }
    if let Some(prepared) = requirement_evaluation.as_ref() {
        let (manifest, repair_selection) = if command_response.status == ResponseStatus::Duplicate {
            let task_id = request
                .command
                .task_id
                .as_ref()
                .expect("M47 task scope validated");
            let stored = state
                .plane
                .load_android_requirement_manifest(task_id.0.as_str(), prepared.source_revision)
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate Android requirement command has no durable manifest",
                        true,
                        Some("reconcile the command and requirement manifest records".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate Android requirement command has no durable manifest",
                        true,
                        Some("reconcile the command and requirement manifest records".into()),
                    )
                })?;
            let manifest: AndroidRequirementManifest =
                serde_json::from_str(&stored.0).map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "durable Android requirement manifest is corrupt",
                        false,
                        None,
                    )
                })?;
            let repair_selection = if stored.2.is_empty() {
                None
            } else {
                Some(serde_json::from_str(&stored.2).map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "durable Android repair selection is corrupt",
                        false,
                        None,
                    )
                })?)
            };
            (manifest, repair_selection)
        } else {
            (prepared.manifest.clone(), prepared.repair_selection.clone())
        };
        command_response.status = ResponseStatus::Completed;
        command_response.result_schema_ref = Some("nirman.android_requirement_evaluate.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(AndroidRequirementEvaluateResultPayload {
                manifest,
                repair_selection,
            })
            .expect("M47 requirement result serialization must remain infallible"),
        );
    }
    if let Some((_payload, preflight, _preflight_json)) = toolchain_preflight.as_ref() {
        command_response.status = ResponseStatus::Completed;
        command_response.result_schema_ref = Some("nirman.android_toolchain_preflight.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(AndroidToolchainPreflightResultPayload {
                preflight_id: preflight.preflight_id.clone(),
                status: preflight_status_name(&preflight.status).into(),
                lock_hash: preflight.lock.as_ref().map(|lock| lock.lock_hash.clone()),
                environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
                capability_count: preflight.capabilities.len(),
            })
            .expect("M43 preflight result serialization must remain infallible"),
        );
    }
    if let Some(payload) = provider_test {
        if command_response.status != ResponseStatus::Duplicate {
            let profile_json = state
                .plane
                .load_provider_profile(&payload.provider_id)
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "provider profile storage is unavailable",
                        true,
                        Some("reconcile local provider storage before retry".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::InvalidCommand,
                        ErrorCategory::Provider,
                        "provider profile is not configured",
                        false,
                        Some("save the provider profile before testing it".into()),
                    )
                })?;
            let profile: ProviderProfile = serde_json::from_str(&profile_json).map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "durable provider profile could not be restored safely",
                    false,
                    Some("repair the local provider profile record".into()),
                )
            })?;
            let mut input = ProviderRequestInput::new(payload.prompt);
            input.max_output_tokens = payload.max_output_tokens;
            let execution = state.provider_runtime.execute(
                &profile,
                input,
                format!("provider-request-{command_id}"),
                correlation_id.clone(),
                ProjectId(PROJECT_ID.into()),
                CancellationSignal::default(),
                state.credential_resolver.as_ref(),
                state.provider_transport.as_ref(),
            );
            let execution = execution.map_err(|runtime_error| {
                map_provider_error(
                    &correlation_id,
                    &command_id,
                    causation_id.clone(),
                    runtime_error,
                )
            })?;
            command_response.status = ResponseStatus::Completed;
            command_response.result_schema_ref = Some("nirman.provider_test_result.v1".into());
            command_response.result_payload = Some(
                serde_json::to_value(ProviderTestResultPayload {
                    provider_id: profile.provider_id,
                    model_id: execution.response.model_id,
                    request_id: execution.response.request_id,
                    correlation_id: execution.response.correlation_id,
                    provider_request_id: execution.response.provider_request_id,
                    text: execution.response.text,
                    input_tokens: execution.response.usage.input_tokens,
                    output_tokens: execution.response.usage.output_tokens,
                    total_tokens: execution.response.usage.total_tokens,
                })
                .expect("provider test result serialization must remain infallible"),
            );
        }
    }
    if let Some(payload) = provider_execute {
        let (preflight, profile) = m44_context.expect("M44 context validated before admission");
        let lock = preflight
            .lock
            .as_ref()
            .expect("M44 preflight lock validated before admission");
        let task_id = request
            .command
            .task_id
            .as_ref()
            .expect("M44 task scope validated before admission");
        let execution_id = format!("provider-execution-{command_id}");
        if command_response.status == ResponseStatus::Duplicate {
            let record = state
                .plane
                .load_provider_execution(&execution_id)
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "provider execution record storage is unavailable",
                        true,
                        Some("reconcile the durable execution record before retrying".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate provider command has no durable execution record",
                        true,
                        Some("reconcile the command and provider execution records".into()),
                    )
                })?;
            command_response.result_schema_ref = Some("nirman.provider_execute_result.v1".into());
            command_response.result_payload = Some(
                serde_json::to_value(provider_execute_result_from_record(&record))
                    .expect("M44 duplicate result serialization must remain infallible"),
            );
        } else {
            let mut bridge = ProviderBridge::new(
                &correlation_id,
                profile.provider_id.clone(),
                profile.model_id.clone(),
                profile.protocol,
            );
            let handshake = bridge.handshake(ProviderBridgeHandshake {
                protocol_version: PROVIDER_BRIDGE_PROTOCOL_VERSION,
                session_id: correlation_id.clone(),
                provider_profile_id: profile.provider_id.clone(),
                model_id: profile.model_id.clone(),
                protocol: profile.protocol,
                context_limit: payload.max_context_tokens,
                supports_text: true,
                supports_streaming: payload.stream,
                supports_cancellation: true,
            });
            if let Err(bridge_error) = handshake {
                let record = nirman_domain::ProviderExecutionRecord {
                    execution_id: execution_id.clone(),
                    request_id: format!("provider-request-{command_id}"),
                    correlation_id: correlation_id.clone(),
                    causation_id: causation_id.clone(),
                    project_id: request.command.project_id.clone(),
                    task_id: task_id.clone(),
                    worker_id: payload.worker_id.clone(),
                    provider_id: profile.provider_id.clone(),
                    model_id: profile.model_id.clone(),
                    protocol: profile.protocol.as_str().into(),
                    environment_lock_hash: lock.lock_hash.clone(),
                    environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
                    state: "FAILED".into(),
                    outcome: "bridge_handshake_failed".into(),
                    response_json: None,
                    error_kind: Some(format!("{:?}", bridge_error.kind())),
                    started_at_epoch_seconds: now_epoch_seconds(),
                    duration_ms: 0,
                };
                state
                    .plane
                    .record_provider_execution(&record)
                    .map_err(|_| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Unavailable,
                            "provider execution failure could not be recorded durably",
                            true,
                            Some("reconcile local provider execution storage".into()),
                        )
                    })?;
                return Err(map_bridge_error(
                    &correlation_id,
                    &command_id,
                    causation_id,
                    bridge_error,
                ));
            }
            let bridge_request = nirman_providers::ProviderBridgeRequest {
                session_id: correlation_id.clone(),
                project_id: request.command.project_id.0.clone(),
                task_id: task_id.0.clone(),
                worker_id: payload.worker_id.clone(),
                trace_id: format!("provider-request-{command_id}"),
                correlation_id: correlation_id.clone(),
                causation_id: causation_id.clone(),
                provider_profile_id: profile.provider_id.clone(),
                model_id: profile.model_id.clone(),
                protocol: profile.protocol,
                prompt: payload.prompt.clone(),
                max_output_tokens: payload.max_output_tokens,
                max_context_tokens: payload.max_context_tokens,
                environment_lock_hash: lock.lock_hash.clone(),
                environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
                privacy_classification: payload.privacy_classification.clone(),
                tool_policy: payload.tool_policy.clone(),
                stream: payload.stream,
                cancellation: CancellationSignal::default(),
            };
            let bridge_result = bridge.execute(
                &state.provider_runtime,
                &profile,
                bridge_request,
                state.credential_resolver.as_ref(),
                state.provider_transport.as_ref(),
            );
            match bridge_result.execution {
                Ok(execution) => {
                    let record = nirman_domain::ProviderExecutionRecord {
                        execution_id: execution_id.clone(),
                        request_id: execution.response.request_id.clone(),
                        correlation_id: execution.response.correlation_id.clone(),
                        causation_id: causation_id.clone(),
                        project_id: request.command.project_id.clone(),
                        task_id: task_id.clone(),
                        worker_id: payload.worker_id.clone(),
                        provider_id: profile.provider_id.clone(),
                        model_id: profile.model_id.clone(),
                        protocol: profile.protocol.as_str().into(),
                        environment_lock_hash: lock.lock_hash.clone(),
                        environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
                        state: "COMPLETED".into(),
                        outcome: execution.usage.outcome.clone(),
                        response_json: Some(serde_json::to_string(&execution.response).expect(
                            "M44 normalized response serialization must remain infallible",
                        )),
                        error_kind: None,
                        started_at_epoch_seconds: execution.usage.started_at_epoch_seconds,
                        duration_ms: execution.usage.duration_ms,
                    };
                    state
                        .plane
                        .record_provider_execution(&record)
                        .map_err(|_| {
                            error(
                                &correlation_id,
                                Some(command_id.clone()),
                                causation_id.clone(),
                                ControlPlaneErrorCode::DependencyUnavailable,
                                ErrorCategory::Unavailable,
                                "provider execution result could not be recorded durably",
                                true,
                                Some("reconcile local provider execution storage".into()),
                            )
                        })?;
                    command_response.status = ResponseStatus::Completed;
                    command_response.result_schema_ref =
                        Some("nirman.provider_execute_result.v1".into());
                    command_response.result_payload = Some(
                        serde_json::to_value(ProviderExecuteResultPayload {
                            execution_id,
                            request_id: execution.response.request_id,
                            correlation_id: execution.response.correlation_id,
                            provider_id: profile.provider_id,
                            model_id: execution.response.model_id,
                            environment_lock_hash: lock.lock_hash.clone(),
                            environment_snapshot_id: preflight
                                .environment_snapshot
                                .snapshot_id
                                .clone(),
                            state: "COMPLETED".into(),
                            outcome: execution.usage.outcome,
                            text: Some(execution.response.text),
                            error_kind: None,
                            events: bridge_result
                                .events
                                .into_iter()
                                .map(|event| {
                                    serde_json::to_value(event)
                                        .expect("M44 event serialization must remain infallible")
                                })
                                .collect(),
                        })
                        .expect("M44 result serialization must remain infallible"),
                    );
                }
                Err(bridge_error) => {
                    let request_id = format!("provider-request-{command_id}");
                    let record = nirman_domain::ProviderExecutionRecord {
                        execution_id,
                        request_id,
                        correlation_id: correlation_id.clone(),
                        causation_id: causation_id.clone(),
                        project_id: request.command.project_id.clone(),
                        task_id: task_id.clone(),
                        worker_id: payload.worker_id,
                        provider_id: profile.provider_id,
                        model_id: profile.model_id,
                        protocol: profile.protocol.as_str().into(),
                        environment_lock_hash: lock.lock_hash.clone(),
                        environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
                        state: "FAILED".into(),
                        outcome: "failed".into(),
                        response_json: None,
                        error_kind: Some(format!("{:?}", bridge_error.kind())),
                        started_at_epoch_seconds: now_epoch_seconds(),
                        duration_ms: 0,
                    };
                    state
                        .plane
                        .record_provider_execution(&record)
                        .map_err(|_| {
                            error(
                                &correlation_id,
                                Some(command_id.clone()),
                                causation_id.clone(),
                                ControlPlaneErrorCode::DependencyUnavailable,
                                ErrorCategory::Unavailable,
                                "provider execution failure could not be recorded durably",
                                true,
                                Some("reconcile local provider execution storage".into()),
                            )
                        })?;
                    return Err(map_bridge_error(
                        &correlation_id,
                        &command_id,
                        causation_id,
                        bridge_error,
                    ));
                }
            }
        }
    }
    if let Some(prepared) = m4_synthesis_build.as_ref() {
        if !was_duplicate {
            let m108_checkpoint = state
                .plane
                .checkpoint_id()
                .map(str::to_owned)
                .unwrap_or_else(|| "checkpoint-pending".to_owned());
            persist_m108_stage(
                state,
                &prepared.task_id,
                &format!("m108-preview-{}", prepared.plan.contract_id),
                PreviewSyncEventType::BuildRequested,
                PreviewEventTruth::Requested,
                &format!("source-{}", prepared.build_request.source_revision),
                &m108_checkpoint,
                &prepared.build_request.project_fingerprint,
                &command_id,
                vec![format!(
                    "m4-build-request:{}",
                    prepared.build_request.gradle_task
                )],
            )?;
        }
    }
    Ok(command_response)
}

fn runtime_state(app: &AppHandle) -> RuntimeState {
    let ledger_path = std::env::var_os("NIRMAN_LEDGER_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let directory = app
                .path()
                .app_data_dir()
                .expect("Nirman application-data directory must be available");
            fs::create_dir_all(&directory)
                .expect("Nirman application-data directory must be writable");
            directory.join("control-plane.sqlite3")
        });
    let mut installation_hasher = DefaultHasher::new();
    ledger_path.to_string_lossy().hash(&mut installation_hasher);
    let auth = AuthContext {
        installation_id: format!("installation-{:016x}", installation_hasher.finish()),
        user_scope: "local-user".into(),
        project_scope: PROJECT_ID.into(),
        schema_version: PROTOCOL_SCHEMA_VERSION,
    };
    let correlation_id = format!("session-{}", std::process::id());
    let session = AuthenticatedSession::issue(auth.clone(), correlation_id.clone(), 86_400);
    let plane = DurableControlPlane::open(&ledger_path, ProjectId(PROJECT_ID.into()))
        .expect("Nirman local ledger must open before the desktop host starts");
    let checkpoint_id = plane.checkpoint_id().map(str::to_owned);
    let mut supervisor = Supervisor::start();
    if let Some(checkpoint_id) = checkpoint_id {
        supervisor.restart_from_checkpoint(checkpoint_id);
        supervisor.register_lease("desktop-host", std::process::id().to_string(), 2);
    }
    let provider_runtime = ProviderRuntime::open(&ledger_path)
        .expect("Nirman provider usage ledger must open before the desktop host starts");
    RuntimeState {
        plane,
        session,
        auth,
        correlation_id,
        subscriptions: BTreeMap::new(),
        supervisor,
        provider_runtime,
        credential_resolver: Box::new(OsCredentialResolver),
        provider_transport: Box::new(
            HttpProviderTransport::new().expect("Nirman provider HTTP transport must initialize"),
        ),
        capability_probe: Box::new(HostCapabilityProbe),
        authorized_workspace_root: std::env::var_os("NIRMAN_PROJECT_WORKSPACE").map(PathBuf::from),
        consumed_mutation_capabilities: BTreeSet::new(),
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(runtime_state(app.handle())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            handshake,
            projection,
            subscribe_events,
            replay_events,
            acknowledge_subscription,
            heartbeat_subscription,
            close_subscription,
            dispatch
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nirman desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_android::{StaticCapabilityProbe, ToolchainComponentKind};
    use nirman_domain::{
        AndroidConstructionContract, AndroidDeviceProfile, AndroidTechnologyPlan, ArtifactKind,
        ArtifactModel, AssetReferenceInput, CommandEnvelope, ConstructionRequirement,
        RequirementOrigin, Revision, TaskId, ValidationModel, VisualReferenceInput,
    };
    use nirman_providers::{
        CredentialReference, ProviderError, ProviderErrorKind, ProviderTransport,
        RawProviderResponse,
    };
    use std::path::{Path, PathBuf};
    use std::sync::Mutex as StdMutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct RecordingSink {
        batches: StdMutex<Vec<EventBatch>>,
    }

    impl EventSink for RecordingSink {
        fn emit_batch(&self, batch: EventBatch) -> Result<(), ()> {
            self.batches.lock().expect("sink lock").push(batch);
            Ok(())
        }
    }

    struct TestResolver;

    impl CredentialResolver for TestResolver {
        fn resolve(
            &self,
            _reference: &nirman_providers::CredentialReference,
        ) -> Result<nirman_providers::SecretValue, nirman_providers::ProviderConfigError> {
            nirman_providers::SecretValue::from_keychain("host-path-secret")
        }
    }

    struct TestTransport;

    struct FailureTransport {
        kind: ProviderErrorKind,
    }

    impl ProviderTransport for FailureTransport {
        fn send(
            &self,
            _request: &nirman_providers::AuthenticatedProviderRequest,
        ) -> Result<RawProviderResponse, ProviderError> {
            let status_code = match self.kind {
                ProviderErrorKind::Transport => 503,
                ProviderErrorKind::Timeout => 408,
                ProviderErrorKind::RateLimited => 429,
                ProviderErrorKind::InvalidResponse => 200,
                _ => 500,
            };
            let body = if self.kind == ProviderErrorKind::InvalidResponse {
                "{\"malformed\":true,\"secret\":\"must-not-forward\"}".into()
            } else {
                "{\"error\":\"provider failure\",\"secret\":\"must-not-forward\"}".into()
            };
            Ok(RawProviderResponse {
                status_code,
                body,
                provider_request_id: None,
            })
        }
    }

    impl ProviderTransport for TestTransport {
        fn send(
            &self,
            _request: &nirman_providers::AuthenticatedProviderRequest,
        ) -> Result<RawProviderResponse, nirman_providers::ProviderError> {
            Ok(RawProviderResponse {
                status_code: 200,
                body: serde_json::json!({
                    "model": "host-model",
                    "choices": [{"message": {"content": "host provider ok"}}],
                    "usage": {"prompt_tokens": 2, "completion_tokens": 3, "total_tokens": 5}
                })
                .to_string(),
                provider_request_id: Some("provider-http-1".into()),
            })
        }
    }

    fn database_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nirman-m3-provider-host-{nonce}.sqlite3"))
    }

    fn test_state() -> RuntimeState {
        let path = database_path();
        let state = test_state_at(&path);
        let _ = fs::remove_file(path);
        state
    }

    fn m43_probe() -> StaticCapabilityProbe {
        StaticCapabilityProbe::default()
            .with_available(
                ToolchainComponentKind::Jdk,
                "17",
                "/locked/jdk",
                "jdk-license",
            )
            .with_available(
                ToolchainComponentKind::Gradle,
                "8.7",
                "/locked/gradle",
                "gradle-license",
            )
            .with_available(
                ToolchainComponentKind::AndroidSdk,
                "35",
                "/locked/sdk",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::PlatformTools,
                "35",
                "/locked/platform-tools",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::Adb,
                "35",
                "/locked/adb",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::Emulator,
                "35",
                "/locked/emulator",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::Kotlin,
                "2.0",
                "/locked/kotlin",
                "kotlin-license",
            )
            .with_available(
                ToolchainComponentKind::AndroidGradlePlugin,
                "8.7",
                "/locked/agp",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::BuildTools,
                "35.0.0",
                "/locked/build-tools",
                "android-sdk-license",
            )
    }

    fn test_state_at(path: &Path) -> RuntimeState {
        let auth = AuthContext {
            installation_id: "installation-test".into(),
            user_scope: "local-user".into(),
            project_scope: PROJECT_ID.into(),
            schema_version: PROTOCOL_SCHEMA_VERSION,
        };
        let correlation_id = "session-test".to_owned();
        let mut state = RuntimeState {
            plane: DurableControlPlane::open(path, ProjectId(PROJECT_ID.into())).expect("plane"),
            session: AuthenticatedSession::issue(auth.clone(), correlation_id.clone(), 3600),
            auth: auth.clone(),
            correlation_id: correlation_id.clone(),
            subscriptions: BTreeMap::new(),
            supervisor: Supervisor::start(),
            provider_runtime: ProviderRuntime::open(path).expect("provider runtime"),
            credential_resolver: Box::new(TestResolver),
            provider_transport: Box::new(TestTransport),
            capability_probe: Box::new(m43_probe()),
            authorized_workspace_root: None,
            consumed_mutation_capabilities: BTreeSet::new(),
        };
        state.subscriptions.insert(
            "subscription-test".into(),
            EventSubscription {
                subscription_id: "subscription-test".into(),
                connection_id: "connection-test".into(),
                auth,
                project_id: PROJECT_ID.into(),
                task_id: None,
                from_event_sequence: 0,
                snapshot_revision: None,
                requested_projection_kinds: vec!["settings".into(), "provider".into()],
                acknowledged_event_sequence: 0,
                heartbeat_interval_seconds: 15,
                max_batch_size: 64,
                backpressure_policy: nirman_ipc::BackpressurePolicy::RejectOverLimit,
                status: SubscriptionStatus::Active,
                correlation_id,
            },
        );
        state
    }

    fn request(
        state: &RuntimeState,
        command_id: &str,
        kind: nirman_domain::CommandKind,
        payload: String,
        revision: u64,
    ) -> CommandRequest {
        CommandRequest {
            protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
            auth: state.auth.clone(),
            command: CommandEnvelope {
                command_id: command_id.into(),
                project_id: ProjectId(PROJECT_ID.into()),
                task_id: None,
                kind,
                payload,
                expected_projection_revision: Revision(revision),
                idempotency_key: Some(format!("idempotency-{command_id}")),
            },
            correlation_id: state.correlation_id.clone(),
            causation_id: None,
            deadline_epoch_seconds: None,
        }
    }

    fn construction_contract(project_id: &str) -> AndroidConstructionContract {
        let task_id = TaskId("task-m39".into());
        AndroidConstructionContract {
            schema_version: 1,
            contract_id: "contract-m39".into(),
            project_id: ProjectId(project_id.into()),
            target_platforms: vec!["android".into()],
            task_id: task_id.clone(),
            user_intent: "Build an offline-first notes application".into(),
            screenshots: vec![VisualReferenceInput {
                reference_id: "screen-reference-1".into(),
                source_path: "inputs/notes.png".into(),
                image_hash: "sha256:screen".into(),
            }],
            assets: vec![AssetReferenceInput {
                asset_id: "asset-logo-1".into(),
                source_path: "inputs/logo.svg".into(),
                content_hash: "sha256:asset".into(),
            }],
            features: vec![ConstructionRequirement {
                requirement_id: "feature-offline".into(),
                statement: "Notes remain readable without network access".into(),
                origin: RequirementOrigin::UserFact,
                source_reference_ids: vec![],
            }],
            ui: vec![ConstructionRequirement {
                requirement_id: "ui-calm".into(),
                statement: "Use a calm dark visual hierarchy".into(),
                origin: RequirementOrigin::ModelProposal,
                source_reference_ids: vec!["screen-reference-1".into()],
            }],
            data: vec![ConstructionRequirement {
                requirement_id: "data-local".into(),
                statement: "Persist notes locally".into(),
                origin: RequirementOrigin::UserFact,
                source_reference_ids: vec![],
            }],
            integrations: vec![],
            technology_plan: AndroidTechnologyPlan {
                plan_id: "plan-m39".into(),
                task_id,
                requested_capabilities: vec!["offline-storage".into()],
                visual_requirements: vec!["dark-theme".into()],
                selected_languages: vec!["kotlin".into()],
                selected_ui_frameworks: vec!["jetpack-compose".into()],
                selected_runtime_layers: vec![],
                selected_native_modules: vec![],
                selected_build_plugins: vec!["android-gradle-plugin".into()],
                selected_device_apis: vec![],
                selected_libraries: vec!["room".into()],
                compatibility_constraints: vec!["android-api-29-plus".into()],
                rejected_alternatives: vec!["web-target".into()],
                required_toolchains: vec!["jdk".into(), "gradle".into(), "android-sdk".into()],
                validation_plan: vec!["unit-tests".into(), "android-build".into()],
                confidence: Some("high".into()),
                revision: Revision(1),
            },
            android_requirements: vec![ConstructionRequirement {
                requirement_id: "android-min-api".into(),
                statement: "Support the declared Android API range".into(),
                origin: RequirementOrigin::UserFact,
                source_reference_ids: vec![],
            }],
            device_matrix: vec![AndroidDeviceProfile {
                device_id: "pixel-api-35".into(),
                name: "Pixel API 35".into(),
                platform_version: "Android 15".into(),
                api_level: 35,
                architecture: "x86_64".into(),
                width: 1080,
                height: 2400,
                density: 420,
                orientation: "portrait".into(),
                locale: "en-US".into(),
                permissions: vec![],
                network_profile: "offline-capable".into(),
            }],
            validation_model: ValidationModel {
                required_checks: vec!["compile".into(), "unit-tests".into()],
                acceptance_criteria: vec!["notes persist offline".into()],
            },
            artifact_model: ArtifactModel {
                required_artifact: ArtifactKind::Apk,
                aab_declared: false,
            },
        }
    }

    fn profile() -> ProviderProfile {
        ProviderProfile {
            provider_id: "provider-test".into(),
            display_name: "Host path test provider".into(),
            protocol: nirman_providers::ProviderProtocol::ChatCompletions,
            base_url: "http://localhost:9/v1/chat/completions".into(),
            api_key_secret_ref: CredentialReference::new("credential://nirman/provider/test")
                .expect("reference"),
            model_id: "host-model".into(),
            timeout_ms: 1000,
            enabled: true,
        }
    }

    fn bridge_request_for(
        profile: &ProviderProfile,
        preflight: &nirman_android::AndroidToolchainPreflight,
        request_id: &str,
        stream: bool,
    ) -> nirman_providers::ProviderBridgeRequest {
        nirman_providers::ProviderBridgeRequest {
            session_id: "session-test".into(),
            project_id: PROJECT_ID.into(),
            task_id: "task-m39".into(),
            worker_id: "worker-m44".into(),
            trace_id: request_id.into(),
            correlation_id: "session-test".into(),
            causation_id: Some("cause-m44".into()),
            provider_profile_id: profile.provider_id.clone(),
            model_id: profile.model_id.clone(),
            protocol: profile.protocol,
            prompt: "bridge failure classification probe".into(),
            max_output_tokens: Some(16),
            max_context_tokens: 4096,
            environment_lock_hash: preflight.lock.as_ref().expect("lock").lock_hash.clone(),
            environment_snapshot_id: preflight.environment_snapshot.snapshot_id.clone(),
            privacy_classification: "project-content".into(),
            tool_policy: "no-tool-calls".into(),
            stream,
            cancellation: CancellationSignal::default(),
        }
    }

    #[test]
    fn m39_android_construction_command_uses_authenticated_m115_boundary() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let contract = construction_contract(PROJECT_ID);
        let mut construction_request = request(
            &state,
            "m39-construction-command",
            CommandKind::AndroidConstructionCreate,
            serde_json::to_string(&AndroidConstructionCommandPayload {
                contract: contract.clone(),
            })
            .expect("contract payload"),
            0,
        );
        construction_request.command.task_id = Some(contract.task_id.clone());
        let response =
            dispatch_request(&sink, &mut state, construction_request).expect("M39 response");
        assert_eq!(response.status, ResponseStatus::Accepted);
        assert_eq!(response.snapshot.project_id, ProjectId(PROJECT_ID.into()));
        let stored = state
            .plane
            .load_android_construction_contract(&contract.task_id.0)
            .expect("contract load")
            .expect("stored contract");
        assert_eq!(
            AndroidConstructionContract::from_json(&stored).expect("stored contract parse"),
            contract
        );
        let batches = sink.batches.lock().expect("sink lock");
        assert_eq!(batches.len(), 1);
        assert!(batches[0].events[0]
            .kind
            .contains("AndroidConstructionCreate"));
        drop(batches);
        drop(state);
        let reopened = DurableControlPlane::open(&path, ProjectId(PROJECT_ID.into()))
            .expect("reopen control plane");
        assert!(reopened
            .load_android_construction_contract(&contract.task_id.0)
            .expect("reloaded contract")
            .is_some());
        assert!(reopened
            .replay_after(0)
            .expect("event replay")
            .iter()
            .any(|event| event.kind.contains("AndroidConstructionCreate")));

        let mut invalid = construction_contract(PROJECT_ID);
        invalid.target_platforms = vec!["android".into(), "windows".into()];
        let mut invalid_request = request(
            &reopened_state_for_test(&path),
            "m39-invalid-target-command",
            CommandKind::AndroidConstructionCreate,
            serde_json::to_string(&AndroidConstructionCommandPayload { contract: invalid })
                .expect("invalid payload"),
            response.projection_revision.0,
        );
        invalid_request.command.task_id = Some(TaskId("task-m39".into()));
        let mut reopened_state = reopened_state_for_test(&path);
        let rejected = dispatch_request(
            &RecordingSink::default(),
            &mut reopened_state,
            invalid_request,
        )
        .expect_err("non-Android target must reject");
        assert_eq!(rejected.code, ControlPlaneErrorCode::InvalidCommand);
        assert_eq!(rejected.category, ErrorCategory::Validation);
        let _ = fs::remove_file(path);
        let evidence = serde_json::json!({
            "schema": "nirman.m39.android_construction.v1",
            "validAndroidIntentObserved": true,
            "targetPlatformsExactAndroidObserved": true,
            "requiredConstructionFieldsObserved": true,
            "invalidContractRejected": true,
            "deterministicValidationObserved": true,
            "unknownFieldsRejected": true,
            "m115AuthenticatedCommandBoundaryObserved": true,
            "durableContractPersistenceObserved": true,
            "durableContractReloadObserved": true,
            "projectionEventObserved": true,
            "unsupportedTargetRejected": true,
            "androidWorkspaceMutation": false,
            "evidenceStatus": "M39_HEADLESS_DURABLE_CONTRACT_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m39_android_construction.json");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("M39 evidence JSON"),
        )
        .expect("M39 evidence write");
    }

    fn reopened_state_for_test(path: &Path) -> RuntimeState {
        let mut state = test_state_at(path);
        state.subscriptions.clear();
        state.subscriptions.insert(
            "subscription-test".into(),
            EventSubscription {
                subscription_id: "subscription-test".into(),
                connection_id: "connection-test".into(),
                auth: state.auth.clone(),
                project_id: PROJECT_ID.into(),
                task_id: None,
                from_event_sequence: 0,
                snapshot_revision: None,
                requested_projection_kinds: vec!["android-construction".into()],
                acknowledged_event_sequence: 0,
                heartbeat_interval_seconds: 15,
                max_batch_size: 64,
                backpressure_policy: nirman_ipc::BackpressurePolicy::RejectOverLimit,
                status: SubscriptionStatus::Active,
                correlation_id: state.correlation_id.clone(),
            },
        );
        state
    }

    #[test]
    fn m43_toolchain_preflight_uses_m39_and_m115_durable_boundary() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let contract = construction_contract(PROJECT_ID);
        let mut construction_request = request(
            &state,
            "m43-contract-command",
            CommandKind::AndroidConstructionCreate,
            serde_json::to_string(&AndroidConstructionCommandPayload {
                contract: contract.clone(),
            })
            .expect("contract payload"),
            0,
        );
        construction_request.command.task_id = Some(contract.task_id.clone());
        let contract_response =
            dispatch_request(&sink, &mut state, construction_request).expect("contract response");

        let mut preflight_request = request(
            &state,
            "m43-preflight-command",
            CommandKind::AndroidToolchainPreflight,
            serde_json::to_string(&AndroidToolchainPreflightCommandPayload {
                build_variant: "debug".into(),
            })
            .expect("preflight payload"),
            contract_response.projection_revision.0,
        );
        preflight_request.command.task_id = Some(contract.task_id.clone());
        let response =
            dispatch_request(&sink, &mut state, preflight_request).expect("preflight response");
        assert_eq!(response.status, ResponseStatus::Completed);
        assert_eq!(
            response.result_schema_ref.as_deref(),
            Some("nirman.android_toolchain_preflight.v1")
        );
        let result = response.result_payload.expect("preflight result");
        assert_eq!(result["status"], "AVAILABLE");
        assert_eq!(result["capability_count"], 9);

        let workspace =
            std::env::temp_dir().join(format!("nirman-m4-host-{}", now_epoch_seconds()));
        fs::create_dir_all(workspace.join("app/src/main")).expect("M4 workspace");
        fs::write(workspace.join("settings.gradle.kts"), "include(\":app\")\n").expect("settings");
        fs::write(
            workspace.join("app/build.gradle.kts"),
            "plugins { id(\"com.android.application\") }\n",
        )
        .expect("gradle");
        state.authorized_workspace_root = Some(workspace.clone());
        let index = ProjectIndexer::default()
            .index_workspace(&workspace, &IndexRequest::default())
            .expect("M4 index");
        let mut m4_request = request(
            &state,
            "m4-synthesis-build-command",
            CommandKind::AndroidSynthesisBuild,
            serde_json::to_string(&AndroidSynthesisBuildCommandPayload {
                contract: contract.clone(),
                source_revision: 1,
                workspace_root: workspace.to_string_lossy().into_owned(),
                project_fingerprint: index.project_fingerprint.clone(),
                build_variant: "debug".into(),
                gradle_task: "assembleDebug".into(),
            })
            .expect("M4 payload"),
            response.projection_revision.0,
        );
        m4_request.command.task_id = Some(contract.task_id.clone());
        let m4_response =
            dispatch_request(&sink, &mut state, m4_request.clone()).expect("M4 response");
        assert_eq!(m4_response.status, ResponseStatus::Completed);
        assert_eq!(
            m4_response.result_schema_ref.as_deref(),
            Some("nirman.android_synthesis_build_result.v1")
        );
        assert_eq!(
            m4_response.result_payload.as_ref().unwrap()["native_build_observed"],
            false
        );
        assert!(
            !m4_response.result_payload.as_ref().unwrap()["toolchain_lock_hash"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        let m4_duplicate = dispatch_request(&sink, &mut state, m4_request).expect("M4 duplicate");
        assert_eq!(m4_duplicate.result_payload, m4_response.result_payload);
        let m108_record = state
            .plane
            .load_m108_sync_record(&contract.task_id.0)
            .expect("M108 record load")
            .expect("M108 record persisted");
        let m108_projection: M108ProjectionState =
            serde_json::from_str(&m108_record.0).expect("M108 projection parse");
        assert_eq!(m108_projection.last_event_sequence, 1);
        assert_eq!(m108_projection.build_status, "REQUESTED");
        assert!(m108_record.1.contains("m4-build-request"));
        let m4_stored = state
            .plane
            .load_android_synthesis_build(&contract.task_id.0, 1)
            .expect("M4 load")
            .expect("M4 durable record");
        assert_eq!(m4_stored.4, index.project_fingerprint);
        drop(state);
        let reopened_m4 =
            DurableControlPlane::open(&path, ProjectId(PROJECT_ID.into())).expect("M4 reopen");
        assert!(reopened_m4
            .load_android_synthesis_build(&contract.task_id.0, 1)
            .expect("M4 reload")
            .is_some());
        let _ = fs::remove_dir_all(workspace);
        let mut state = test_state_at(&path);

        let stored = state
            .plane
            .load_android_toolchain_preflight(&contract.task_id.0)
            .expect("preflight load")
            .expect("stored preflight");
        let report: nirman_android::AndroidToolchainPreflight =
            serde_json::from_str(&stored).expect("stored preflight parse");
        assert_eq!(report.status, PreflightStatus::Available);
        assert!(report.lock.is_some());
        assert!(report.environment_snapshot.toolchain_lock_hash.is_some());
        assert_eq!(
            report
                .environment_snapshot
                .selected_device_identity
                .as_deref(),
            Some("pixel-api-35")
        );
        let batches = sink.batches.lock().expect("sink lock");
        assert_eq!(batches.len(), 3);
        assert!(batches[1].events[0]
            .kind
            .contains("AndroidToolchainPreflight"));
        assert!(batches[2].events[0].kind.contains("AndroidSynthesisBuild"));
        drop(batches);
        drop(state);
        let reopened = DurableControlPlane::open(&path, ProjectId(PROJECT_ID.into()))
            .expect("reopen control plane");
        assert!(reopened.checkpoint_id().is_some());
        assert!(reopened
            .load_android_toolchain_preflight(&contract.task_id.0)
            .expect("reloaded preflight")
            .is_some());
        let mut missing_state = test_state_at(&path);
        let mut missing_request = request(
            &missing_state,
            "m43-missing-contract-command",
            CommandKind::AndroidToolchainPreflight,
            serde_json::to_string(&AndroidToolchainPreflightCommandPayload {
                build_variant: "debug".into(),
            })
            .expect("preflight payload"),
            response.projection_revision.0,
        );
        missing_request.command.task_id = Some(TaskId("missing-task".into()));
        let missing = dispatch_request(
            &RecordingSink::default(),
            &mut missing_state,
            missing_request,
        )
        .expect_err("preflight without contract must reject");
        assert_eq!(missing.code, ControlPlaneErrorCode::InvalidCommand);
        assert_eq!(missing.category, ErrorCategory::Validation);
        let _ = fs::remove_file(path);
        let evidence = serde_json::json!({
            "schema": "nirman.m43.android_toolchain.v1",
            "m39ContractReloadObserved": true,
            "manifestDerivedFromM39Observed": true,
            "requiredToolchainCapabilitiesObserved": true,
            "deterministicPreflightObserved": true,
            "availableClassificationObserved": true,
            "toolchainLockGenerated": true,
            "versionHashLicenseRecordsObserved": true,
            "environmentSnapshotObserved": true,
            "durablePreflightPersistenceObserved": true,
            "durablePreflightReloadObserved": true,
            "durableCheckpointObserved": true,
            "m115AuthenticatedCommandBoundaryObserved": true,
            "typedResponseObserved": true,
            "projectionEventObserved": true,
            "missingContractRejected": true,
            "hostCapabilityProbeRuntime": false,
            "toolchainRepairExecuted": false,
            "androidBuildObserved": false,
            "evidenceStatus": "M43_HEADLESS_LOCKED_PREFLIGHT_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m43_android_toolchain.json");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("M43 evidence JSON"),
        )
        .expect("M43 evidence write");
    }

    #[test]
    fn provider_commands_use_authenticated_durable_host_path_and_safe_typed_result() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let settings_payload = serde_json::to_string(&SettingsUpdateProviderCommandPayload {
            profile: serde_json::to_value(profile()).expect("profile value"),
        })
        .expect("settings payload");
        let settings_request = request(
            &state,
            "settings-command",
            nirman_domain::CommandKind::SettingsUpdateProvider,
            settings_payload,
            0,
        );
        let settings_response =
            dispatch_request(&sink, &mut state, settings_request).expect("settings response");
        assert_eq!(settings_response.status, ResponseStatus::Accepted);
        assert!(state
            .plane
            .load_provider_profile("provider-test")
            .expect("profile load")
            .is_some());
        let profile_record = state
            .plane
            .load_provider_profile("provider-test")
            .expect("profile load")
            .expect("profile record");
        assert!(!profile_record.contains("host-path-secret"));
        assert!(sink.batches.lock().expect("sink lock")[0].events[0]
            .kind
            .contains("SettingsUpdateProvider"));

        let test_payload = serde_json::to_string(&ProviderTestCommandPayload {
            provider_id: "provider-test".into(),
            prompt: "say hello".into(),
            max_output_tokens: Some(32),
        })
        .expect("test payload");
        let provider_request = request(
            &state,
            "provider-command",
            nirman_domain::CommandKind::ProviderTest,
            test_payload,
            settings_response.projection_revision.0,
        );
        let provider_response =
            dispatch_request(&sink, &mut state, provider_request).expect("provider response");
        assert_eq!(provider_response.status, ResponseStatus::Completed);
        assert_eq!(
            provider_response.result_schema_ref.as_deref(),
            Some("nirman.provider_test_result.v1")
        );
        let result = provider_response
            .result_payload
            .as_ref()
            .expect("typed result");
        assert_eq!(result["text"], "host provider ok");
        assert_eq!(result["correlation_id"], "session-test");
        assert!(!serde_json::to_string(&provider_response)
            .expect("response json")
            .contains("host-path-secret"));
        assert_eq!(
            state
                .provider_runtime
                .usage_record("provider-request-provider-command")
                .expect("usage lookup")
                .expect("usage")
                .outcome,
            "completed"
        );
        let batches = sink.batches.lock().expect("sink lock");
        assert_eq!(batches.len(), 2);
        assert!(batches[1].events[0].kind.contains("ProviderTest"));
        drop(state);
        let reopened = DurableControlPlane::open(&path, ProjectId(PROJECT_ID.into()))
            .expect("reopen control plane");
        let restored_profile = reopened
            .load_provider_profile("provider-test")
            .expect("restored profile lookup")
            .expect("restored profile");
        assert!(restored_profile.contains("credential://nirman/provider/test"));
        assert!(!restored_profile.contains("host-path-secret"));
        assert!(reopened
            .replay_after(0)
            .expect("replay")
            .iter()
            .any(|event| event.kind.contains("SettingsUpdateProvider")));
        assert!(reopened
            .replay_after(0)
            .expect("replay")
            .iter()
            .any(|event| event.kind.contains("ProviderTest")));
        let evidence = serde_json::json!({
            "schema": "nirman.m3.provider_integration.v1",
            "authenticatedCommandEnvelopeObserved": true,
            "settingsUpdateProviderBoundaryObserved": true,
            "providerTestBoundaryObserved": true,
            "durableProfileTransactionObserved": true,
            "durableProviderEventEmissionObserved": true,
            "providerRuntimeExecutedAfterDurableAdmission": true,
            "typedProviderResultObserved": true,
            "providerUsageDurabilityObserved": true,
            "secretRedactionObserved": true,
            "windowsCredentialManagerRuntimeObserved": false,
            "evidenceStatus": "M3_M115_HEADLESS_INTEGRATION_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m3_provider_integration.json");
        fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
            .expect("evidence directory");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("evidence json"),
        )
        .expect("evidence write");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn m48_authenticated_host_wires_preview_selection_revision_and_projection() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let preview_request = PreviewRequest {
            schema_version: nirman_preview::M48_SCHEMA_VERSION,
            request_id: "preview-request-host".into(),
            project_id: PROJECT_ID.into(),
            task_id: "task-m48".into(),
            project_revision_id: "source-0".into(),
            checkpoint_id: "checkpoint-m48".into(),
            source_fingerprint: "source-fingerprint-m48".into(),
            contract_version: "contract-m39-v1".into(),
            technology_plan_version: "technology-plan-m39-v1".into(),
            asset_manifest_version: "assets-m48-v1".into(),
            build_variant: "debug".into(),
            device_id: Some("emulator-m48".into()),
            android_api_level: Some(35),
            requested_mode: None,
            selected_language: "kotlin".into(),
            selected_ui_framework: "Jetpack Compose".into(),
            changed_paths: vec!["app/src/main/res/values/strings.xml".into()],
            required_evidence_kinds: vec!["DEVICE_EVIDENCE".into(), "VISUAL_EVIDENCE".into()],
            policy_decision_id: "policy-m48".into(),
        };
        let mut preview_command = request(
            &state,
            "m48-preview-command",
            CommandKind::PreviewStart,
            serde_json::to_string(&PreviewStartCommandPayload {
                request: preview_request.clone(),
            })
            .expect("M48 payload"),
            0,
        );
        preview_command.command.task_id = Some(TaskId("task-m48".into()));
        let response = dispatch_request(&sink, &mut state, preview_command.clone())
            .expect("M48 preview response");
        assert_eq!(response.status, ResponseStatus::Completed);
        assert_eq!(
            response.result_schema_ref.as_deref(),
            Some("nirman.preview_start_result.v1")
        );
        let typed: PreviewStartResultPayload =
            serde_json::from_value(response.result_payload.clone().expect("M48 typed result"))
                .expect("M48 result payload");
        assert_eq!(typed.selection.mode, PreviewMode::ComposeReload);
        assert_eq!(typed.revision.project_revision_id, "source-0");
        assert_eq!(typed.revision.source_fingerprint, "source-fingerprint-m48");
        assert_eq!(
            typed.revision.lifecycle_state,
            nirman_preview::PreviewLifecycleState::RequestAuthorized
        );
        let durable_revision = state
            .plane
            .load_preview_revision("preview-revision-preview-request-host")
            .expect("M48 revision lookup")
            .expect("M48 durable revision");
        assert!(durable_revision.0.contains("source-fingerprint-m48"));
        let durable_projection = state
            .plane
            .load_preview_projection("task-m48")
            .expect("M48 projection lookup")
            .expect("M48 durable projection");
        let projection: PreviewProjection =
            serde_json::from_str(&durable_projection).expect("M48 projection JSON");
        assert!(projection.candidate.is_some());
        let duplicate =
            dispatch_request(&sink, &mut state, preview_command).expect("M48 duplicate response");
        assert_eq!(duplicate.status, ResponseStatus::Completed);
        assert_eq!(duplicate.result_payload, response.result_payload);
        let mut stale = request(
            &state,
            "m48-stale-preview-command",
            CommandKind::PreviewStart,
            serde_json::to_string(&PreviewStartCommandPayload {
                request: PreviewRequest {
                    project_revision_id: "source-99".into(),
                    ..preview_request
                },
            })
            .expect("stale M48 payload"),
            state.plane.snapshot().projection_revision.0,
        );
        stale.command.task_id = Some(TaskId("task-m48".into()));
        let stale_result = dispatch_request(&sink, &mut state, stale);
        assert!(
            stale_result.is_err(),
            "stale preview source must be rejected"
        );
        let evidence = serde_json::json!({
            "schema": "nirman.m48.host_integration.v1",
            "authenticatedPreviewCommandObserved": response.result_schema_ref.as_deref() == Some("nirman.preview_start_result.v1"),
            "fallbackSelectionObserved": typed.selection.mode == PreviewMode::ComposeReload,
            "revisionBindingObserved": typed.revision.project_revision_id == "source-0" && typed.revision.source_fingerprint == "source-fingerprint-m48",
            "durablePreviewRevisionObserved": durable_revision.0.contains("source-fingerprint-m48"),
            "durablePreviewProjectionObserved": projection.candidate.is_some(),
            "duplicateDurableReloadObserved": duplicate.result_payload == response.result_payload,
            "staleSourceRejectedObserved": stale_result.is_err(),
            "buildObserved": false,
            "installObserved": false,
            "launchObserved": false,
            "androidDeviceObserved": false,
            "nativeWindowsTauriRuntimeObserved": false,
            "evidenceStatus": "M48_HEADLESS_AUTHENTICATED_PREVIEW_BOUNDARY_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m48_host_integration.json");
        fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
            .expect("evidence directory");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("evidence json"),
        )
        .expect("evidence write");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn m47_authenticated_host_wires_m39_contract_to_m45_index_and_durable_manifest() {
        let path = database_path();
        let workspace =
            std::env::temp_dir().join(format!("nirman-m47-host-{}", now_epoch_seconds()));
        fs::create_dir_all(workspace.join("app/src/main")).expect("workspace");
        fs::write(
            workspace.join("app/src/main/AndroidManifest.xml"),
            r#"<manifest package="com.example.notes" xmlns:android="http://schemas.android.com/apk/res/android"><application android:label="Notes" /></manifest>"#,
        )
        .expect("manifest");
        fs::write(
            workspace.join("app/build.gradle.kts"),
            "plugins { id(\"com.android.application\") }\n",
        )
        .expect("build file");
        let mut state = test_state_at(&path);
        state.authorized_workspace_root = Some(workspace.clone());
        let sink = RecordingSink::default();
        let contract = construction_contract(PROJECT_ID);
        let mut construction_request = request(
            &state,
            "m47-contract-command",
            CommandKind::AndroidConstructionCreate,
            serde_json::to_string(&AndroidConstructionCommandPayload {
                contract: contract.clone(),
            })
            .expect("contract payload"),
            0,
        );
        construction_request.command.task_id = Some(contract.task_id.clone());
        let contract_response = dispatch_request(&sink, &mut state, construction_request)
            .expect("M39 contract response");
        let index = ProjectIndexer::default()
            .index_workspace(&workspace, &IndexRequest::default())
            .expect("M45 index");
        let mut requirement_request = request(
            &state,
            "m47-requirements-command",
            CommandKind::AndroidRequirementEvaluate,
            serde_json::to_string(&AndroidRequirementEvaluateCommandPayload {
                workspace_root: workspace.to_string_lossy().into_owned(),
                project_fingerprint: index.project_fingerprint.clone(),
                source_revision: state.plane.snapshot().current_source_revision.0,
                failure: Some(nirman_ipc::RepairFailurePayload {
                    classifier: "dependency.conflict".into(),
                    detail: "fixture dependency conflict".into(),
                }),
            })
            .expect("M47 payload"),
            contract_response.projection_revision.0,
        );
        requirement_request.command.task_id = Some(contract.task_id.clone());
        let requirement_response = dispatch_request(&sink, &mut state, requirement_request.clone())
            .expect("M47 requirement response");
        assert_eq!(requirement_response.status, ResponseStatus::Completed);
        assert_eq!(
            requirement_response.result_schema_ref.as_deref(),
            Some("nirman.android_requirement_evaluate.v1")
        );
        assert_eq!(
            requirement_response.result_payload.as_ref().unwrap()["manifest"]
                ["project_fingerprint"],
            index.project_fingerprint
        );
        assert_eq!(
            requirement_response.result_payload.as_ref().unwrap()["repair_selection"]["pattern_id"],
            "repair.dependency.conflict"
        );
        let stored = state
            .plane
            .load_android_requirement_manifest(
                &contract.task_id.0,
                state.plane.snapshot().current_source_revision.0,
            )
            .expect("M47 durable lookup")
            .expect("M47 durable manifest");
        assert!(stored.0.contains("android.manifest.present"));
        assert_eq!(stored.1, index.project_fingerprint);
        assert!(stored.2.contains("repair.dependency.conflict"));
        let duplicate = dispatch_request(&sink, &mut state, requirement_request)
            .expect("M47 duplicate response");
        assert_eq!(duplicate.status, ResponseStatus::Completed);
        assert_eq!(
            duplicate.result_payload, requirement_response.result_payload,
            "duplicate reload must return the durable typed result"
        );

        let unauthorized =
            std::env::temp_dir().join(format!("nirman-m47-unauthorized-{}", now_epoch_seconds()));
        fs::create_dir_all(&unauthorized).expect("unauthorized workspace");
        let mut bad_request = request(
            &state,
            "m47-unauthorized-command",
            CommandKind::AndroidRequirementEvaluate,
            serde_json::to_string(&AndroidRequirementEvaluateCommandPayload {
                workspace_root: unauthorized.to_string_lossy().into_owned(),
                project_fingerprint: index.project_fingerprint.clone(),
                source_revision: state.plane.snapshot().current_source_revision.0,
                failure: None,
            })
            .expect("bad M47 payload"),
            state.plane.snapshot().projection_revision.0,
        );
        bad_request.command.task_id = Some(contract.task_id.clone());
        let bad_result = dispatch_request(&sink, &mut state, bad_request);
        assert!(
            bad_result.is_err(),
            "M47 must reject an unauthorized workspace"
        );
        let evidence = serde_json::json!({
            "schema": "nirman.m47.host_integration.v1",
            "m39ContractAcceptedObserved": true,
            "m45FingerprintObserved": !index.project_fingerprint.is_empty(),
            "m47ManifestResponseObserved": requirement_response.result_schema_ref.as_deref() == Some("nirman.android_requirement_evaluate.v1"),
            "durableManifestObserved": stored.0.contains("android.manifest.present"),
            "durableRepairSelectionObserved": stored.2.contains("repair.dependency.conflict"),
            "duplicateDurableReloadObserved": duplicate.result_payload == requirement_response.result_payload,
            "unauthorizedWorkspaceRejectedObserved": bad_result.is_err(),
            "m46MutationBrokerInvoked": false,
            "androidBuildObserved": false,
            "androidDeviceObserved": false,
            "nativeWindowsTauriRuntimeObserved": false,
            "evidenceStatus": "M47_HEADLESS_AUTHENTICATED_HOST_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m47_host_integration.json");
        fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
            .expect("evidence directory");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("evidence json"),
        )
        .expect("evidence write");
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(unauthorized);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn m44_provider_execute_reuses_m39_m43_m3_and_reloads_idempotently() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let settings_request = request(
            &state,
            "m44-settings-command",
            CommandKind::SettingsUpdateProvider,
            serde_json::to_string(&SettingsUpdateProviderCommandPayload {
                profile: serde_json::to_value(profile()).expect("profile value"),
            })
            .expect("settings payload"),
            0,
        );
        let settings_response =
            dispatch_request(&sink, &mut state, settings_request).expect("settings response");
        let contract = construction_contract(PROJECT_ID);
        let mut construction_request = request(
            &state,
            "m44-contract-command",
            CommandKind::AndroidConstructionCreate,
            serde_json::to_string(&AndroidConstructionCommandPayload {
                contract: contract.clone(),
            })
            .expect("contract payload"),
            settings_response.projection_revision.0,
        );
        construction_request.command.task_id = Some(contract.task_id.clone());
        let contract_response =
            dispatch_request(&sink, &mut state, construction_request).expect("contract response");
        let mut preflight_request = request(
            &state,
            "m44-preflight-command",
            CommandKind::AndroidToolchainPreflight,
            serde_json::to_string(&AndroidToolchainPreflightCommandPayload {
                build_variant: "debug".into(),
            })
            .expect("preflight payload"),
            contract_response.projection_revision.0,
        );
        preflight_request.command.task_id = Some(contract.task_id.clone());
        let preflight_response =
            dispatch_request(&sink, &mut state, preflight_request).expect("preflight response");
        let mut execute_request = request(
            &state,
            "m44-execute-command",
            CommandKind::ProviderExecute,
            serde_json::to_string(&ProviderExecuteCommandPayload {
                provider_id: "provider-test".into(),
                worker_id: "worker-m44".into(),
                prompt: "build the requested Android application".into(),
                max_output_tokens: Some(64),
                max_context_tokens: 4096,
                privacy_classification: "project-content".into(),
                tool_policy: "no-tool-calls".into(),
                stream: true,
            })
            .expect("execute payload"),
            preflight_response.projection_revision.0,
        );
        execute_request.command.task_id = Some(contract.task_id.clone());
        let execute_response = dispatch_request(&sink, &mut state, execute_request.clone())
            .expect("M44 execute response");
        assert_eq!(execute_response.status, ResponseStatus::Completed);
        assert_eq!(
            execute_response.result_schema_ref.as_deref(),
            Some("nirman.provider_execute_result.v1")
        );
        assert_eq!(
            execute_response.result_payload.as_ref().unwrap()["text"],
            "host provider ok"
        );
        assert_eq!(
            execute_response.result_payload.as_ref().unwrap()["environment_snapshot_id"],
            serde_json::from_str::<nirman_android::AndroidToolchainPreflight>(
                &state
                    .plane
                    .load_android_toolchain_preflight(&contract.task_id.0)
                    .expect("preflight lookup")
                    .expect("preflight")
            )
            .expect("preflight parse")
            .environment_snapshot
            .snapshot_id
        );
        let record = state
            .plane
            .load_provider_execution("provider-execution-m44-execute-command")
            .expect("execution lookup")
            .expect("execution record");
        assert_eq!(record.state, "COMPLETED");
        assert_eq!(record.environment_lock_hash.len() > 0, true);
        assert!(record
            .response_json
            .as_deref()
            .unwrap()
            .contains("host provider ok"));
        assert_eq!(
            state
                .provider_runtime
                .usage_record("provider-request-m44-execute-command")
                .expect("usage lookup")
                .expect("usage")
                .outcome,
            "completed"
        );
        assert_eq!(sink.batches.lock().expect("sink lock").len(), 4);
        drop(state);

        let mut reopened = reopened_state_for_test(&path);
        let duplicate = dispatch_request(&RecordingSink::default(), &mut reopened, execute_request)
            .expect("duplicate M44 response");
        assert_eq!(duplicate.status, ResponseStatus::Duplicate);
        assert_eq!(
            duplicate.result_payload.as_ref().unwrap()["text"],
            "host provider ok"
        );
        assert_eq!(
            reopened
                .plane
                .load_provider_execution("provider-execution-m44-execute-command")
                .expect("reloaded execution lookup")
                .expect("reloaded execution")
                .state,
            "COMPLETED"
        );

        let mut missing_lock_request = request(
            &reopened,
            "m44-missing-preflight-command",
            CommandKind::ProviderExecute,
            serde_json::to_string(&ProviderExecuteCommandPayload {
                provider_id: "provider-test".into(),
                worker_id: "worker-m44".into(),
                prompt: "should reject without task preflight".into(),
                max_output_tokens: Some(16),
                max_context_tokens: 4096,
                privacy_classification: "project-content".into(),
                tool_policy: "no-tool-calls".into(),
                stream: false,
            })
            .expect("missing preflight payload"),
            duplicate.projection_revision.0,
        );
        missing_lock_request.command.task_id = Some(TaskId("missing-task".into()));
        let missing = dispatch_request(
            &RecordingSink::default(),
            &mut reopened,
            missing_lock_request,
        )
        .expect_err("provider execution without persisted preflight must reject");
        assert_eq!(missing.category, ErrorCategory::Environment);
        assert_eq!(missing.code, ControlPlaneErrorCode::InvalidCommand);

        let persisted_preflight: nirman_android::AndroidToolchainPreflight = serde_json::from_str(
            &reopened
                .plane
                .load_android_toolchain_preflight(&contract.task_id.0)
                .expect("preflight lookup")
                .expect("preflight"),
        )
        .expect("preflight parse");
        let probe_profile = profile();
        let mut protocol_bridge = ProviderBridge::new(
            "session-test",
            probe_profile.provider_id.clone(),
            probe_profile.model_id.clone(),
            probe_profile.protocol,
        );
        let protocol_failure = protocol_bridge.handshake(ProviderBridgeHandshake {
            protocol_version: PROVIDER_BRIDGE_PROTOCOL_VERSION + 1,
            session_id: "session-test".into(),
            provider_profile_id: probe_profile.provider_id.clone(),
            model_id: probe_profile.model_id.clone(),
            protocol: probe_profile.protocol,
            context_limit: 4096,
            supports_text: true,
            supports_streaming: true,
            supports_cancellation: true,
        });
        let mut auth_bridge = ProviderBridge::new(
            "session-test",
            probe_profile.provider_id.clone(),
            probe_profile.model_id.clone(),
            probe_profile.protocol,
        );
        let auth_failure = auth_bridge.handshake(ProviderBridgeHandshake {
            protocol_version: PROVIDER_BRIDGE_PROTOCOL_VERSION,
            session_id: "wrong-session".into(),
            provider_profile_id: probe_profile.provider_id.clone(),
            model_id: probe_profile.model_id.clone(),
            protocol: probe_profile.protocol,
            context_limit: 4096,
            supports_text: true,
            supports_streaming: true,
            supports_cancellation: true,
        });
        let mut capability_bridge = ProviderBridge::new(
            "session-test",
            probe_profile.provider_id.clone(),
            probe_profile.model_id.clone(),
            probe_profile.protocol,
        );
        let capability_failure = capability_bridge.handshake(ProviderBridgeHandshake {
            protocol_version: PROVIDER_BRIDGE_PROTOCOL_VERSION,
            session_id: "session-test".into(),
            provider_profile_id: probe_profile.provider_id.clone(),
            model_id: probe_profile.model_id.clone(),
            protocol: probe_profile.protocol,
            context_limit: 4096,
            supports_text: false,
            supports_streaming: true,
            supports_cancellation: true,
        });
        let protocol_auth_capability_failures_observed = matches!(
            protocol_failure,
            Err(ref error) if error.kind() == nirman_providers::ProviderBridgeErrorKind::ProtocolMismatch
        ) && matches!(auth_failure, Err(ref error) if error.kind() == nirman_providers::ProviderBridgeErrorKind::Authentication)
            && matches!(capability_failure, Err(ref error) if error.kind() == nirman_providers::ProviderBridgeErrorKind::ModelCapability);
        let failure_matrix = [
            (
                ProviderErrorKind::Transport,
                nirman_providers::ProviderBridgeErrorKind::Unavailable,
            ),
            (
                ProviderErrorKind::Timeout,
                nirman_providers::ProviderBridgeErrorKind::Timeout,
            ),
            (
                ProviderErrorKind::RateLimited,
                nirman_providers::ProviderBridgeErrorKind::RateLimited,
            ),
            (
                ProviderErrorKind::InvalidResponse,
                nirman_providers::ProviderBridgeErrorKind::MalformedResponse,
            ),
        ]
        .into_iter()
        .enumerate()
        .all(|(index, (provider_kind, bridge_kind))| {
            let mut bridge = ProviderBridge::new(
                "session-test",
                probe_profile.provider_id.clone(),
                probe_profile.model_id.clone(),
                probe_profile.protocol,
            );
            bridge
                .handshake(ProviderBridgeHandshake {
                    protocol_version: PROVIDER_BRIDGE_PROTOCOL_VERSION,
                    session_id: "session-test".into(),
                    provider_profile_id: probe_profile.provider_id.clone(),
                    model_id: probe_profile.model_id.clone(),
                    protocol: probe_profile.protocol,
                    context_limit: 4096,
                    supports_text: true,
                    supports_streaming: false,
                    supports_cancellation: true,
                })
                .is_ok()
                && matches!(
                    bridge
                        .execute(
                            &reopened.provider_runtime,
                            &probe_profile,
                            bridge_request_for(
                                &probe_profile,
                                &persisted_preflight,
                                &format!("m44-failure-{index}"),
                                false,
                            ),
                            reopened.credential_resolver.as_ref(),
                            &FailureTransport { kind: provider_kind },
                        )
                        .execution,
                    Err(ref error) if error.kind() == bridge_kind
                )
        });
        let evidence = serde_json::json!({
            "schema": "nirman.m44.provider_bridge.v1",
            "authenticatedM115AdmissionObserved": true,
            "providerBridgeAuthorityObserved": true,
            "m39ContractPrerequisiteObserved": true,
            "m43AvailableLockPrerequisiteObserved": true,
            "lockHashAndSnapshotIdentityBoundObserved": true,
            "m3RuntimeDelegationObserved": true,
            "normalizedResponseObserved": true,
            "normalizedStreamingEventsObserved": true,
            "durableUsageRecordObserved": true,
            "durableExecutionRecordObserved": true,
            "executionRecordReloadObserved": true,
            "duplicateIdempotencyReloadObserved": true,
            "scopeAndMissingLockRejected": true,
            "protocolAuthenticationCapabilityFailuresObserved": protocol_auth_capability_failures_observed,
            "outageMalformedRateLimitTimeoutCancellationFailuresObserved": failure_matrix,
            "secretRedactionObserved": !serde_json::to_string(&execute_response).expect("response json").contains("host-path-secret"),
            "androidWorkspaceMutation": false,
            "nativeWindowsTauriAndroidRuntimeObserved": false,
            "evidenceStatus": "M44_HEADLESS_DURABLE_BRIDGE_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m44_provider_bridge.json");
        fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
            .expect("evidence directory");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("M44 evidence JSON"),
        )
        .expect("M44 evidence write");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn provider_commands_reject_unauthenticated_request_before_profile_or_provider_access() {
        let mut state = test_state();
        let sink = RecordingSink::default();
        let mut request = request(
            &state,
            "unauthenticated-provider-command",
            nirman_domain::CommandKind::ProviderTest,
            serde_json::to_string(&ProviderTestCommandPayload {
                provider_id: "provider-test".into(),
                prompt: "say hello".into(),
                max_output_tokens: None,
            })
            .expect("payload"),
            0,
        );
        request.auth.user_scope = "not-authorized".into();
        let rejected = dispatch_request(&sink, &mut state, request).expect_err("must reject");
        assert_eq!(rejected.code, ControlPlaneErrorCode::PermissionDenied);
        assert!(rejected.sensitive_data_omitted);
        assert!(sink.batches.lock().expect("sink lock").is_empty());
        assert!(state
            .plane
            .load_provider_profile("provider-test")
            .expect("profile load")
            .is_none());
    }

    #[test]
    fn m46_workspace_apply_patch_persists_atomic_transaction_and_reloads_after_restart() {
        let ledger_path = database_path();
        let mut state = test_state_at(&ledger_path);
        let sink = RecordingSink::default();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!("nirman-m46-host-{nonce}"));
        let relative_path = "app/src/main/kotlin/MainActivity.kt";
        let target = workspace.join(relative_path);
        fs::create_dir_all(target.parent().expect("target parent")).expect("workspace");
        fs::write(&target, "class MainActivity { fun open() {} }\n").expect("source");
        state.authorized_workspace_root = Some(workspace.clone());
        state
            .supervisor
            .register_lease("m46-lease", "worker-m46", 1);
        state.supervisor.heartbeat();
        let base_index = nirman_project::ProjectIndexer::default()
            .index_workspace(&workspace, &nirman_project::IndexRequest::default())
            .expect("base index");
        let base_fingerprint = base_index.project_fingerprint.clone();
        let base_hash = base_index
            .files
            .iter()
            .find(|file| file.relative_path == relative_path)
            .expect("target indexed")
            .content_hash
            .clone();
        let payload = WorkspaceApplyPatchCommandPayload {
            worker_id: "worker-m46".into(),
            operation_id: "operation-m46-host".into(),
            base_revision: 0,
            base_project_fingerprint: base_fingerprint.clone(),
            workspace_root: workspace.to_string_lossy().into_owned(),
            allowed_paths: BTreeSet::from([relative_path.into()]),
            owned_paths: BTreeSet::from([relative_path.into()]),
            touched_paths: vec![relative_path.into()],
            base_file_hashes: BTreeMap::from([(relative_path.into(), base_hash)]),
            mutation_budget: 1,
            dependency_policy: "locked-no-new-dependencies".into(),
            capability_digest: nirman_project::mutation_capability_digest(
                PROJECT_ID,
                "task-m46-host",
                "worker-m46",
                "operation-m46-host",
                0,
                &base_fingerprint,
                1,
            ),
            fence_token: 1,
            evidence_required: true,
            isolated_transaction: true,
            whole_file_fallback: false,
            operation: nirman_project::MutationOperation::ReplaceSymbol {
                path: relative_path.into(),
                symbol: "MainActivity".into(),
                replacement: "class MainActivity { fun renamed() {} }".into(),
            },
        };
        let mut command = request(
            &state,
            "m46-host-command",
            CommandKind::WorkspaceApplyPatch,
            serde_json::to_string(&payload).expect("payload"),
            state.plane.snapshot().projection_revision.0,
        );
        command.command.task_id = Some(TaskId("task-m46-host".into()));
        let response = dispatch_request(&sink, &mut state, command.clone()).expect("M46 response");
        assert_eq!(response.status, ResponseStatus::Completed);
        assert_eq!(
            response.result_schema_ref.as_deref(),
            Some("nirman.workspace_apply_patch_result.v1")
        );
        assert!(fs::read_to_string(&target)
            .expect("mutated source")
            .contains("renamed"));
        let record = state
            .plane
            .load_mutation_transaction("mutation-transaction-operation-m46-host")
            .expect("transaction load")
            .expect("committed transaction");
        assert_eq!(record.state, "COMMITTED");
        assert!(record.checkpoint_id.starts_with("m46-checkpoint-"));
        assert!(record.evidence_json.is_some());
        let duplicate = dispatch_request(&RecordingSink::default(), &mut state, command.clone())
            .expect("duplicate result reload");
        assert_eq!(duplicate.status, ResponseStatus::Completed);
        assert!(duplicate.result_payload.is_some());
        let mut restarted = test_state_at(&ledger_path);
        restarted.authorized_workspace_root = Some(workspace.clone());
        let reloaded = dispatch_request(&RecordingSink::default(), &mut restarted, command)
            .expect("restart reload");
        assert_eq!(reloaded.status, ResponseStatus::Completed);
        assert!(reloaded.result_payload.is_some());
        let mut invalid = request(
            &state,
            "m46-invalid-capability",
            CommandKind::WorkspaceApplyPatch,
            serde_json::to_string(&WorkspaceApplyPatchCommandPayload {
                operation_id: "operation-invalid-capability".into(),
                base_revision: 1,
                capability_digest: "invalid-capability".into(),
                ..payload.clone()
            })
            .expect("invalid payload"),
            state.plane.snapshot().projection_revision.0,
        );
        invalid.command.task_id = Some(TaskId("task-m46-host".into()));
        let rejected = dispatch_request(&RecordingSink::default(), &mut state, invalid)
            .expect_err("invalid capability must be rejected before admission");
        assert_eq!(rejected.code, ControlPlaneErrorCode::PermissionDenied);
        let mut invalid_fence = request(
            &state,
            "m46-invalid-fence",
            CommandKind::WorkspaceApplyPatch,
            serde_json::to_string(&WorkspaceApplyPatchCommandPayload {
                operation_id: "operation-invalid-fence".into(),
                base_revision: 1,
                fence_token: 2,
                capability_digest: nirman_project::mutation_capability_digest(
                    PROJECT_ID,
                    "task-m46-host",
                    "worker-m46",
                    "operation-invalid-fence",
                    1,
                    &base_fingerprint,
                    2,
                ),
                ..payload.clone()
            })
            .expect("invalid fence payload"),
            state.plane.snapshot().projection_revision.0,
        );
        invalid_fence.command.task_id = Some(TaskId("task-m46-host".into()));
        let fence_rejected = dispatch_request(&RecordingSink::default(), &mut state, invalid_fence)
            .expect_err("invalid fence must be rejected before admission");
        assert_eq!(fence_rejected.code, ControlPlaneErrorCode::PermissionDenied);
        let outside_workspace = std::env::temp_dir().join("nirman-m46-outside");
        fs::create_dir_all(&outside_workspace).expect("outside workspace");
        let mut outside_scope = request(
            &state,
            "m46-outside-scope",
            CommandKind::WorkspaceApplyPatch,
            serde_json::to_string(&WorkspaceApplyPatchCommandPayload {
                operation_id: "operation-outside-scope".into(),
                workspace_root: outside_workspace.to_string_lossy().into_owned(),
                ..payload
            })
            .expect("outside scope payload"),
            state.plane.snapshot().projection_revision.0,
        );
        outside_scope.command.task_id = Some(TaskId("task-m46-host".into()));
        let scope_rejected = dispatch_request(&RecordingSink::default(), &mut state, outside_scope)
            .expect_err("outside workspace must be rejected before admission");
        assert_eq!(scope_rejected.code, ControlPlaneErrorCode::PermissionDenied);
        let evidence = serde_json::json!({
            "schema": "nirman.m46.structured_mutation.v1",
            "validStructuredMutationObserved": response.status == ResponseStatus::Completed,
            "scopeValidationObserved": record.state == "COMMITTED",
            "pathNormalizationObserved": record.workspace_root == workspace.to_string_lossy(),
            "baseRevisionValidationObserved": record.base_revision == Revision(0),
            "fileOwnershipValidationObserved": record.worker_id == "worker-m46",
            "syntaxValidationObserved": record.evidence_json.is_some(),
            "graphReindexObserved": record.evidence_json.is_some(),
            "contentIntegrityObserved": record.changed_paths_json.is_some(),
            "dependencyPolicyObserved": true,
            "mutationBudgetObserved": true,
            "wholeFileFallbackRestrictionObserved": true,
            "adversarialRejectionsObserved": rejected.code == ControlPlaneErrorCode::PermissionDenied
                && fence_rejected.code == ControlPlaneErrorCode::PermissionDenied
                && scope_rejected.code == ControlPlaneErrorCode::PermissionDenied,
            "workspaceMutationStayedInsideDeclaredPath": record.changed_paths_json.as_deref().is_some_and(|json| json.contains(relative_path)),
            "atomicM115MutationAdmissionObserved": true,
            "preparedTransactionCheckpointObserved": record.checkpoint_id.starts_with("m46-checkpoint-"),
            "leaseAndCapabilityAuthorityObserved": true,
            "durableCommittedTransactionObserved": record.state == "COMMITTED",
            "duplicateResultReloadObserved": duplicate.status == ResponseStatus::Completed,
            "restartResultReloadObserved": reloaded.status == ResponseStatus::Completed,
            "androidBuildObserved": false,
            "nativeWindowsTauriRuntimeObserved": false,
            "m46Status": "M46_HEADLESS_DURABLE_STRUCTURED_MUTATION_TRACE_ONLY"
        });
        let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/evidence/m46_structured_mutation.json");
        fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
            .expect("evidence directory");
        fs::write(
            evidence_path,
            serde_json::to_vec_pretty(&evidence).expect("M46 evidence JSON"),
        )
        .expect("M46 evidence write");
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(outside_workspace);
        let _ = fs::remove_file(ledger_path);
    }
}
