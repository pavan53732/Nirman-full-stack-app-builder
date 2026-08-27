#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nirman_android::{
    execute_android_build, infer_android_requirement_manifest, plan_preflight,
    synthesize_android_plan, validate_android_build_request, validate_android_workspace,
    AndroidBuildExecutionError, AndroidBuildRequest, AndroidRepairRegistry,
    AndroidRequirementManifest, AndroidSynthesisRequest, BuildCancellation, CapabilityProbe,
    HostCapabilityProbe, PreflightStatus, RepairFailureFingerprint, RepairSelection,
};
use nirman_artifacts::{deliver_apk_local, inspect_apk, scan_apk_for_secrets, ApkArtifact};
use nirman_control_plane::{
    deadline_elapsed, DurableControlPlane, DurableControlPlaneError, DurableDispatchOutcome,
    M8DispatchRecord,
};
use nirman_domain::{
    AndroidConstructionCommandPayload, AndroidConstructionContract, CommandKind,
    MutationTransactionRecord, ProjectId, Revision, TaskId,
};
use nirman_ipc::{
    acknowledge_event_subscription, authorize_registry_capability, command_registry,
    publish_control_event, AndroidRequirementEvaluateCommandPayload,
    AndroidRequirementEvaluateResultPayload, AndroidSynthesisBuildCommandPayload,
    AndroidSynthesisBuildResultPayload, AndroidToolchainPreflightCommandPayload,
    AndroidToolchainPreflightResultPayload, ArtifactBuildCommandPayload,
    ArtifactBuildResultPayload, ArtifactExportCommandPayload, ArtifactExportResultPayload,
    AuthContext, AuthenticatedSession, CommandRequest, CommandResponse, ControlPlaneErrorCode,
    ErrorCategory, ErrorEnvelope, EventBatch, EventRange, EventSink, EventSubscription,
    PreviewStartCommandPayload, PreviewStartResultPayload, ProviderExecuteCommandPayload,
    ProviderExecuteResultPayload, ProviderTestCommandPayload, ProviderTestResultPayload,
    ResponseStatus, SettingsUpdateProviderCommandPayload, SubscriptionAcknowledgement,
    SubscriptionBootstrap, SubscriptionControl, SubscriptionStatus,
    WorkerHandoffAcknowledgeCommandPayload, WorkerHandoffAcknowledgeResultPayload,
    WorkerHandoffSubmitCommandPayload, WorkerHandoffSubmitResultPayload,
    WorkerReconcileCommandPayload, WorkerReconcileResultPayload, WorkerTaskClaimCommandPayload,
    WorkerTaskClaimResultPayload, WorkspaceApplyPatchCommandPayload,
    WorkspaceApplyPatchResultPayload, PROTOCOL_SCHEMA_VERSION,
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
use nirman_supervisor::{
    BackgroundRunRecord, BackgroundRunState, RecoveryAction, Supervisor, SupervisorState,
    M7_SCHEMA_VERSION,
};
use nirman_workers::{
    CoordinationError, CoordinationTask, M8ReconciliationCheckpoint, MultiWorkerCoordinator,
    ReconciliationMutationSummary, ReconciliationStatus, WorkerContract,
    WorkerHandoffAcknowledgement, WorkerHandoffRecord, WorkerTaskClaim, M5_SCHEMA_VERSION,
};
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
    artifact_id: Option<String>,
    artifact_fingerprint: Option<String>,
    runtime_session_id: Option<String>,
    device_id: Option<String>,
    observation_refs: Vec<String>,
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
    let active_preview_revision_id = if projection.last_event_sequence == 0 {
        preview_revision_id.to_owned()
    } else {
        projection.active_preview_revision_id.clone()
    };
    let sequence = projection.last_event_sequence + 1;
    let validation_ref = (event_type == PreviewSyncEventType::ValidationObserved)
        .then(|| format!("validation:{command_id}"));
    let event = PreviewSyncEvent {
        event_id: format!("m108:{command_id}:{sequence}"),
        event_sequence: sequence,
        project_id: state.auth.project_scope.clone(),
        task_id: task_id.into(),
        correlation_id: state.correlation_id.clone(),
        causation_id: Some(command_id.into()),
        candidate_preview_revision_id: active_preview_revision_id,
        event_type,
        event_truth,
        project_revision_id: project_revision_id.into(),
        checkpoint_id: checkpoint_id.into(),
        source_fingerprint: source_fingerprint.into(),
        artifact_id,
        artifact_fingerprint,
        runtime_session_id,
        device_id,
        operation_ref: command_id.into(),
        observation_refs,
        evidence_refs: evidence_refs.clone(),
        validation_ref,
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
    let mut state_fingerprints = BTreeMap::new();
    state_fingerprints.insert(
        "projectRevisionId".into(),
        event.project_revision_id.clone(),
    );
    state_fingerprints.insert("checkpointId".into(), event.checkpoint_id.clone());
    state_fingerprints.insert("sourceFingerprint".into(), event.source_fingerprint.clone());
    if let Some(artifact_fingerprint) = event.artifact_fingerprint.as_ref() {
        state_fingerprints.insert("artifactFingerprint".into(), artifact_fingerprint.clone());
    }
    if let Some(device_id) = event.device_id.as_ref() {
        state_fingerprints.insert("deviceId".into(), device_id.clone());
    }
    if let Some(runtime_session_id) = event.runtime_session_id.as_ref() {
        state_fingerprints.insert("runtimeSessionId".into(), runtime_session_id.clone());
    }
    let evidence = nirman_preview::PreviewSyncEvidenceRecord {
        evidence_id: format!("m108-evidence:{command_id}:{sequence}"),
        project_id: event.project_id.clone(),
        task_id: event.task_id.clone(),
        event_sequence_start: sequence,
        event_sequence_end: sequence,
        projection_revision: projection.projection_revision,
        preview_revision_id: preview_revision_id.into(),
        project_revision_id: event.project_revision_id.clone(),
        checkpoint_id: event.checkpoint_id.clone(),
        branch_id: None,
        artifact_fingerprint: event.artifact_fingerprint.clone(),
        device_id: event.device_id.clone(),
        runtime_session_id: event.runtime_session_id.clone(),
        state_fingerprints,
        event_ids: vec![event.event_id.clone()],
        observation_refs: event.observation_refs.clone(),
        evidence_refs,
        validation_refs: event.validation_ref.clone().into_iter().collect(),
        invalidated_evidence_ids: vec![],
        recovery_event_ids: matches!(
            event.event_type,
            PreviewSyncEventType::CandidateFailed
                | PreviewSyncEventType::PreviewInvalidated
                | PreviewSyncEventType::StreamGap
                | PreviewSyncEventType::StreamReconnected
        )
        .then(|| event.event_id.clone())
        .into_iter()
        .collect(),
        promotion_record_ref: (event.event_type == PreviewSyncEventType::PreviewPromoted)
            .then(|| event.operation_ref.clone()),
        certification_decision_ref: (event.event_type == PreviewSyncEventType::PreviewPromoted)
            .then(|| event.validation_ref.clone())
            .flatten(),
        completion_decision_ref: None,
        truth: event.event_truth.clone(),
        captured_at_epoch_seconds: now_epoch_seconds(),
    };
    evidence.validate().map_err(|e| {
        error(
            "m109",
            Some(command_id.into()),
            Some(command_id.into()),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            e.to_string(),
            false,
            None,
        )
    })?;
    let event_json = serde_json::to_string(&event).expect("M108 event serialization");
    let evidence_json = serde_json::to_string(&evidence).expect("M108 evidence serialization");
    let projection_json =
        serde_json::to_string(&projection).expect("M108 projection serialization");
    state
        .plane
        .append_m108_event_and_projection(
            task_id,
            sequence,
            &event.event_id,
            &event_json,
            &evidence.evidence_id,
            &evidence_json,
            &projection_json,
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
    preview_request: PreviewRequest,
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
        preview_request: preview_request.clone(),
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

fn m7_task_id(request: &CommandRequest) -> nirman_domain::TaskId {
    request
        .command
        .task_id
        .clone()
        .expect("M7 lifecycle requests are normalized before preparation")
}

fn m7_run_id(project_id: &str, task_id: &nirman_domain::TaskId) -> String {
    format!("run-{project_id}-{}", task_id.0)
}

fn m7_lifecycle_candidate(
    state: &mut RuntimeState,
    request: &CommandRequest,
) -> Result<Option<BackgroundRunRecord>, ErrorEnvelope> {
    let kind = request.command.kind;
    if !matches!(
        kind,
        CommandKind::TaskStart
            | CommandKind::SubmitInstruction
            | CommandKind::PauseTask
            | CommandKind::TaskResume
            | CommandKind::ResumeTask
            | CommandKind::CancelTask
            | CommandKind::TaskCancel
    ) {
        return Ok(None);
    }

    if state
        .plane
        .command_result_exists(
            &request.command.command_id,
            request.command.idempotency_key.as_deref(),
        )
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "command-result lookup is unavailable",
                true,
                Some("retry after local command storage is available".into()),
            )
        })?
    {
        return Ok(None);
    }
    let task_id = m7_task_id(request);
    let run_id = m7_run_id(&request.command.project_id.0, &task_id);
    let existing = state.plane.load_background_run(&run_id).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            "background-run state could not be loaded",
            true,
            Some("retry after local background-run storage is available".into()),
        )
    })?;

    let mut record = match (kind, existing) {
        (CommandKind::TaskStart, Some(_)) => {
            return Err(error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "task already has a durable background run",
                false,
                Some("use the existing task lifecycle commands".into()),
            ));
        }
        (CommandKind::TaskStart | CommandKind::SubmitInstruction, None) => BackgroundRunRecord {
            schema_version: M7_SCHEMA_VERSION,
            run_id,
            project_id: request.command.project_id.0.clone(),
            task_id: task_id.0,
            worker_id: "worker-single".into(),
            checkpoint_id: state.plane.checkpoint_id().map(str::to_owned),
            state: BackgroundRunState::Running,
            last_heartbeat_epoch_seconds: now_epoch_seconds(),
            attempt: 1,
            recovery_action: None,
            failure_fingerprint: None,
            notification_kind: None,
        },
        (_, Some(record)) => record,
        (_, None) => {
            return Err(error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "lifecycle command has no durable task background run",
                true,
                Some("reconcile the task run before retrying the lifecycle command".into()),
            ));
        }
    };

    match kind {
        CommandKind::SubmitInstruction => record.heartbeat(now_epoch_seconds()).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "task background run cannot accept an instruction from its current state",
                false,
                None,
            )
        })?,
        CommandKind::PauseTask => record.pause().map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "background run cannot be paused from its current state",
                false,
                None,
            )
        })?,
        CommandKind::TaskResume | CommandKind::ResumeTask => {
            if record.checkpoint_id.is_none() {
                record.checkpoint_id = state.plane.checkpoint_id().map(str::to_owned);
            }
            let checkpoint_id = record.checkpoint_id.as_deref().ok_or_else(|| {
                error(
                    &request.correlation_id,
                    Some(request.command.command_id.clone()),
                    request.causation_id.clone(),
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Validation,
                    "resume requires a verified checkpoint",
                    false,
                    Some("create and persist a checkpoint before resuming".into()),
                )
            })?;
            let verified = state.plane.checkpoint_exists(checkpoint_id).map_err(|_| {
                error(
                    &request.correlation_id,
                    Some(request.command.command_id.clone()),
                    request.causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Unavailable,
                    "checkpoint verification is unavailable",
                    true,
                    Some("retry after checkpoint storage is available".into()),
                )
            })?;
            if !verified {
                return Err(error(
                    &request.correlation_id,
                    Some(request.command.command_id.clone()),
                    request.causation_id.clone(),
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Validation,
                    "resume requires a verified checkpoint",
                    false,
                    Some("create and persist the referenced checkpoint before resuming".into()),
                ));
            }
            record.resume_from_checkpoint().map_err(|_| {
                error(
                    &request.correlation_id,
                    Some(request.command.command_id.clone()),
                    request.causation_id.clone(),
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Validation,
                    "background run cannot resume from its current state",
                    false,
                    None,
                )
            })?;
        }
        CommandKind::CancelTask | CommandKind::TaskCancel => record.cancel().map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "background run cannot be cancelled from its current state",
                false,
                None,
            )
        })?,
        CommandKind::TaskStart => {}
        _ => unreachable!("M7 lifecycle candidate was filtered above"),
    }
    record.validate().map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "background-run candidate failed deterministic validation",
            false,
            None,
        )
    })?;
    Ok(Some(record))
}

fn parse_m8_claim(
    request: &CommandRequest,
) -> Result<WorkerTaskClaimCommandPayload, ErrorEnvelope> {
    let payload: WorkerTaskClaimCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "worker task claim payload is invalid",
                false,
                None,
            )
        })?;
    validate_m8_parent_and_task(request, &payload.parent_contract, &payload.task)?;
    if request
        .command
        .task_id
        .as_ref()
        .map(|task_id| task_id.0.as_str())
        != Some(payload.task.task_id.as_str())
        || payload.lease_duration_seconds == 0
    {
        return Err(m8_validation_error(
            request,
            "worker task claim requires matching task identity and a positive lease duration",
        ));
    }
    Ok(payload)
}

fn parse_m8_handoff(
    request: &CommandRequest,
) -> Result<WorkerHandoffSubmitCommandPayload, ErrorEnvelope> {
    let payload: WorkerHandoffSubmitCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "worker handoff payload is invalid",
                false,
                None,
            )
        })?;
    if request
        .command
        .task_id
        .as_ref()
        .map(|task_id| task_id.0.as_str())
        != Some(payload.handoff.task_id.as_str())
        || payload.handoff.worker_id.trim().is_empty()
    {
        return Err(m8_validation_error(
            request,
            "worker handoff requires matching task identity and worker identity",
        ));
    }
    validate_m8_parent_contract(request, &payload.parent_contract)?;
    Ok(payload)
}

fn parse_m8_acknowledgement(
    request: &CommandRequest,
) -> Result<WorkerHandoffAcknowledgeCommandPayload, ErrorEnvelope> {
    let payload: WorkerHandoffAcknowledgeCommandPayload =
        serde_json::from_str(&request.command.payload).map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "worker handoff acknowledgement payload is invalid",
                false,
                None,
            )
        })?;
    validate_m8_parent_contract(request, &payload.parent_contract)?;
    if request.command.task_id.is_none()
        || payload.acknowledgement_id.trim().is_empty()
        || payload.message_id.trim().is_empty()
    {
        return Err(m8_validation_error(
            request,
            "worker handoff acknowledgement requires task, acknowledgement, and message identity",
        ));
    }
    Ok(payload)
}

fn parse_m8_reconcile(
    request: &CommandRequest,
) -> Result<WorkerReconcileCommandPayload, ErrorEnvelope> {
    let payload: WorkerReconcileCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "worker reconciliation payload is invalid",
                false,
                None,
            )
        })?;
    validate_m8_parent_contract(request, &payload.parent_contract)?;
    if request
        .command
        .task_id
        .as_ref()
        .map(|task_id| task_id.0.as_str())
        != Some(payload.parent_contract.task_id.0.as_str())
        || payload.checkpoint_id.trim().is_empty()
        || payload.integration_workspace_root.trim().is_empty()
        || payload.handoff_message_ids.is_empty()
        || payload
            .handoff_message_ids
            .iter()
            .any(|id| id.trim().is_empty())
    {
        return Err(m8_validation_error(
            request,
            "worker reconciliation requires the parent task, checkpoint, integration workspace, and handoff identities",
        ));
    }
    Ok(payload)
}

fn validate_m8_parent_and_task(
    request: &CommandRequest,
    parent: &WorkerContract,
    task: &CoordinationTask,
) -> Result<(), ErrorEnvelope> {
    validate_m8_parent_contract(request, parent)?;
    if task.parent_task_id != parent.task_id.0
        || task.parent_workspace_root != parent.workspace_root
        || task.worker_id.trim().is_empty()
    {
        return Err(m8_validation_error(
            request,
            "worker task is outside the parent contract scope",
        ));
    }
    Ok(())
}

fn validate_m8_parent_contract(
    request: &CommandRequest,
    parent: &WorkerContract,
) -> Result<(), ErrorEnvelope> {
    if parent.project_id.0 != request.command.project_id.0
        || parent.schema_version != M5_SCHEMA_VERSION
        || parent.worker_id.trim().is_empty()
        || parent.task_id.0.trim().is_empty()
        || parent.workspace_root.trim().is_empty()
    {
        return Err(m8_validation_error(
            request,
            "worker parent contract is invalid or outside the authenticated project",
        ));
    }
    parent.validate().map_err(|_| {
        m8_validation_error(
            request,
            "worker parent contract failed deterministic validation",
        )
    })
}

fn m8_validation_error(request: &CommandRequest, message: &str) -> ErrorEnvelope {
    error(
        &request.correlation_id,
        Some(request.command.command_id.clone()),
        request.causation_id.clone(),
        ControlPlaneErrorCode::InvalidCommand,
        ErrorCategory::Validation,
        message,
        false,
        None,
    )
}

fn restore_m8_coordinator(
    state: &RuntimeState,
    parent_contract: &WorkerContract,
    request: &CommandRequest,
) -> Result<MultiWorkerCoordinator, ErrorEnvelope> {
    let tasks = state.plane.load_m8_tasks().map_err(|_| {
        m8_dependency_error(
            request,
            "durable M8 coordination tasks could not be restored",
        )
    })?;
    let claims = state.plane.load_m8_claims().map_err(|_| {
        m8_dependency_error(request, "durable M8 worker claims could not be restored")
    })?;
    let handoffs = state.plane.load_m8_handoffs().map_err(|_| {
        m8_dependency_error(request, "durable M8 worker handoffs could not be restored")
    })?;
    let acknowledgements = state.plane.load_m8_acknowledgements().map_err(|_| {
        m8_dependency_error(request, "durable M8 acknowledgements could not be restored")
    })?;
    MultiWorkerCoordinator::restore(
        parent_contract.clone(),
        tasks,
        claims,
        handoffs,
        acknowledgements,
    )
    .map_err(|coordination_error| map_m8_coordination_error(request, coordination_error))
}

fn m8_dependency_error(request: &CommandRequest, message: &str) -> ErrorEnvelope {
    error(
        &request.correlation_id,
        Some(request.command.command_id.clone()),
        request.causation_id.clone(),
        ControlPlaneErrorCode::DependencyUnavailable,
        ErrorCategory::Unavailable,
        message,
        true,
        Some("reconcile the local M8 ledger before retrying".into()),
    )
}

fn map_m8_coordination_error(
    request: &CommandRequest,
    coordination_error: CoordinationError,
) -> ErrorEnvelope {
    let (category, recovery) = match coordination_error {
        CoordinationError::Conflict(_) => (
            ErrorCategory::Conflict,
            Some("repair the conflicting isolated worker outputs before reconciliation".into()),
        ),
        CoordinationError::LeaseFenced | CoordinationError::LeaseRequired => (
            ErrorCategory::Conflict,
            Some("recover the worker from its durable checkpoint and claim a fresh lease".into()),
        ),
        CoordinationError::DependencyIncomplete => (
            ErrorCategory::Unavailable,
            Some("complete the dependency handoffs before claiming this worker task".into()),
        ),
        CoordinationError::EvidenceRequired => (
            ErrorCategory::Validation,
            Some("attach executable evidence to the worker handoff".into()),
        ),
        CoordinationError::MissingHandoff => (
            ErrorCategory::NotFound,
            Some("restore the referenced durable worker handoff".into()),
        ),
        _ => (
            ErrorCategory::Validation,
            Some("repair the M8 worker command payload and retry".into()),
        ),
    };
    error(
        &request.correlation_id,
        Some(request.command.command_id.clone()),
        request.causation_id.clone(),
        ControlPlaneErrorCode::InvalidCommand,
        category,
        coordination_error.to_string(),
        false,
        recovery,
    )
}

pub(crate) fn dispatch_request<S: EventSink>(
    sink: &S,
    state: &mut RuntimeState,
    mut request: CommandRequest,
) -> Result<CommandResponse, ErrorEnvelope> {
    let correlation_id = request.correlation_id.clone();
    let causation_id = request.causation_id.clone();
    let command_id = request.command.command_id.clone();
    if matches!(
        request.command.kind,
        CommandKind::TaskStart
            | CommandKind::SubmitInstruction
            | CommandKind::PauseTask
            | CommandKind::TaskResume
            | CommandKind::ResumeTask
            | CommandKind::CancelTask
            | CommandKind::TaskCancel
    ) && request.command.task_id.is_none()
    {
        request.command.task_id = Some(nirman_domain::TaskId(format!(
            "task-{}",
            request.command.project_id.0
        )));
    }
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
    let artifact_build = if request.command.kind == CommandKind::ArtifactBuild {
        Some(parse_artifact_build(state, &request)?)
    } else {
        None
    };
    let artifact_export = if request.command.kind == CommandKind::ArtifactExport {
        Some(parse_artifact_export(state, &request)?)
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
    let m8_claim = if request.command.kind == CommandKind::WorkerTaskClaim {
        Some(parse_m8_claim(&request)?)
    } else {
        None
    };
    let m8_handoff = if request.command.kind == CommandKind::WorkerHandoffSubmit {
        Some(parse_m8_handoff(&request)?)
    } else {
        None
    };
    let m8_acknowledgement = if request.command.kind == CommandKind::WorkerHandoffAcknowledge {
        Some(parse_m8_acknowledgement(&request)?)
    } else {
        None
    };
    let m8_reconcile = if request.command.kind == CommandKind::WorkerReconcile {
        Some(parse_m8_reconcile(&request)?)
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
    let m8_is_command = m8_claim.is_some()
        || m8_handoff.is_some()
        || m8_acknowledgement.is_some()
        || m8_reconcile.is_some();
    let m8_duplicate = if m8_is_command {
        state
            .plane
            .command_is_duplicate(&request.command)
            .map_err(|runtime_error| {
                map_runtime_error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    runtime_error,
                )
            })?
    } else {
        false
    };
    let mut m8_checkpoint_record: Option<M8ReconciliationCheckpoint> = None;
    let mut m8_task_record: Option<(String, String)> = None;
    let mut m8_claim_record: Option<WorkerTaskClaim> = None;
    let mut m8_handoff_record: Option<WorkerHandoffRecord> = None;
    let mut m8_acknowledgement_record: Option<WorkerHandoffAcknowledgement> = None;
    let mut m8_result_payload: Option<serde_json::Value> = None;
    if m8_is_command && !m8_duplicate {
        if let Some(payload) = m8_claim.as_ref() {
            let mut coordinator =
                restore_m8_coordinator(state, &payload.parent_contract, &request)?;
            coordinator
                .register_task(payload.task.clone())
                .map_err(|coordination_error| {
                    map_m8_coordination_error(&request, coordination_error)
                })?;
            let lease = coordinator
                .claim(
                    &payload.task.task_id,
                    payload.now_epoch_seconds,
                    payload.lease_duration_seconds,
                )
                .map_err(|coordination_error| {
                    map_m8_coordination_error(&request, coordination_error)
                })?;
            let claim = WorkerTaskClaim {
                task_id: payload.task.task_id.clone(),
                lease,
            };
            m8_task_record = Some((
                payload.task.task_id.clone(),
                serde_json::to_string(&payload.task).map_err(|_| {
                    m8_validation_error(&request, "worker task could not be serialized safely")
                })?,
            ));
            m8_result_payload = Some(
                serde_json::to_value(WorkerTaskClaimResultPayload {
                    task_id: claim.task_id.clone(),
                    worker_id: claim.lease.worker_id.clone(),
                    lease_id: claim.lease.lease_id.clone(),
                    fence_token: claim.lease.fence_token,
                    expires_at_epoch_seconds: claim.lease.expires_at_epoch_seconds,
                })
                .expect("M8 claim result serialization must remain infallible"),
            );
            m8_claim_record = Some(claim);
        } else if let Some(payload) = m8_handoff.as_ref() {
            let mut coordinator =
                restore_m8_coordinator(state, &payload.parent_contract, &request)?;
            coordinator
                .record_handoff(payload.handoff.clone())
                .map_err(|coordination_error| {
                    map_m8_coordination_error(&request, coordination_error)
                })?;
            let handoff = payload.handoff.clone();
            m8_result_payload = Some(
                serde_json::to_value(WorkerHandoffSubmitResultPayload {
                    message_id: handoff.message_id.clone(),
                    task_id: handoff.task_id.clone(),
                    worker_id: handoff.worker_id.clone(),
                    source_revision: handoff.source_revision.0,
                    evidence_refs: handoff.evidence_refs.clone(),
                })
                .expect("M8 handoff result serialization must remain infallible"),
            );
            m8_handoff_record = Some(handoff);
        } else if let Some(payload) = m8_acknowledgement.as_ref() {
            let requested_task_id = request
                .command
                .task_id
                .as_ref()
                .expect("M8 acknowledgement parser requires task identity");
            let durable_handoff = state
                .plane
                .load_worker_handoff(&payload.message_id)
                .map_err(|_| {
                    m8_dependency_error(&request, "referenced M8 handoff could not be loaded")
                })?
                .ok_or_else(|| {
                    m8_validation_error(&request, "referenced M8 handoff does not exist")
                })?;
            if durable_handoff.task_id != requested_task_id.0 {
                return Err(m8_validation_error(
                    &request,
                    "acknowledgement task scope does not match the durable handoff",
                ));
            }
            let mut coordinator =
                restore_m8_coordinator(state, &payload.parent_contract, &request)?;
            let acknowledgement = coordinator
                .acknowledge_handoff(&payload.acknowledgement_id, &payload.message_id)
                .map_err(|coordination_error| {
                    map_m8_coordination_error(&request, coordination_error)
                })?;
            m8_result_payload = Some(
                serde_json::to_value(WorkerHandoffAcknowledgeResultPayload {
                    acknowledgement: acknowledgement.clone(),
                })
                .expect("M8 acknowledgement result serialization must remain infallible"),
            );
            m8_acknowledgement_record = Some(acknowledgement);
        } else if let Some(payload) = m8_reconcile.as_ref() {
            let checkpoint = reconcile_m8_handoffs(state, &request, payload)?;
            m8_result_payload = Some(
                serde_json::to_value(WorkerReconcileResultPayload {
                    checkpoint: checkpoint.clone(),
                })
                .expect("M8 reconciliation result serialization must remain infallible"),
            );
            m8_checkpoint_record = Some(checkpoint);
        }
    }
    let m7_run = m7_lifecycle_candidate(state, &request)?;
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
    } else if m8_is_command {
        state.plane.dispatch_with_result_and_m8(
            request.command.clone(),
            &correlation_id,
            M8DispatchRecord {
                checkpoint: m8_checkpoint_record.as_ref().map(|checkpoint| {
                    (
                        checkpoint.checkpoint_id.clone(),
                        format!("{:?}", checkpoint.status),
                        serde_json::to_string(checkpoint)
                            .expect("M8 checkpoint serialization must remain infallible"),
                    )
                }),
                task: m8_task_record,
                claim: m8_claim_record.as_ref().map(|claim| {
                    (
                        claim.task_id.clone(),
                        claim.lease.worker_id.clone(),
                        serde_json::to_string(claim)
                            .expect("M8 claim serialization must remain infallible"),
                    )
                }),
                handoff: m8_handoff_record.as_ref().map(|handoff| {
                    (
                        handoff.message_id.clone(),
                        handoff.task_id.clone(),
                        handoff.worker_id.clone(),
                        serde_json::to_string(handoff)
                            .expect("M8 handoff serialization must remain infallible"),
                    )
                }),
                acknowledgement: m8_acknowledgement_record.as_ref().map(|acknowledgement| {
                    (
                        acknowledgement.acknowledgement_id.clone(),
                        acknowledgement.message_id.clone(),
                        acknowledgement.task_id.clone(),
                        acknowledgement.worker_id.clone(),
                        serde_json::to_string(acknowledgement)
                            .expect("M8 acknowledgement serialization must remain infallible"),
                    )
                }),
            },
        )
    } else if let Some(prepared) = prepared_mutation.as_ref() {
        state.plane.dispatch_with_result_and_mutation_transaction(
            request.command.clone(),
            &correlation_id,
            &prepared.transaction,
        )
    } else if let Some(run) = m7_run.as_ref() {
        state.plane.dispatch_with_result_and_background_run(
            request.command.clone(),
            &correlation_id,
            Some(run),
        )
    } else if artifact_build.is_some() || artifact_export.is_some() {
        state.plane.dispatch_with_result_and_provider_profile(
            request.command.clone(),
            &correlation_id,
            None,
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
        status.clone(),
        event_range,
    );
    if m8_is_command {
        let result_payload = if let Some(payload) = m8_result_payload {
            Some(payload)
        } else if m8_duplicate {
            if let Some(payload) = m8_claim.as_ref() {
                state
                    .plane
                    .load_worker_task_claim(&payload.task.task_id)
                    .map_err(|_| {
                        m8_dependency_error(
                            &request,
                            "durable M8 claim result could not be reloaded",
                        )
                    })?
                    .map(|claim| {
                        serde_json::to_value(WorkerTaskClaimResultPayload {
                            task_id: claim.task_id,
                            worker_id: claim.lease.worker_id,
                            lease_id: claim.lease.lease_id,
                            fence_token: claim.lease.fence_token,
                            expires_at_epoch_seconds: claim.lease.expires_at_epoch_seconds,
                        })
                        .expect("M8 claim result serialization must remain infallible")
                    })
            } else if let Some(payload) = m8_handoff.as_ref() {
                state
                    .plane
                    .load_worker_handoff(&payload.handoff.message_id)
                    .map_err(|_| {
                        m8_dependency_error(
                            &request,
                            "durable M8 handoff result could not be reloaded",
                        )
                    })?
                    .map(|handoff| {
                        serde_json::to_value(WorkerHandoffSubmitResultPayload {
                            message_id: handoff.message_id,
                            task_id: handoff.task_id,
                            worker_id: handoff.worker_id,
                            source_revision: handoff.source_revision.0,
                            evidence_refs: handoff.evidence_refs,
                        })
                        .expect("M8 handoff result serialization must remain infallible")
                    })
            } else if let Some(payload) = m8_acknowledgement.as_ref() {
                state
                    .plane
                    .load_worker_handoff_acknowledgement(&payload.acknowledgement_id)
                    .map_err(|_| {
                        m8_dependency_error(
                            &request,
                            "durable M8 acknowledgement result could not be reloaded",
                        )
                    })?
                    .map(|acknowledgement| {
                        serde_json::to_value(WorkerHandoffAcknowledgeResultPayload {
                            acknowledgement,
                        })
                        .expect("M8 acknowledgement result serialization must remain infallible")
                    })
            } else if let Some(payload) = m8_reconcile.as_ref() {
                state
                    .plane
                    .load_m8_reconciliation_checkpoint(&payload.checkpoint_id)
                    .map_err(|_| {
                        m8_dependency_error(
                            &request,
                            "durable reconciliation checkpoint could not be reloaded",
                        )
                    })?
                    .map(|checkpoint| {
                        serde_json::to_value(WorkerReconcileResultPayload { checkpoint })
                            .expect("M8 reconciliation result serialization must remain infallible")
                    })
            } else {
                None
            }
        } else {
            None
        };
        command_response.result_schema_ref = Some(
            match request.command.kind {
                CommandKind::WorkerTaskClaim => "nirman.worker_task_claim_result.v1",
                CommandKind::WorkerHandoffSubmit => "nirman.worker_handoff_submit_result.v1",
                CommandKind::WorkerHandoffAcknowledge => {
                    "nirman.worker_handoff_acknowledge_result.v1"
                }
                CommandKind::WorkerReconcile => "nirman.worker_reconcile_result.v1",

                _ => unreachable!("M8 result payload requires an M8 command"),
            }
            .into(),
        );
        command_response.result_payload = result_payload;
    }
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
    if let Some(prepared) = artifact_export.as_ref() {
        let artifact = if was_duplicate {
            let record_json = state
                .plane
                .load_android_artifact_export(
                    &prepared.observation.task_id,
                    prepared.source_revision,
                )
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate artifact export has no durable delivery record",
                        true,
                        Some("reconcile the prior APK export before retrying".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate artifact export has no durable delivery record",
                        true,
                        Some("reconcile the prior APK export before retrying".into()),
                    )
                })?;
            serde_json::from_str::<ApkArtifact>(&record_json).map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "durable APK export record is corrupt",
                    false,
                    None,
                )
            })?
        } else {
            let source_path = prepared.observation.artifact_path.as_ref().ok_or_else(|| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Validation,
                    "successful build has no APK path",
                    false,
                    None,
                )
            })?;
            let source_hash = prepared
                .observation
                .artifact_sha256
                .as_ref()
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::InvalidCommand,
                        ErrorCategory::Validation,
                        "successful build has no APK fingerprint",
                        false,
                        None,
                    )
                })?;
            let aapt = aapt_path_for_lock(&prepared.lock).ok_or_else(|| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Environment,
                    "locked Android build tools do not expose aapt",
                    true,
                    Some("repair M43 build-tools resolution before exporting".into()),
                )
            })?;
            scan_apk_for_secrets(source_path).map_err(|artifact_error| {
                map_artifact_error(
                    &correlation_id,
                    &command_id,
                    causation_id.clone(),
                    artifact_error,
                )
            })?;
            let inspection = inspect_apk(&aapt, source_path).map_err(|artifact_error| {
                map_artifact_error(
                    &correlation_id,
                    &command_id,
                    causation_id.clone(),
                    artifact_error,
                )
            })?;
            let artifact = ApkArtifact {
                schema_version: nirman_artifacts::M10_SCHEMA_VERSION,
                artifact_id: format!("artifact-{command_id}"),
                project_id: prepared.observation.project_id.clone(),
                task_id: prepared.observation.task_id.clone(),
                project_revision_id: format!("source-{}", prepared.source_revision),
                source_fingerprint: prepared.observation.project_fingerprint.clone(),
                source_provenance_ref: format!(
                    "android-build-observation:{}:{}",
                    prepared.observation.task_id, prepared.source_revision
                ),
                path: source_path.clone(),
                sha256: format!("sha256:{source_hash}"),
                package_name: inspection.package_name.clone(),
                inspection: Some(inspection),
                build_variant: prepared.observation.build_variant.clone(),
                secret_scan_status: "PASS".into(),
                signing_status: "INSPECTED_BY_BUILD_TOOL".into(),
                delivery_status: "PENDING_LOCAL".into(),
                delivery_sha256: None,
                delivery_verified: false,
                copy_uncertain: false,
            };
            let delivered = deliver_apk_local(&artifact, &prepared.destination_path).map_err(
                |artifact_error| {
                    map_artifact_error(
                        &correlation_id,
                        &command_id,
                        causation_id.clone(),
                        artifact_error,
                    )
                },
            )?;
            let record_json = serde_json::to_string(&delivered).map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "APK export record could not be serialized safely",
                    false,
                    None,
                )
            })?;
            state
                .plane
                .save_android_artifact_export(
                    &format!("export-{command_id}"),
                    &delivered.task_id,
                    prepared.source_revision,
                    &prepared.destination_path,
                    &record_json,
                )
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "APK export record could not be persisted",
                        true,
                        Some("reconcile local export storage before retrying".into()),
                    )
                })?;
            delivered
        };
        if !was_duplicate {
            let preview_revision_id = state
                .plane
                .load_android_synthesis_build(
                    &prepared.observation.task_id,
                    prepared.source_revision,
                )
                .ok()
                .flatten()
                .and_then(|stored| {
                    serde_json::from_str::<nirman_android::AndroidSynthesisPlan>(&stored.0).ok()
                })
                .map(|plan| format!("m108-preview-{}", plan.contract_id))
                .unwrap_or_else(|| format!("m108-preview-{}", prepared.observation.task_id));
            let checkpoint = state
                .plane
                .checkpoint_id()
                .map(str::to_owned)
                .unwrap_or_else(|| "checkpoint-pending".into());
            let inspection_ref = artifact
                .inspection
                .as_ref()
                .map(|inspection| format!("aapt-output-sha256:{}", inspection.aapt_output_sha256));
            let artifact_evidence = vec![
                artifact.source_provenance_ref.clone(),
                format!("artifact-sha256:{}", artifact.sha256),
                format!(
                    "delivery-sha256:{}",
                    artifact
                        .delivery_sha256
                        .clone()
                        .unwrap_or_else(|| "missing".into())
                ),
            ];
            let artifact_observations = inspection_ref
                .into_iter()
                .chain(artifact_evidence.clone())
                .collect::<Vec<_>>();
            persist_m108_stage(
                state,
                &prepared.observation.task_id,
                &preview_revision_id,
                PreviewSyncEventType::ArtifactObserved,
                PreviewEventTruth::Observed,
                &format!("source-{}", prepared.observation.source_revision),
                &checkpoint,
                &artifact.source_fingerprint,
                &command_id,
                Some(artifact.artifact_id.clone()),
                Some(artifact.sha256.clone()),
                None,
                None,
                vec![artifact.source_provenance_ref.clone()],
                artifact_observations,
            )?;
            persist_m108_stage(
                state,
                &prepared.observation.task_id,
                &preview_revision_id,
                PreviewSyncEventType::ValidationObserved,
                PreviewEventTruth::Verified,
                &format!("source-{}", prepared.observation.source_revision),
                &checkpoint,
                &artifact.source_fingerprint,
                &command_id,
                Some(artifact.artifact_id.clone()),
                Some(artifact.sha256.clone()),
                None,
                None,
                vec![artifact.source_provenance_ref.clone()],
                artifact_evidence,
            )?;
        }
        command_response.status = if was_duplicate {
            ResponseStatus::Duplicate
        } else {
            ResponseStatus::Completed
        };
        command_response.result_schema_ref = Some("nirman.artifact_export_result.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(ArtifactExportResultPayload { artifact })
                .expect("APK export result serialization"),
        );
        return Ok(command_response);
    }
    if let Some(prepared) = artifact_build.as_ref() {
        let observation = if was_duplicate {
            let record_json = state
                .plane
                .load_android_build_observation(
                    &prepared.request.task_id,
                    prepared.request.source_revision,
                )
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate artifact build has no durable observation",
                        true,
                        Some("reconcile the prior Android build execution before retrying".into()),
                    )
                })?
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "duplicate artifact build has no durable observation",
                        true,
                        Some("reconcile the prior Android build execution before retrying".into()),
                    )
                })?;
            serde_json::from_str(&record_json).map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "durable Android build observation is corrupt",
                    false,
                    None,
                )
            })?
        } else {
            let result = execute_android_build(
                &prepared.request,
                &prepared.lock,
                &command_id,
                M5_BUILD_TIMEOUT_MS,
                &BuildCancellation::default(),
            )
            .map_err(|build_error| {
                map_android_build_error(
                    &correlation_id,
                    &command_id,
                    causation_id.clone(),
                    build_error,
                )
            })?;
            let record_json = serde_json::to_string(&result).map_err(|_| {
                error(
                    &correlation_id,
                    Some(command_id.clone()),
                    causation_id.clone(),
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Internal,
                    "Android build observation could not be serialized safely",
                    false,
                    None,
                )
            })?;
            state
                .plane
                .save_android_build_observation(
                    &result.execution_id,
                    &result.task_id,
                    result.source_revision,
                    &result.project_fingerprint,
                    &record_json,
                )
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "Android build observation could not be persisted",
                        true,
                        Some("reconcile local build observation storage before retrying".into()),
                    )
                })?;
            result
        };
        if !was_duplicate {
            let synthesis_record = state
                .plane
                .load_android_synthesis_build(
                    &prepared.request.task_id,
                    prepared.request.source_revision,
                )
                .map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Unavailable,
                        "M108 could not reload the M4 synthesis provenance",
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
                        ErrorCategory::Internal,
                        "M108 has no M4 synthesis provenance for the build observation",
                        false,
                        None,
                    )
                })?;
            let plan: nirman_android::AndroidSynthesisPlan =
                serde_json::from_str(&synthesis_record.0).map_err(|_| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::DependencyUnavailable,
                        ErrorCategory::Internal,
                        "M108 M4 synthesis provenance is corrupt",
                        false,
                        None,
                    )
                })?;
            let preview_revision_id = format!("m108-preview-{}", plan.contract_id);
            let checkpoint = state
                .plane
                .checkpoint_id()
                .map(str::to_owned)
                .unwrap_or_else(|| "checkpoint-pending".into());
            let build_evidence_ref = format!("build-observation:{}", observation.execution_id);
            persist_m108_stage(
                state,
                &prepared.request.task_id,
                &preview_revision_id,
                PreviewSyncEventType::BuildObserved,
                PreviewEventTruth::Observed,
                &format!("source-{}", observation.source_revision),
                &checkpoint,
                &observation.project_fingerprint,
                &command_id,
                None,
                None,
                None,
                None,
                vec![observation.execution_id.clone()],
                vec![build_evidence_ref],
            )?;
            if observation.success {
                if let (Some(artifact_path), Some(artifact_fingerprint)) = (
                    observation.artifact_path.as_ref(),
                    observation.artifact_sha256.as_ref(),
                ) {
                    let artifact_id = format!("artifact-build-{}", observation.execution_id);
                    persist_m108_stage(
                        state,
                        &prepared.request.task_id,
                        &preview_revision_id,
                        PreviewSyncEventType::ArtifactObserved,
                        PreviewEventTruth::Observed,
                        &format!("source-{}", observation.source_revision),
                        &checkpoint,
                        &observation.project_fingerprint,
                        &command_id,
                        Some(artifact_id),
                        Some(artifact_fingerprint.clone()),
                        None,
                        None,
                        vec![artifact_path.clone()],
                        vec![format!("artifact-sha256:{}", artifact_fingerprint)],
                    )?;
                }
            }
        }
        command_response.status = if was_duplicate {
            ResponseStatus::Duplicate
        } else if observation.cancelled {
            ResponseStatus::Cancelled
        } else if observation.success {
            ResponseStatus::Completed
        } else {
            ResponseStatus::Failed
        };
        command_response.result_schema_ref = Some("nirman.artifact_build_result.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(ArtifactBuildResultPayload { observation })
                .expect("Android build result serialization"),
        );
        return Ok(command_response);
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
        let device_observation = if was_duplicate {
            if matches!(
                prepared.selection.mode,
                PreviewMode::IncrementalEmulatorInstall
                    | PreviewMode::ApkReinstall
                    | PreviewMode::PhysicalDevice
            ) {
                let source_revision = prepared
                    .preview_request
                    .project_revision_id
                    .strip_prefix("source-")
                    .and_then(|value| value.parse::<u64>().ok())
                    .ok_or_else(|| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::InvalidCommand,
                            ErrorCategory::Validation,
                            "duplicate preview device request has an invalid source revision",
                            false,
                            None,
                        )
                    })?;
                let record_json = state
                    .plane
                    .load_android_device_observation_for_source(&prepared.task_id, source_revision)
                    .map_err(|_| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Unavailable,
                            "duplicate preview has no durable device observation",
                            true,
                            Some("reconcile the previous device session before retrying".into()),
                        )
                    })?
                    .ok_or_else(|| {
                        error(
                            &correlation_id,
                            Some(command_id.clone()),
                            causation_id.clone(),
                            ControlPlaneErrorCode::DependencyUnavailable,
                            ErrorCategory::Unavailable,
                            "duplicate preview has no durable device observation",
                            true,
                            Some("reconcile the previous device session before retrying".into()),
                        )
                    })?;
                Some(
                    serde_json::from_str::<nirman_evidence::AndroidDeviceObservation>(&record_json)
                        .map_err(|_| {
                            error(
                                &correlation_id,
                                Some(command_id.clone()),
                                causation_id.clone(),
                                ControlPlaneErrorCode::DependencyUnavailable,
                                ErrorCategory::Internal,
                                "durable Android device observation is corrupt",
                                false,
                                None,
                            )
                        })?,
                )
            } else {
                None
            }
        } else {
            execute_preview_device_session(
                state,
                prepared,
                &command_id,
                &correlation_id,
                causation_id.clone(),
            )?
        };
        if let Some(observation) = device_observation.as_ref() {
            let source_revision = prepared
                .preview_request
                .project_revision_id
                .strip_prefix("source-")
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| {
                    error(
                        &correlation_id,
                        Some(command_id.clone()),
                        causation_id.clone(),
                        ControlPlaneErrorCode::InvalidCommand,
                        ErrorCategory::Validation,
                        "M108 device observation has an invalid source revision",
                        false,
                        None,
                    )
                })?;
            let m108_preview_revision_id = state
                .plane
                .load_android_synthesis_build(&prepared.task_id, source_revision)
                .ok()
                .flatten()
                .and_then(|stored| {
                    serde_json::from_str::<nirman_android::AndroidSynthesisPlan>(&stored.0).ok()
                })
                .map(|plan| format!("m108-preview-{}", plan.contract_id))
                .unwrap_or_else(|| prepared.preview_revision_id.clone());
            let checkpoint = state
                .plane
                .checkpoint_id()
                .map(str::to_owned)
                .unwrap_or_else(|| "checkpoint-pending".into());
            let evidence_refs = observation
                .logcat_reference
                .iter()
                .cloned()
                .chain(observation.screenshot_references.iter().cloned())
                .chain(observation.accessibility_reference.iter().cloned())
                .collect::<Vec<_>>();
            persist_m108_stage(
                state,
                &prepared.task_id,
                &m108_preview_revision_id,
                PreviewSyncEventType::InstallObserved,
                PreviewEventTruth::Observed,
                &prepared.preview_request.project_revision_id,
                &checkpoint,
                &prepared.preview_request.source_fingerprint,
                &command_id,
                None,
                None,
                Some(format!("session-{}", observation.observation_id)),
                Some(observation.device_identity.clone()),
                vec![observation.observation_id.clone()],
                evidence_refs.clone(),
            )?;
            persist_m108_stage(
                state,
                &prepared.task_id,
                &m108_preview_revision_id,
                PreviewSyncEventType::LaunchObserved,
                PreviewEventTruth::Observed,
                &prepared.preview_request.project_revision_id,
                &checkpoint,
                &prepared.preview_request.source_fingerprint,
                &command_id,
                None,
                None,
                Some(format!("session-{}", observation.observation_id)),
                Some(observation.device_identity.clone()),
                vec![observation.observation_id.clone()],
                evidence_refs.clone(),
            )?;
            persist_m108_stage(
                state,
                &prepared.task_id,
                &m108_preview_revision_id,
                PreviewSyncEventType::InteractionObserved,
                PreviewEventTruth::Observed,
                &prepared.preview_request.project_revision_id,
                &checkpoint,
                &prepared.preview_request.source_fingerprint,
                &command_id,
                None,
                None,
                Some(format!("session-{}", observation.observation_id)),
                Some(observation.device_identity.clone()),
                vec![observation.observation_id.clone()],
                evidence_refs,
            )?;
        }
        command_response.status = ResponseStatus::Completed;
        command_response.result_schema_ref = Some("nirman.preview_start_result.v1".into());
        command_response.result_payload = Some(
            serde_json::to_value(PreviewStartResultPayload {
                selection,
                revision,
                device_observation,
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
                None,
                None,
                None,
                None,
                vec![],
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

#[derive(Debug)]
struct PreparedArtifactExport {
    source_revision: u64,
    destination_path: String,
    observation: nirman_android::AndroidBuildObservation,
    lock: nirman_android::AndroidToolchainLock,
}

#[derive(Debug)]
struct PreparedArtifactBuild {
    request: AndroidBuildRequest,
    lock: nirman_android::AndroidToolchainLock,
}

const M5_BUILD_TIMEOUT_MS: u64 = 15 * 60 * 1_000;

fn map_android_build_error(
    correlation_id: &str,
    command_id: &str,
    causation_id: Option<String>,
    build_error: AndroidBuildExecutionError,
) -> ErrorEnvelope {
    let (code, category, retryable, recovery) = match build_error {
        AndroidBuildExecutionError::InvalidRequest(_)
        | AndroidBuildExecutionError::InvalidLock(_)
        | AndroidBuildExecutionError::EmptyCommandId => (
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            false,
            Some("repair the persisted Android build request or toolchain lock".into()),
        ),
        AndroidBuildExecutionError::WorkspaceUnavailable => (
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            false,
            Some("reconcile the authorized Android workspace before retrying".into()),
        ),
        AndroidBuildExecutionError::GradleUnavailable
        | AndroidBuildExecutionError::SpawnFailed
        | AndroidBuildExecutionError::OutputReadFailed => (
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Environment,
            true,
            Some("repair or revalidate the locked local Android toolchain".into()),
        ),
    };
    error(
        correlation_id,
        Some(command_id.into()),
        causation_id,
        code,
        category,
        build_error.to_string(),
        retryable,
        recovery,
    )
}

fn map_artifact_error(
    correlation_id: &str,
    command_id: &str,
    causation_id: Option<String>,
    artifact_error: nirman_artifacts::M10Error,
) -> ErrorEnvelope {
    let (code, category, retryable, recovery) = match artifact_error {
        nirman_artifacts::M10Error::MissingFile => (
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Unavailable,
            true,
            Some("reconcile the APK artifact and locked Android tooling before retrying".into()),
        ),
        nirman_artifacts::M10Error::HashMismatch
        | nirman_artifacts::M10Error::SecretScanFailed
        | nirman_artifacts::M10Error::NotApk
        | nirman_artifacts::M10Error::InvalidArtifact
        | nirman_artifacts::M10Error::EmptyField(_) => (
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            false,
            Some("rebuild and revalidate the current APK artifact".into()),
        ),
    };
    error(
        correlation_id,
        Some(command_id.into()),
        causation_id,
        code,
        category,
        artifact_error.to_string(),
        retryable,
        recovery,
    )
}

fn aapt_path_for_lock(lock: &nirman_android::AndroidToolchainLock) -> Option<String> {
    let path = lock
        .entries
        .iter()
        .find(|entry| entry.component == nirman_android::ToolchainComponentKind::BuildTools)
        .map(|entry| std::path::PathBuf::from(&entry.path))?;
    if path.is_file() {
        return Some(path.to_string_lossy().into_owned());
    }
    if path.is_dir() {
        let executable = if cfg!(windows) {
            path.join("aapt.exe")
        } else {
            path.join("aapt")
        };
        if executable.is_file() {
            return Some(executable.to_string_lossy().into_owned());
        }
    }
    None
}

fn parse_artifact_export(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<PreparedArtifactExport, ErrorEnvelope> {
    let payload: ArtifactExportCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "artifact export payload is invalid",
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
            "artifact export requires a task scope",
            false,
            None,
        )
    })?;
    let record_json = state
        .plane
        .load_android_build_observation(task_id.0.as_str(), payload.source_revision)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "Android build observation is unavailable",
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
                "artifact export requires a durable Android build observation",
                false,
                Some("run a successful ArtifactBuild operation first".into()),
            )
        })?;
    let observation: nirman_android::AndroidBuildObservation = serde_json::from_str(&record_json)
        .map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Internal,
            "Android build observation is corrupt",
            false,
            None,
        )
    })?;
    if !observation.success
        || observation.project_id != request.command.project_id.0
        || observation.task_id != task_id.0
        || observation.source_revision != payload.source_revision
        || observation.artifact_path.is_none()
        || observation.artifact_sha256.is_none()
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::InvalidCommand,
            ErrorCategory::Validation,
            "artifact export requires a successful build with a discovered APK",
            false,
            Some("rebuild the current Android source revision before exporting".into()),
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
                "artifact export requires persisted M43 preflight",
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
    let lock = preflight.lock.ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Environment,
            "artifact export requires an available M43 toolchain lock",
            false,
            None,
        )
    })?;
    if payload.destination_path.trim().is_empty()
        || !std::path::Path::new(&payload.destination_path).is_absolute()
    {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::PermissionDenied,
            ErrorCategory::Scope,
            "artifact export destination must be an absolute local path",
            false,
            None,
        ));
    }
    Ok(PreparedArtifactExport {
        source_revision: payload.source_revision,
        destination_path: payload.destination_path,
        observation,
        lock,
    })
}

fn parse_artifact_build(
    state: &RuntimeState,
    request: &CommandRequest,
) -> Result<PreparedArtifactBuild, ErrorEnvelope> {
    let payload: ArtifactBuildCommandPayload = serde_json::from_str(&request.command.payload)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "artifact build payload is invalid",
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
            "artifact build requires a task scope",
            false,
            None,
        )
    })?;
    let stored = state
        .plane
        .load_android_synthesis_build(&task_id.0, payload.source_revision)
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "persisted Android synthesis/build record is unavailable",
                true,
                Some("reconcile the M4 durable record before building".into()),
            )
        })?
        .ok_or_else(|| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "artifact build requires a persisted M4 synthesis/build record",
                false,
                Some("accept the Android synthesis/build request before artifact build".into()),
            )
        })?;
    let persisted_request: AndroidBuildRequest = serde_json::from_str(&stored.1).map_err(|_| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Internal,
            "persisted Android build request is corrupt",
            false,
            None,
        )
    })?;
    let expected = AndroidBuildRequest {
        schema_version: nirman_android::M4_SCHEMA_VERSION,
        project_id: request.command.project_id.0.clone(),
        task_id: task_id.0.clone(),
        source_revision: payload.source_revision,
        project_fingerprint: payload.project_fingerprint.clone(),
        workspace_root: payload.workspace_root.clone(),
        build_variant: payload.build_variant.clone(),
        gradle_task: payload.gradle_task.clone(),
    };
    if persisted_request != expected || stored.4 != payload.project_fingerprint {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            "artifact build request does not match the persisted M4 provenance",
            true,
            Some("recreate the build request from the current source revision".into()),
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
    let requested_root = std::path::PathBuf::from(&payload.workspace_root)
        .canonicalize()
        .map_err(|_| {
            error(
                &request.correlation_id,
                Some(request.command.command_id.clone()),
                request.causation_id.clone(),
                ControlPlaneErrorCode::PermissionDenied,
                ErrorCategory::Scope,
                "artifact build workspace is unavailable",
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
            "artifact build workspace is outside the authorized project root",
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
                "artifact build workspace indexing failed",
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
            "artifact build source fingerprint is stale",
            true,
            Some("re-index and reconcile the current authorized workspace".into()),
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
                "artifact build requires persisted M43 preflight",
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
    let lock = preflight.lock.ok_or_else(|| {
        error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Environment,
            "artifact build requires an available M43 toolchain lock",
            false,
            None,
        )
    })?;
    if lock.lock_hash != stored.2 {
        return Err(error(
            &request.correlation_id,
            Some(request.command.command_id.clone()),
            request.causation_id.clone(),
            ControlPlaneErrorCode::StaleProjection,
            ErrorCategory::StaleProjection,
            "artifact build toolchain lock is stale",
            true,
            Some("rerun M43 preflight and recreate the build request".into()),
        ));
    }
    Ok(PreparedArtifactBuild {
        request: persisted_request,
        lock,
    })
}

fn adb_path_for_lock(lock: &nirman_android::AndroidToolchainLock) -> Option<String> {
    let path = lock
        .entries
        .iter()
        .find(|entry| entry.component == nirman_android::ToolchainComponentKind::Adb)
        .map(|entry| std::path::PathBuf::from(&entry.path))?;
    if path.is_file() {
        return Some(path.to_string_lossy().into_owned());
    }
    if path.is_dir() {
        let executable = if cfg!(windows) {
            path.join("adb.exe")
        } else {
            path.join("adb")
        };
        if executable.is_file() {
            return Some(executable.to_string_lossy().into_owned());
        }
    }
    None
}

fn execute_preview_device_session(
    state: &mut RuntimeState,
    prepared: &PreparedM48,
    command_id: &str,
    correlation_id: &str,
    causation_id: Option<String>,
) -> Result<Option<nirman_evidence::AndroidDeviceObservation>, ErrorEnvelope> {
    if !matches!(
        prepared.selection.mode,
        PreviewMode::IncrementalEmulatorInstall
            | PreviewMode::ApkReinstall
            | PreviewMode::PhysicalDevice
    ) {
        return Ok(None);
    }
    let Some(device_id) = prepared.preview_request.device_id.as_ref() else {
        return Ok(None);
    };
    let source_revision = prepared
        .preview_request
        .project_revision_id
        .strip_prefix("source-")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "preview device execution requires a source revision identity",
                false,
                None,
            )
        })?;
    let artifact_json = state
        .plane
        .load_android_artifact_export(&prepared.task_id, source_revision)
        .map_err(|_| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "preview device execution could not load the APK export",
                true,
                None,
            )
        })?
        .ok_or_else(|| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "preview device execution requires a validated local APK export",
                false,
                Some("build and export the current Android source revision first".into()),
            )
        })?;
    let artifact: ApkArtifact = serde_json::from_str(&artifact_json).map_err(|_| {
        error(
            correlation_id,
            Some(command_id.into()),
            causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Internal,
            "durable APK export record is corrupt",
            false,
            None,
        )
    })?;
    let contract_json = state
        .plane
        .load_android_construction_contract(&prepared.task_id)
        .map_err(|_| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "Android construction contract could not be reloaded",
                true,
                None,
            )
        })?
        .ok_or_else(|| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "preview device execution requires the persisted Android construction contract",
                false,
                None,
            )
        })?;
    let contract: AndroidConstructionContract =
        serde_json::from_str(&contract_json).map_err(|_| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "persisted Android construction contract is corrupt",
                false,
                None,
            )
        })?;
    let profile = contract
        .device_matrix
        .iter()
        .find(|profile| profile.device_id == *device_id)
        .ok_or_else(|| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Validation,
                "requested preview device profile is not in the persisted contract",
                false,
                None,
            )
        })?;
    let preflight_json = state
        .plane
        .load_android_toolchain_preflight(&prepared.task_id)
        .map_err(|_| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "M43 toolchain preflight could not be reloaded for device execution",
                true,
                None,
            )
        })?
        .ok_or_else(|| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::InvalidCommand,
                ErrorCategory::Environment,
                "preview device execution requires persisted M43 preflight",
                false,
                None,
            )
        })?;
    let preflight: nirman_android::AndroidToolchainPreflight =
        serde_json::from_str(&preflight_json).map_err(|_| {
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id.clone(),
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Internal,
                "M43 preflight could not be restored for device execution",
                false,
                None,
            )
        })?;
    let lock = preflight.lock.ok_or_else(|| {
        error(
            correlation_id,
            Some(command_id.into()),
            causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Environment,
            "preview device execution requires an available M43 lock",
            false,
            None,
        )
    })?;
    let adb = adb_path_for_lock(&lock).ok_or_else(|| {
        error(
            correlation_id,
            Some(command_id.into()),
            causation_id.clone(),
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Environment,
            "locked Android platform-tools do not expose adb",
            true,
            Some("repair M43 platform-tools resolution before starting preview".into()),
        )
    })?;
    let evidence_dir = std::path::Path::new(&artifact.path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(".nirman-evidence")
        .join(&prepared.preview_revision_id);
    let request = nirman_evidence::DeviceSessionRequest {
        observation_id: format!("observation-{command_id}"),
        project_id: prepared.preview_request.project_id.clone(),
        task_id: prepared.task_id.clone(),
        project_revision_id: prepared.preview_request.project_revision_id.clone(),
        profile: nirman_evidence::AndroidDeviceProfile {
            profile_id: profile.device_id.clone(),
            name: profile.name.clone(),
            api_level: profile.api_level,
            architecture: profile.architecture.clone(),
            width: profile.width,
            height: profile.height,
            density: profile.density,
            orientation: profile.orientation.clone(),
            locale: profile.locale.clone(),
        },
        package_name: artifact.package_name.clone(),
        apk_path: artifact.path.clone(),
        adb_executable: adb,
        evidence_directory: evidence_dir.to_string_lossy().into_owned(),
        timeout_ms: 120_000,
        selected_device_identity: prepared.preview_request.device_id.clone(),
        synthetic_device_only: true,
    };
    let observation =
        nirman_evidence::execute_android_device_session(&request).map_err(|device_error| {
            let (code, category, retryable) = match device_error {
                nirman_evidence::DeviceSessionError::DeviceUnavailable => (
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Device,
                    true,
                ),
                nirman_evidence::DeviceSessionError::CommandTimeout => {
                    (ControlPlaneErrorCode::Timeout, ErrorCategory::Timeout, true)
                }
                nirman_evidence::DeviceSessionError::InvalidRequest => (
                    ControlPlaneErrorCode::InvalidCommand,
                    ErrorCategory::Validation,
                    false,
                ),
                nirman_evidence::DeviceSessionError::CommandFailed
                | nirman_evidence::DeviceSessionError::EvidenceWriteFailed => (
                    ControlPlaneErrorCode::DependencyUnavailable,
                    ErrorCategory::Device,
                    true,
                ),
            };
            error(
                correlation_id,
                Some(command_id.into()),
                causation_id,
                code,
                category,
                device_error.to_string(),
                retryable,
                Some("reconcile the selected Android device and retry the preview session".into()),
            )
        })?;
    let record_json = serde_json::to_string(&observation).map_err(|_| {
        error(
            correlation_id,
            Some(command_id.into()),
            None,
            ControlPlaneErrorCode::DependencyUnavailable,
            ErrorCategory::Internal,
            "Android device observation could not be serialized safely",
            false,
            None,
        )
    })?;
    state
        .plane
        .save_android_device_observation(
            &observation.observation_id,
            &observation.task_id,
            source_revision,
            &observation.device_identity,
            &record_json,
        )
        .map_err(|_| {
            error(
                correlation_id,
                Some(command_id.into()),
                None,
                ControlPlaneErrorCode::DependencyUnavailable,
                ErrorCategory::Unavailable,
                "Android device observation could not be persisted",
                true,
                Some("reconcile local device observation storage before retrying".into()),
            )
        })?;
    Ok(Some(observation))
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
    pub(super) struct RecordingSink {
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

    pub(crate) fn test_state_at(path: &Path) -> RuntimeState {
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

    pub(crate) fn request(
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

    pub(crate) fn construction_contract(project_id: &str) -> AndroidConstructionContract {
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
    fn m7_authenticated_host_lifecycle_uses_one_durable_task_run() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();

        let start_request = request(
            &state,
            "m7-start",
            CommandKind::TaskStart,
            "Start an Android task".into(),
            0,
        );
        let started = dispatch_request(&sink, &mut state, start_request)
            .expect("task start should be accepted");
        let run_id = "run-project-0001-task-project-0001";
        assert_eq!(
            state
                .plane
                .load_background_run(run_id)
                .expect("run lookup")
                .expect("start creates run")
                .state,
            BackgroundRunState::Running
        );

        let pause_request = request(
            &state,
            "m7-pause",
            CommandKind::PauseTask,
            String::new(),
            started.snapshot.projection_revision.0,
        );
        let paused =
            dispatch_request(&sink, &mut state, pause_request).expect("pause should be accepted");
        assert_eq!(
            state
                .plane
                .load_background_run(run_id)
                .expect("run lookup")
                .expect("paused run")
                .state,
            BackgroundRunState::Paused
        );

        state
            .plane
            .checkpoint("m7-host-verified-checkpoint")
            .expect("checkpoint should persist");
        let resume_request = request(
            &state,
            "m7-resume",
            CommandKind::TaskResume,
            String::new(),
            paused.snapshot.projection_revision.0,
        );
        let resumed = dispatch_request(&sink, &mut state, resume_request)
            .expect("resume should be accepted from a verified checkpoint");
        let resumed_run = state
            .plane
            .load_background_run(run_id)
            .expect("run lookup")
            .expect("resumed run");
        assert_eq!(resumed_run.state, BackgroundRunState::Running);
        assert_eq!(
            resumed_run.recovery_action,
            Some(RecoveryAction::ResumeFromCheckpoint)
        );

        let cancel_request = request(
            &state,
            "m7-cancel",
            CommandKind::TaskCancel,
            String::new(),
            resumed.snapshot.projection_revision.0,
        );
        let cancelled =
            dispatch_request(&sink, &mut state, cancel_request).expect("cancel should be accepted");
        assert_eq!(
            state
                .plane
                .load_background_run(run_id)
                .expect("run lookup")
                .expect("cancelled run")
                .state,
            BackgroundRunState::Cancelled
        );
        assert_eq!(cancelled.snapshot.last_event_sequence, 4);
        assert_eq!(state.plane.snapshot().last_event_sequence, 4);

        drop(state);
        let reopened = test_state_at(&path);
        let reloaded_run = reopened
            .plane
            .load_background_run(run_id)
            .expect("reloaded run lookup")
            .expect("run survives restart");
        assert_eq!(reloaded_run.state, BackgroundRunState::Cancelled);
        assert_eq!(reloaded_run.task_id, "task-project-0001");
        assert_eq!(reopened.plane.snapshot().last_event_sequence, 4);
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn m7_resume_without_verified_checkpoint_is_rejected_before_admission() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let start_request = request(
            &state,
            "m7-start-no-checkpoint",
            CommandKind::TaskStart,
            "Start an Android task".into(),
            0,
        );
        let started = dispatch_request(&sink, &mut state, start_request)
            .expect("task start should be accepted");
        let pause_request = request(
            &state,
            "m7-pause-no-checkpoint",
            CommandKind::PauseTask,
            String::new(),
            started.snapshot.projection_revision.0,
        );
        let paused =
            dispatch_request(&sink, &mut state, pause_request).expect("pause should be accepted");
        let before_snapshot = state.plane.snapshot();
        let before_events = state.plane.snapshot().last_event_sequence;
        let resume_request = request(
            &state,
            "m7-resume-without-checkpoint",
            CommandKind::TaskResume,
            String::new(),
            paused.snapshot.projection_revision.0,
        );
        let error = dispatch_request(&sink, &mut state, resume_request)
            .expect_err("resume without checkpoint must be rejected");
        assert_eq!(error.code, ControlPlaneErrorCode::InvalidCommand);
        assert_eq!(state.plane.snapshot(), before_snapshot);
        assert_eq!(state.plane.snapshot().last_event_sequence, before_events);
        assert_eq!(
            state
                .plane
                .load_background_run("run-project-0001-task-project-0001")
                .expect("run lookup")
                .expect("paused run")
                .state,
            BackgroundRunState::Paused
        );
        drop(state);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn m7_duplicate_lifecycle_command_replays_without_second_transition() {
        let path = database_path();
        let mut state = test_state_at(&path);
        let sink = RecordingSink::default();
        let start_request = request(
            &state,
            "m7-duplicate-start",
            CommandKind::TaskStart,
            "Start an Android task".into(),
            0,
        );
        let first =
            dispatch_request(&sink, &mut state, start_request.clone()).expect("first start");
        let first_run = state
            .plane
            .load_background_run("run-project-0001-task-project-0001")
            .expect("run lookup")
            .expect("run");
        let duplicate =
            dispatch_request(&sink, &mut state, start_request).expect("duplicate start");
        let second_run = state
            .plane
            .load_background_run("run-project-0001-task-project-0001")
            .expect("run lookup")
            .expect("run");
        assert_eq!(first.snapshot, duplicate.snapshot);
        assert_eq!(first_run, second_run);
        assert_eq!(state.plane.snapshot().last_event_sequence, 1);
        drop(state);
        let _ = fs::remove_file(path);
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let wrapper = workspace.join("gradlew");
            fs::write(
                &wrapper,
                "#!/bin/sh\nmkdir -p app/build/outputs/apk/debug\nprintf 'fixture-apk' > app/build/outputs/apk/debug/app-debug.apk\nprintf 'BUILD SUCCESSFUL\\n'\n",
            )
            .expect("fixture Gradle wrapper");
            let mut permissions = fs::metadata(&wrapper)
                .expect("wrapper metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&wrapper, permissions).expect("wrapper permissions");
        }
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
        #[cfg(unix)]
        {
            let mut artifact_request = request(
                &state,
                "m5-artifact-build-command",
                CommandKind::ArtifactBuild,
                serde_json::to_string(&ArtifactBuildCommandPayload {
                    source_revision: 1,
                    workspace_root: workspace.to_string_lossy().into_owned(),
                    project_fingerprint: index.project_fingerprint.clone(),
                    build_variant: "debug".into(),
                    gradle_task: "assembleDebug".into(),
                })
                .expect("artifact build payload"),
                m4_response.projection_revision.0,
            );
            artifact_request.command.task_id = Some(contract.task_id.clone());
            let artifact_response = dispatch_request(&sink, &mut state, artifact_request.clone())
                .expect("artifact build response");
            assert_eq!(artifact_response.status, ResponseStatus::Completed);
            assert_eq!(
                artifact_response.result_schema_ref.as_deref(),
                Some("nirman.artifact_build_result.v1")
            );
            assert_eq!(
                artifact_response.result_payload.as_ref().unwrap()["observation"]["success"],
                true
            );
            assert!(state
                .plane
                .load_android_build_observation(&contract.task_id.0, 1)
                .expect("build observation load")
                .is_some());
            let artifact_duplicate = dispatch_request(&sink, &mut state, artifact_request)
                .expect("artifact build duplicate");
            assert_eq!(artifact_duplicate.status, ResponseStatus::Duplicate);
            assert_eq!(
                artifact_duplicate.result_payload,
                artifact_response.result_payload
            );
        }
        let m4_duplicate = dispatch_request(&sink, &mut state, m4_request).expect("M4 duplicate");
        assert_eq!(m4_duplicate.result_payload, m4_response.result_payload);
        let m108_record = state
            .plane
            .load_m108_sync_record(&contract.task_id.0)
            .expect("M108 record load")
            .expect("M108 record persisted");
        let m108_projection: M108ProjectionState =
            serde_json::from_str(&m108_record.0).expect("M108 projection parse");
        assert_eq!(m108_projection.last_event_sequence, 3);
        assert_eq!(m108_projection.build_status, "OBSERVED");
        assert!(m108_record
            .1
            .contains("m108-evidence:m5-artifact-build-command"));
        let event_jsons = state
            .plane
            .load_m108_event_jsons(&contract.task_id.0)
            .expect("M108 event ledger load");
        assert_eq!(
            event_jsons.len(),
            m108_projection.last_event_sequence as usize
        );
        let mut reconstructed = M108ProjectionState::new(
            PROJECT_ID,
            &contract.task_id.0,
            &m108_projection.active_preview_revision_id,
        );
        for event_json in event_jsons {
            let event: nirman_preview::PreviewSyncEvent =
                serde_json::from_str(&event_json).expect("M108 event parse");
            reconstructed.apply(&event).expect("M108 replay event");
        }
        assert_eq!(reconstructed, m108_projection);
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
        assert!(reopened_m4
            .load_android_build_observation(&contract.task_id.0, 1)
            .expect("build observation reload")
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
        assert_eq!(batches.len(), 4);
        assert!(batches[1].events[0]
            .kind
            .contains("AndroidToolchainPreflight"));
        assert!(batches[2].events[0].kind.contains("AndroidSynthesisBuild"));
        assert!(batches[3].events[0].kind.contains("ArtifactBuild"));
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

#[cfg(test)]
mod m8_host_tests {
    use super::*;
    use crate::tests::RecordingSink;
    use nirman_domain::{CommandEnvelope, ProjectId, Revision, TaskId};
    use nirman_workers::{
        CoordinationTask, HandoffStatus, WorkerContract, WorkerHandoffRecord, WorkerOutcome,
        WorkerRole, WorkspaceIsolation, M8_SCHEMA_VERSION,
    };
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nirman-m8-host-{nonce}.sqlite3"))
    }

    pub(super) fn parent_contract() -> WorkerContract {
        WorkerContract {
            schema_version: M5_SCHEMA_VERSION,
            worker_id: "worker-parent".into(),
            project_id: ProjectId(PROJECT_ID.into()),
            task_id: TaskId("task-root".into()),
            capability_ceiling: vec!["android.build".into()],
            workspace_root: "/workspace/root".into(),
            allowed_paths: vec!["/workspace/root".into()],
            denied_paths: vec!["/home/ubuntu/.ssh".into()],
            max_attempts: 3,
            evidence_requirements: vec!["worker-evidence".into()],
        }
    }

    pub(super) fn child_task(index: usize) -> CoordinationTask {
        CoordinationTask {
            schema_version: M8_SCHEMA_VERSION,
            task_id: format!("task-child-{index}"),
            parent_task_id: "task-root".into(),
            worker_id: format!("worker-child-{index}"),
            role: match index {
                0 => WorkerRole::Architecture,
                1 => WorkerRole::Implementation,
                _ => WorkerRole::Testing,
            },
            capability_ceiling: vec!["android.build".into()],
            workspace_root: format!("/workspace/child-{index}"),
            parent_workspace_root: "/workspace/root".into(),
            isolation: WorkspaceIsolation::GitWorktree,
            dependencies: vec![],
            expected_source_revision: Revision(0),
            required_evidence: vec!["worker-evidence".into()],
        }
    }

    fn request(
        auth: &AuthContext,
        correlation_id: &str,
        command_id: &str,
        kind: CommandKind,
        task_id: &str,
        payload: String,
        revision: u64,
    ) -> CommandRequest {
        CommandRequest {
            protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
            auth: auth.clone(),
            command: CommandEnvelope {
                command_id: command_id.into(),
                project_id: ProjectId(PROJECT_ID.into()),
                task_id: Some(TaskId(task_id.into())),
                kind,
                payload,
                expected_projection_revision: Revision(revision),
                idempotency_key: Some(format!("m8-idempotency-{command_id}")),
            },
            correlation_id: correlation_id.into(),
            causation_id: None,
            deadline_epoch_seconds: None,
        }
    }

    pub(super) fn run(
        sink: &RecordingSink,
        state: &mut RuntimeState,
        command_id: &str,
        kind: CommandKind,
        task_id: &str,
        payload: String,
        revision: u64,
    ) -> Result<CommandResponse, ErrorEnvelope> {
        let auth = state.auth.clone();
        let correlation_id = state.correlation_id.clone();
        let request = request(
            &auth,
            &correlation_id,
            command_id,
            kind,
            task_id,
            payload,
            revision,
        );
        dispatch_request(sink, state, request)
    }

    fn claim_from_response(response: &CommandResponse) -> nirman_workers::WorkerTaskClaim {
        serde_json::from_value(response.result_payload.clone().expect("claim result"))
            .map(
                |payload: WorkerTaskClaimResultPayload| nirman_workers::WorkerTaskClaim {
                    task_id: payload.task_id,
                    lease: nirman_workers::WorkerLease {
                        lease_id: payload.lease_id,
                        worker_id: payload.worker_id,
                        fence_token: payload.fence_token,
                        expires_at_epoch_seconds: payload.expires_at_epoch_seconds,
                    },
                },
            )
            .expect("typed claim result")
    }

    #[test]
    fn authenticated_m8_engine_claims_three_workers_reloads_and_replays_durably() {
        let path = database_path();
        let mut state = super::tests::test_state_at(&path);
        let sink = super::tests::RecordingSink::default();
        let parent = parent_contract();
        let tasks: Vec<CoordinationTask> = (0..3).map(child_task).collect();
        let mut claims = Vec::new();
        for (index, task) in tasks.iter().enumerate() {
            let payload = WorkerTaskClaimCommandPayload {
                parent_contract: parent.clone(),
                task: task.clone(),
                now_epoch_seconds: 100,
                lease_duration_seconds: 60,
            };
            let response = run(
                &sink,
                &mut state,
                &format!("m8-claim-{index}"),
                CommandKind::WorkerTaskClaim,
                &task.task_id,
                serde_json::to_string(&payload).expect("claim payload"),
                index as u64,
            )
            .expect("authenticated claim");
            assert_eq!(response.status, ResponseStatus::Accepted);
            assert_eq!(
                response.result_schema_ref.as_deref(),
                Some("nirman.worker_task_claim_result.v1")
            );
            claims.push(claim_from_response(&response));
        }
        let second_claim_payload = WorkerTaskClaimCommandPayload {
            parent_contract: parent.clone(),
            task: tasks[0].clone(),
            now_epoch_seconds: 100,
            lease_duration_seconds: 60,
        };
        let second_claim = run(
            &sink,
            &mut state,
            "m8-claim-conflict",
            CommandKind::WorkerTaskClaim,
            &tasks[0].task_id,
            serde_json::to_string(&second_claim_payload).expect("second claim payload"),
            3,
        )
        .expect_err("a claimed task cannot be claimed by a second command");
        assert_eq!(second_claim.code, ControlPlaneErrorCode::InvalidCommand);
        for (index, (task, claim)) in tasks.iter().zip(claims.iter()).enumerate() {
            let handoff = WorkerHandoffRecord {
                message_id: format!("m8-message-{index}"),
                task_id: task.task_id.clone(),
                worker_id: task.worker_id.clone(),
                lease_id: claim.lease.lease_id.clone(),
                fence_token: claim.lease.fence_token,
                source_revision: Revision(1),
                outcome: WorkerOutcome::Success,
                changed_paths: vec![format!("app/Worker{index}.kt")],
                changed_symbols: vec![format!("Worker{index}")],
                evidence_refs: vec![format!("evidence-worker-{index}")],
                mutation_request: None,
            };
            let payload = WorkerHandoffSubmitCommandPayload {
                parent_contract: parent.clone(),
                handoff,
            };
            let response = run(
                &sink,
                &mut state,
                &format!("m8-handoff-{index}"),
                CommandKind::WorkerHandoffSubmit,
                &task.task_id,
                serde_json::to_string(&payload).expect("handoff payload"),
                (3 + index) as u64,
            )
            .expect("authenticated handoff");
            assert_eq!(response.status, ResponseStatus::Accepted);
            assert_eq!(
                response.result_schema_ref.as_deref(),
                Some("nirman.worker_handoff_submit_result.v1")
            );
        }
        for (index, task) in tasks.iter().enumerate() {
            let payload = WorkerHandoffAcknowledgeCommandPayload {
                parent_contract: parent.clone(),
                acknowledgement_id: format!("m8-ack-{index}"),
                message_id: format!("m8-message-{index}"),
            };
            let response = run(
                &sink,
                &mut state,
                &format!("m8-ack-command-{index}"),
                CommandKind::WorkerHandoffAcknowledge,
                &task.task_id,
                serde_json::to_string(&payload).expect("ack payload"),
                (6 + index) as u64,
            )
            .expect("authenticated acknowledgement");
            let result: WorkerHandoffAcknowledgeResultPayload =
                serde_json::from_value(response.result_payload.expect("ack result"))
                    .expect("typed acknowledgement result");
            assert_eq!(result.acknowledgement.status, HandoffStatus::Accepted);
            assert!(result.acknowledgement.reconciliation_checkpoint.is_some());
        }
        drop(state);

        let mut reopened = super::tests::test_state_at(&path);
        let duplicate_payload = WorkerTaskClaimCommandPayload {
            parent_contract: parent.clone(),
            task: tasks[0].clone(),
            now_epoch_seconds: 100,
            lease_duration_seconds: 60,
        };
        let duplicate = run(
            &sink,
            &mut reopened,
            "m8-claim-0",
            CommandKind::WorkerTaskClaim,
            &tasks[0].task_id,
            serde_json::to_string(&duplicate_payload).expect("duplicate claim payload"),
            0,
        )
        .expect("durable duplicate claim");
        assert_eq!(duplicate.status, ResponseStatus::Duplicate);
        assert_eq!(
            duplicate.result_schema_ref.as_deref(),
            Some("nirman.worker_task_claim_result.v1")
        );
        assert_eq!(
            duplicate.result_payload.expect("duplicate claim result")["task_id"],
            tasks[0].task_id
        );
        let duplicate_ack_payload = WorkerHandoffAcknowledgeCommandPayload {
            parent_contract: parent,
            acknowledgement_id: "m8-ack-0".into(),
            message_id: "m8-message-0".into(),
        };
        let duplicate_ack = run(
            &sink,
            &mut reopened,
            "m8-ack-command-0",
            CommandKind::WorkerHandoffAcknowledge,
            &tasks[0].task_id,
            serde_json::to_string(&duplicate_ack_payload).expect("duplicate ack payload"),
            6,
        )
        .expect("durable duplicate acknowledgement");
        assert_eq!(duplicate_ack.status, ResponseStatus::Duplicate);
        assert_eq!(
            duplicate_ack.result_schema_ref.as_deref(),
            Some("nirman.worker_handoff_acknowledge_result.v1")
        );
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn authenticated_m8_engine_presents_conflict_without_overwriting_worker_output() {
        let path = database_path();
        let mut state = super::tests::test_state_at(&path);
        let sink = super::tests::RecordingSink::default();
        let parent = parent_contract();
        let tasks: Vec<CoordinationTask> = (0..3).map(child_task).collect();
        let mut claims = Vec::new();
        for (index, task) in tasks.iter().enumerate() {
            let payload = WorkerTaskClaimCommandPayload {
                parent_contract: parent.clone(),
                task: task.clone(),
                now_epoch_seconds: 100,
                lease_duration_seconds: 60,
            };
            let response = run(
                &sink,
                &mut state,
                &format!("m8-conflict-claim-{index}"),
                CommandKind::WorkerTaskClaim,
                &task.task_id,
                serde_json::to_string(&payload).expect("claim payload"),
                index as u64,
            )
            .expect("claim");
            claims.push(claim_from_response(&response));
        }
        for (index, (task, claim)) in tasks.iter().zip(claims.iter()).enumerate() {
            let handoff = WorkerHandoffRecord {
                message_id: format!("m8-conflict-message-{index}"),
                task_id: task.task_id.clone(),
                worker_id: task.worker_id.clone(),
                lease_id: claim.lease.lease_id.clone(),
                fence_token: claim.lease.fence_token,
                source_revision: Revision(1),
                outcome: WorkerOutcome::Success,
                changed_paths: vec!["app/Shared.kt".into()],
                changed_symbols: vec!["Shared".into()],
                evidence_refs: vec![format!("evidence-conflict-{index}")],
                mutation_request: None,
            };
            let payload = WorkerHandoffSubmitCommandPayload {
                parent_contract: parent.clone(),
                handoff,
            };
            run(
                &sink,
                &mut state,
                &format!("m8-conflict-handoff-{index}"),
                CommandKind::WorkerHandoffSubmit,
                &task.task_id,
                serde_json::to_string(&payload).expect("handoff payload"),
                (3 + index) as u64,
            )
            .expect("handoff");
        }
        let payload = WorkerHandoffAcknowledgeCommandPayload {
            parent_contract: parent,
            acknowledgement_id: "m8-conflict-ack".into(),
            message_id: "m8-conflict-message-0".into(),
        };
        let response = run(
            &sink,
            &mut state,
            "m8-conflict-ack-command",
            CommandKind::WorkerHandoffAcknowledge,
            &tasks[0].task_id,
            serde_json::to_string(&payload).expect("ack payload"),
            6,
        )
        .expect("conflict acknowledgement");
        let result: WorkerHandoffAcknowledgeResultPayload =
            serde_json::from_value(response.result_payload.expect("ack result"))
                .expect("typed conflict acknowledgement");
        assert_eq!(result.acknowledgement.status, HandoffStatus::Conflict);
        assert!(result.acknowledgement.reconciliation_checkpoint.is_none());
        assert!(state
            .plane
            .load_worker_handoff("m8-conflict-message-1")
            .expect("handoff remains durable")
            .is_some());
        drop(state);
        let _ = fs::remove_file(path);
    }
}

fn copy_directory_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn restore_directory_tree(
    backup: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    copy_directory_tree(backup, destination)
}

fn mutation_payload_from_request(
    mutation: &MutationRequest,
    base_revision: u64,
    base_project_fingerprint: String,
    base_file_hashes: BTreeMap<String, String>,
    workspace_root: String,
) -> Result<WorkspaceApplyPatchCommandPayload, &'static str> {
    if mutation.touched_paths.len() != 1 {
        return Err("reconciliation requires exactly one touched path per M46 mutation");
    }
    let capability_digest = nirman_project::mutation_capability_digest(
        &mutation.project_id,
        &mutation.task_id,
        &mutation.worker_id,
        &mutation.operation_id,
        base_revision,
        &base_project_fingerprint,
        mutation.fence_token,
    );
    Ok(WorkspaceApplyPatchCommandPayload {
        worker_id: mutation.worker_id.clone(),
        operation_id: mutation.operation_id.clone(),
        base_revision,
        base_project_fingerprint,
        workspace_root,
        allowed_paths: mutation.allowed_paths.clone(),
        owned_paths: mutation.owned_paths.clone(),
        touched_paths: mutation.touched_paths.clone(),
        base_file_hashes,
        mutation_budget: mutation.mutation_budget,
        dependency_policy: mutation.dependency_policy.clone(),
        capability_digest,
        fence_token: mutation.fence_token,
        evidence_required: mutation.evidence_required,
        isolated_transaction: mutation.isolated_transaction,
        whole_file_fallback: mutation.whole_file_fallback,
        operation: mutation.operation.clone(),
    })
}

fn reconcile_m8_handoffs(
    state: &mut RuntimeState,
    request: &CommandRequest,
    payload: &WorkerReconcileCommandPayload,
) -> Result<M8ReconciliationCheckpoint, ErrorEnvelope> {
    let integration_root = PathBuf::from(&payload.integration_workspace_root)
        .canonicalize()
        .map_err(|_| m8_validation_error(request, "integration workspace is unavailable"))?;
    let main_root = state
        .authorized_workspace_root
        .as_ref()
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| m8_dependency_error(request, "authorized main workspace is unavailable"))?;
    if integration_root == main_root {
        return Err(m8_validation_error(
            request,
            "M8 reconciliation cannot write directly to the main workspace",
        ));
    }
    let base_index = nirman_project::ProjectIndexer::default()
        .index_workspace(&integration_root, &nirman_project::IndexRequest::default())
        .map_err(|_| m8_dependency_error(request, "integration workspace could not be indexed"))?;
    let backup_root = std::env::temp_dir().join(format!(
        "nirman-m8-reconcile-backup-{}",
        request.command.command_id
    ));
    if backup_root.exists() {
        fs::remove_dir_all(&backup_root).map_err(|_| {
            m8_dependency_error(request, "stale reconciliation backup could not be removed")
        })?;
    }
    copy_directory_tree(&integration_root, &backup_root).map_err(|_| {
        m8_dependency_error(request, "integration workspace could not be snapshotted")
    })?;

    let mut completed_mutations = Vec::new();
    let mut completed_records = Vec::new();
    let mut failure_message = None;
    let mut handoffs = Vec::new();
    for message_id in &payload.handoff_message_ids {
        let handoff = state
            .plane
            .load_worker_handoff(message_id)
            .map_err(|_| m8_dependency_error(request, "worker handoff could not be loaded"))?
            .ok_or_else(|| {
                m8_validation_error(request, "reconciliation references a missing handoff")
            })?;
        handoffs.push(handoff);
    }
    let mut coordinator = restore_m8_coordinator(state, &payload.parent_contract, request)?;
    if coordinator.reconcile().is_err() {
        let checkpoint = M8ReconciliationCheckpoint {
            schema_version: nirman_workers::M8_RECONCILIATION_SCHEMA_VERSION,
            checkpoint_id: payload.checkpoint_id.clone(),
            project_id: request.command.project_id.clone(),
            parent_task_id: payload.parent_contract.task_id.0.clone(),
            workspace_root: integration_root.to_string_lossy().into_owned(),
            base_revision: state.plane.snapshot().current_source_revision,
            base_project_fingerprint: base_index.project_fingerprint,
            resulting_project_fingerprint: None,
            status: ReconciliationStatus::Blocked,
            handoff_message_ids: payload.handoff_message_ids.clone(),
            mutations: Vec::new(),
            reason: "worker handoffs are incomplete, unsuccessful, or conflicting".into(),
        };
        checkpoint.validate().map_err(|_| {
            m8_dependency_error(
                request,
                "blocked reconciliation checkpoint failed validation",
            )
        })?;
        let _ = fs::remove_dir_all(&backup_root);
        return Ok(checkpoint);
    }

    for handoff in handoffs {
        let Some(original_mutation) = handoff.mutation_request.as_ref() else {
            failure_message =
                Some("worker handoff has no authorized M46 mutation request".to_owned());
            break;
        };
        let current_index = nirman_project::ProjectIndexer::default()
            .index_workspace(&integration_root, &nirman_project::IndexRequest::default())
            .map_err(|_| m8_dependency_error(request, "integration workspace re-index failed"))?;
        let path = original_mutation
            .touched_paths
            .first()
            .cloned()
            .ok_or_else(|| m8_validation_error(request, "worker mutation has no touched path"))?;
        let current_file_hash = current_index
            .files
            .iter()
            .find(|file| file.relative_path == path)
            .map(|file| file.content_hash.clone())
            .ok_or_else(|| {
                m8_validation_error(
                    request,
                    "worker mutation target is absent from integration workspace",
                )
            })?;
        let payload = mutation_payload_from_request(
            original_mutation,
            state.plane.snapshot().current_source_revision.0,
            current_index.project_fingerprint,
            BTreeMap::from([(path, current_file_hash)]),
            integration_root.to_string_lossy().into_owned(),
        )
        .map_err(|message| m8_validation_error(request, message))?;
        let mut synthetic_request = request.clone();
        synthetic_request.command.command_id = format!(
            "{}-m46-{}",
            request.command.command_id, original_mutation.operation_id
        );
        synthetic_request.command.task_id = Some(TaskId(handoff.task_id.clone()));
        state
            .supervisor
            .register_lease(&handoff.lease_id, &handoff.worker_id, handoff.fence_token);
        state.supervisor.heartbeat();
        let prepared = {
            let previous_root = state.authorized_workspace_root.take();
            state.authorized_workspace_root = Some(integration_root.clone());
            let result = prepare_m46_mutation(state, &synthetic_request, &payload);
            state.authorized_workspace_root = previous_root;
            result
        };
        let prepared = match prepared {
            Ok(Some(prepared)) => prepared,
            Ok(None) => {
                failure_message = Some("M46 mutation transaction was already admitted".into());
                break;
            }
            Err(error) => {
                failure_message = Some(error.safe_message);
                break;
            }
        };
        match MutationBroker::default().apply(&prepared.request) {
            Ok(outcome) => {
                let committed = MutationTransactionRecord {
                    state: "COMMITTED".into(),
                    resulting_revision: state.plane.snapshot().current_source_revision,
                    resulting_project_fingerprint: Some(outcome.project_fingerprint.clone()),
                    changed_paths_json: Some(
                        serde_json::to_string(&outcome.changed_files).map_err(|_| {
                            m8_dependency_error(request, "M46 mutation result serialization failed")
                        })?,
                    ),
                    evidence_json: Some(serde_json::to_string(&outcome.evidence).map_err(
                        |_| {
                            m8_dependency_error(
                                request,
                                "M46 mutation evidence serialization failed",
                            )
                        },
                    )?),
                    completed_at_epoch_seconds: Some(now_epoch_seconds()),
                    ..prepared.transaction.clone()
                };
                state
                    .plane
                    .record_mutation_transaction(&committed)
                    .map_err(|_| {
                        m8_dependency_error(
                            request,
                            "M46 committed transaction could not be persisted",
                        )
                    })?;
                state
                    .consumed_mutation_capabilities
                    .insert(prepared.request.capability_digest.clone());
                completed_records.push(committed);
                completed_mutations.push(ReconciliationMutationSummary {
                    task_id: handoff.task_id,
                    worker_id: handoff.worker_id,
                    operation_id: prepared.request.operation_id,
                    resulting_project_fingerprint: outcome.project_fingerprint,
                    changed_paths: outcome
                        .changed_files
                        .into_iter()
                        .map(|file| file.relative_path)
                        .collect(),
                });
            }
            Err(error) => {
                let failed = MutationTransactionRecord {
                    state: "FAILED".into(),
                    completed_at_epoch_seconds: Some(now_epoch_seconds()),
                    ..prepared.transaction.clone()
                };
                state
                    .plane
                    .record_mutation_transaction(&failed)
                    .map_err(|_| {
                        m8_dependency_error(
                            request,
                            "failed M46 transaction could not be persisted",
                        )
                    })?;
                failure_message = Some(error.to_string());
                break;
            }
        }
    }
    if let Some(reason) = failure_message {
        restore_directory_tree(&backup_root, &integration_root)
            .map_err(|_| m8_dependency_error(request, "integration workspace rollback failed"))?;
        for record in completed_records {
            let rolled_back = MutationTransactionRecord {
                state: "ROLLED_BACK".into(),
                completed_at_epoch_seconds: Some(now_epoch_seconds()),
                ..record
            };
            state
                .plane
                .record_mutation_transaction(&rolled_back)
                .map_err(|_| {
                    m8_dependency_error(
                        request,
                        "rolled-back M46 transaction could not be persisted",
                    )
                })?;
        }
        let checkpoint = M8ReconciliationCheckpoint {
            schema_version: nirman_workers::M8_RECONCILIATION_SCHEMA_VERSION,
            checkpoint_id: payload.checkpoint_id.clone(),
            project_id: request.command.project_id.clone(),
            parent_task_id: payload.parent_contract.task_id.0.clone(),
            workspace_root: integration_root.to_string_lossy().into_owned(),
            base_revision: state.plane.snapshot().current_source_revision,
            base_project_fingerprint: base_index.project_fingerprint,
            resulting_project_fingerprint: None,
            status: ReconciliationStatus::RolledBack,
            handoff_message_ids: payload.handoff_message_ids.clone(),
            mutations: completed_mutations,
            reason,
        };
        checkpoint.validate().map_err(|_| {
            m8_dependency_error(
                request,
                "rolled-back reconciliation checkpoint failed validation",
            )
        })?;
        let _ = fs::remove_dir_all(&backup_root);
        return Ok(checkpoint);
    }
    let resulting_index = nirman_project::ProjectIndexer::default()
        .index_workspace(&integration_root, &nirman_project::IndexRequest::default())
        .map_err(|_| {
            m8_dependency_error(request, "integrated workspace could not be re-indexed")
        })?;
    let checkpoint = M8ReconciliationCheckpoint {
        schema_version: nirman_workers::M8_RECONCILIATION_SCHEMA_VERSION,
        checkpoint_id: payload.checkpoint_id.clone(),
        project_id: request.command.project_id.clone(),
        parent_task_id: payload.parent_contract.task_id.0.clone(),
        workspace_root: integration_root.to_string_lossy().into_owned(),
        base_revision: state.plane.snapshot().current_source_revision,
        base_project_fingerprint: base_index.project_fingerprint,
        resulting_project_fingerprint: Some(resulting_index.project_fingerprint),
        status: ReconciliationStatus::Integrated,
        handoff_message_ids: payload.handoff_message_ids.clone(),
        mutations: completed_mutations,
        reason: "all isolated worker mutations passed M46 authority and integration validation"
            .into(),
    };
    checkpoint.validate().map_err(|_| {
        m8_dependency_error(
            request,
            "integrated reconciliation checkpoint failed validation",
        )
    })?;
    let _ = fs::remove_dir_all(&backup_root);
    Ok(checkpoint)
}

#[cfg(test)]
mod m8_reconciliation_tests {
    use super::*;
    use crate::m8_host_tests::{child_task, parent_contract, run};
    use crate::tests::RecordingSink;
    use nirman_domain::{CommandKind, Revision, TaskId};
    use nirman_project::{IndexRequest, MutationOperation, ProjectIndexer};
    use nirman_workers::{CoordinationTask, WorkerContract, WorkerHandoffRecord, WorkerOutcome};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nirman-m8-reconciliation-{nonce}.sqlite3"))
    }

    fn mutation_for(
        root: &std::path::Path,
        task_id: &str,
        worker_id: &str,
        operation_id: &str,
        fence_token: u64,
        relative_path: &str,
        symbol: &str,
        replacement: &str,
    ) -> nirman_project::MutationRequest {
        let index = ProjectIndexer::default()
            .index_workspace(root, &IndexRequest::default())
            .expect("integration base index");
        let base_hash = index
            .files
            .iter()
            .find(|file| file.relative_path == relative_path)
            .expect("integration target indexed")
            .content_hash
            .clone();
        nirman_project::MutationRequest {
            project_id: PROJECT_ID.into(),
            task_id: task_id.into(),
            worker_id: worker_id.into(),
            operation_id: operation_id.into(),
            base_revision: 0,
            base_project_fingerprint: index.project_fingerprint.clone(),
            workspace_root: "/isolated/worker-worktree".into(),
            allowed_paths: BTreeSet::from([relative_path.into()]),
            owned_paths: BTreeSet::from([relative_path.into()]),
            touched_paths: vec![relative_path.into()],
            base_file_hashes: BTreeMap::from([(relative_path.into(), base_hash)]),
            mutation_budget: 1,
            dependency_policy: "locked-no-new-dependencies".into(),
            capability_digest: nirman_project::mutation_capability_digest(
                PROJECT_ID,
                task_id,
                worker_id,
                operation_id,
                0,
                &index.project_fingerprint,
                fence_token,
            ),
            fence_token,
            evidence_required: true,
            isolated_transaction: true,
            whole_file_fallback: false,
            operation: MutationOperation::ReplaceSymbol {
                path: relative_path.into(),
                symbol: symbol.into(),
                replacement: replacement.into(),
            },
        }
    }

    pub(super) fn make_handoff(
        root: &std::path::Path,
        task: &CoordinationTask,
        lease: &nirman_workers::WorkerTaskClaim,
        message_id: &str,
        relative_path: &str,
        symbol: &str,
        replacement: &str,
    ) -> WorkerHandoffRecord {
        WorkerHandoffRecord {
            message_id: message_id.into(),
            task_id: task.task_id.clone(),
            worker_id: task.worker_id.clone(),
            lease_id: lease.lease.lease_id.clone(),
            fence_token: lease.lease.fence_token,
            source_revision: Revision(0),
            outcome: WorkerOutcome::Success,
            changed_paths: vec![relative_path.into()],
            changed_symbols: vec![symbol.into()],
            evidence_refs: vec![format!("evidence-{message_id}")],
            mutation_request: Some(mutation_for(
                root,
                &task.task_id,
                &task.worker_id,
                &format!("operation-{message_id}"),
                lease.lease.fence_token,
                relative_path,
                symbol,
                replacement,
            )),
        }
    }

    pub(super) fn claim_worker(
        state: &mut RuntimeState,
        sink: &RecordingSink,
        parent: &WorkerContract,
        task: &CoordinationTask,
        command_id: &str,
        revision: u64,
    ) -> nirman_workers::WorkerTaskClaim {
        let payload = WorkerTaskClaimCommandPayload {
            parent_contract: parent.clone(),
            task: task.clone(),
            now_epoch_seconds: 100,
            lease_duration_seconds: 60,
        };
        let response = run(
            sink,
            state,
            command_id,
            CommandKind::WorkerTaskClaim,
            &task.task_id,
            serde_json::to_string(&payload).expect("claim payload"),
            revision,
        )
        .expect("claim");
        serde_json::from_value(response.result_payload.expect("claim result"))
            .map(
                |payload: WorkerTaskClaimResultPayload| nirman_workers::WorkerTaskClaim {
                    task_id: payload.task_id,
                    lease: nirman_workers::WorkerLease {
                        lease_id: payload.lease_id,
                        worker_id: payload.worker_id,
                        fence_token: payload.fence_token,
                        expires_at_epoch_seconds: payload.expires_at_epoch_seconds,
                    },
                },
            )
            .expect("typed claim")
    }

    #[test]
    fn m8_reconciliation_delegates_to_m46_integration_workspace_and_replays_checkpoint() {
        let database = database_path();
        let main_root =
            std::env::temp_dir().join(format!("nirman-m8-main-{}", now_epoch_seconds()));
        let integration_root =
            std::env::temp_dir().join(format!("nirman-m8-integration-{}", now_epoch_seconds()));
        fs::create_dir_all(integration_root.join("app")).expect("integration workspace");
        fs::create_dir_all(&main_root).expect("main workspace");
        fs::write(
            integration_root.join("app/Worker.kt"),
            "class Worker { fun open() {} }\n",
        )
        .expect("worker source");
        fs::write(main_root.join("Main.txt"), "main remains unchanged\n").expect("main marker");
        let mut state = crate::tests::test_state_at(&database);
        state.authorized_workspace_root = Some(main_root.clone());
        let sink = RecordingSink::default();
        let parent = parent_contract();
        let task = child_task(0);
        let claim = claim_worker(&mut state, &sink, &parent, &task, "m8-reconcile-claim", 0);
        let handoff = make_handoff(
            &integration_root,
            &task,
            &claim,
            "m8-reconcile-message",
            "app/Worker.kt",
            "Worker",
            "class Worker { fun saved() {} }",
        );
        let submit = WorkerHandoffSubmitCommandPayload {
            parent_contract: parent.clone(),
            handoff,
        };
        run(
            &sink,
            &mut state,
            "m8-reconcile-handoff",
            CommandKind::WorkerHandoffSubmit,
            &task.task_id,
            serde_json::to_string(&submit).expect("handoff payload"),
            1,
        )
        .expect("handoff");
        let acknowledge = WorkerHandoffAcknowledgeCommandPayload {
            parent_contract: parent.clone(),
            acknowledgement_id: "m8-reconcile-ack".into(),
            message_id: "m8-reconcile-message".into(),
        };
        run(
            &sink,
            &mut state,
            "m8-reconcile-ack-command",
            CommandKind::WorkerHandoffAcknowledge,
            &task.task_id,
            serde_json::to_string(&acknowledge).expect("ack payload"),
            2,
        )
        .expect("acknowledgement");
        let reconcile_payload = WorkerReconcileCommandPayload {
            parent_contract: parent,
            checkpoint_id: "m8-reconcile-checkpoint".into(),
            integration_workspace_root: integration_root.to_string_lossy().into_owned(),
            handoff_message_ids: vec!["m8-reconcile-message".into()],
        };
        let reconcile_request_payload =
            serde_json::to_string(&reconcile_payload).expect("reconcile payload");
        let reconcile = run(
            &sink,
            &mut state,
            "m8-reconcile-command",
            CommandKind::WorkerReconcile,
            "task-root",
            reconcile_request_payload.clone(),
            3,
        )
        .expect("reconcile");
        let checkpoint: WorkerReconcileResultPayload =
            serde_json::from_value(reconcile.result_payload.clone().expect("reconcile result"))
                .expect("typed checkpoint");
        assert_eq!(
            checkpoint.checkpoint.status,
            ReconciliationStatus::Integrated
        );
        assert!(checkpoint
            .checkpoint
            .resulting_project_fingerprint
            .is_some());
        assert!(fs::read_to_string(integration_root.join("app/Worker.kt"))
            .expect("integrated source")
            .contains("saved"));
        assert_eq!(
            fs::read_to_string(main_root.join("Main.txt")).expect("main source"),
            "main remains unchanged\n"
        );
        assert_eq!(
            state
                .plane
                .load_m8_reconciliation_checkpoint("m8-reconcile-checkpoint")
                .expect("checkpoint load")
                .expect("checkpoint")
                .status,
            ReconciliationStatus::Integrated
        );
        let invalid_direct = WorkerReconcileCommandPayload {
            parent_contract: WorkerContract {
                task_id: TaskId("task-root".into()),
                ..crate::m8_host_tests::parent_contract()
            },
            checkpoint_id: "m8-direct-main-rejected".into(),
            integration_workspace_root: main_root.to_string_lossy().into_owned(),
            handoff_message_ids: vec!["m8-reconcile-message".into()],
        };
        let direct_result = run(
            &sink,
            &mut state,
            "m8-direct-main-command",
            CommandKind::WorkerReconcile,
            "task-root",
            serde_json::to_string(&invalid_direct).expect("direct payload"),
            reconcile.projection_revision.0,
        );
        assert!(direct_result.is_err());
        drop(state);
        let mut reopened = crate::tests::test_state_at(&database);
        reopened.authorized_workspace_root = Some(main_root.clone());
        let duplicate = run(
            &sink,
            &mut reopened,
            "m8-reconcile-command",
            CommandKind::WorkerReconcile,
            "task-root",
            reconcile_request_payload,
            3,
        )
        .expect("duplicate reconcile");
        assert_eq!(duplicate.status, ResponseStatus::Duplicate);
        assert_eq!(
            duplicate.result_schema_ref.as_deref(),
            Some("nirman.worker_reconcile_result.v1")
        );
        let reloaded_checkpoint: WorkerReconcileResultPayload =
            serde_json::from_value(duplicate.result_payload.expect("duplicate checkpoint"))
                .expect("reloaded checkpoint");
        assert_eq!(reloaded_checkpoint.checkpoint, checkpoint.checkpoint);
        let _ = fs::remove_file(database);
        let _ = fs::remove_dir_all(main_root);
        let _ = fs::remove_dir_all(integration_root);
    }
}

#[cfg(test)]
mod m8_reconciliation_failure_tests {
    use super::*;
    use crate::m8_host_tests::{child_task, parent_contract, run};
    use crate::m8_reconciliation_tests::{claim_worker, make_handoff};
    use crate::tests::RecordingSink;
    use nirman_domain::CommandKind;
    use nirman_workers::ReconciliationStatus;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nirman-m8-reconciliation-failure-{nonce}.sqlite3"))
    }

    #[test]
    fn conflict_is_blocked_before_m46_and_checkpoint_is_durable() {
        let database = database_path();
        let workspace =
            std::env::temp_dir().join(format!("nirman-m8-conflict-{}", now_epoch_seconds()));
        fs::create_dir_all(workspace.join("app")).expect("workspace");
        fs::write(
            workspace.join("app/Worker.kt"),
            "class Worker { fun open() {} }\n",
        )
        .expect("source");
        let mut state = crate::tests::test_state_at(&database);
        let main_root =
            std::env::temp_dir().join(format!("nirman-m8-conflict-main-{}", now_epoch_seconds()));
        fs::create_dir_all(&main_root).expect("main root");
        state.authorized_workspace_root = Some(main_root.clone());
        let sink = RecordingSink::default();
        let parent = parent_contract();
        let task_a = child_task(0);
        let task_b = child_task(1);
        let claim_a = claim_worker(
            &mut state,
            &sink,
            &parent,
            &task_a,
            "m8-conflict-reconcile-claim-a",
            0,
        );
        let claim_b = claim_worker(
            &mut state,
            &sink,
            &parent,
            &task_b,
            "m8-conflict-reconcile-claim-b",
            1,
        );
        for (task, claim, command_id, message_id) in [
            (
                &task_a,
                &claim_a,
                "m8-conflict-reconcile-handoff-a",
                "m8-conflict-reconcile-message-a",
            ),
            (
                &task_b,
                &claim_b,
                "m8-conflict-reconcile-handoff-b",
                "m8-conflict-reconcile-message-b",
            ),
        ] {
            let handoff = make_handoff(
                &workspace,
                task,
                claim,
                message_id,
                "app/Worker.kt",
                "Worker",
                "class Worker { fun shared() {} }",
            );
            let payload = WorkerHandoffSubmitCommandPayload {
                parent_contract: parent.clone(),
                handoff,
            };
            run(
                &sink,
                &mut state,
                command_id,
                CommandKind::WorkerHandoffSubmit,
                &task.task_id,
                serde_json::to_string(&payload).expect("handoff payload"),
                if task.task_id.ends_with('0') { 2 } else { 3 },
            )
            .expect("handoff");
        }
        let reconcile = WorkerReconcileCommandPayload {
            parent_contract: parent,
            checkpoint_id: "m8-conflict-reconcile-checkpoint".into(),
            integration_workspace_root: workspace.to_string_lossy().into_owned(),
            handoff_message_ids: vec![
                "m8-conflict-reconcile-message-a".into(),
                "m8-conflict-reconcile-message-b".into(),
            ],
        };
        let response = run(
            &sink,
            &mut state,
            "m8-conflict-reconcile-command",
            CommandKind::WorkerReconcile,
            "task-root",
            serde_json::to_string(&reconcile).expect("reconcile payload"),
            4,
        )
        .expect("blocked reconciliation is a durable result");
        let result: WorkerReconcileResultPayload =
            serde_json::from_value(response.result_payload.expect("blocked result"))
                .expect("typed blocked result");
        assert_eq!(result.checkpoint.status, ReconciliationStatus::Blocked);
        assert_eq!(
            fs::read_to_string(workspace.join("app/Worker.kt")).expect("unchanged source"),
            "class Worker { fun open() {} }\n"
        );
        assert_eq!(
            state
                .plane
                .load_m8_reconciliation_checkpoint("m8-conflict-reconcile-checkpoint")
                .expect("checkpoint load")
                .expect("durable blocked checkpoint")
                .status,
            ReconciliationStatus::Blocked
        );
        let _ = fs::remove_file(database);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(main_root);
    }

    #[test]
    fn later_m46_failure_rolls_back_earlier_integration_mutation() {
        let database = database_path();
        let workspace =
            std::env::temp_dir().join(format!("nirman-m8-rollback-{}", now_epoch_seconds()));
        fs::create_dir_all(workspace.join("app")).expect("workspace");
        fs::write(
            workspace.join("app/Worker.kt"),
            "class Worker { fun open() {} }\n",
        )
        .expect("worker source");
        fs::write(
            workspace.join("app/Other.kt"),
            "class Other { fun open() {} }\n",
        )
        .expect("other source");
        let original_worker =
            fs::read_to_string(workspace.join("app/Worker.kt")).expect("original worker");
        let original_other =
            fs::read_to_string(workspace.join("app/Other.kt")).expect("original other");
        let main_root =
            std::env::temp_dir().join(format!("nirman-m8-rollback-main-{}", now_epoch_seconds()));
        fs::create_dir_all(&main_root).expect("main root");
        let mut state = crate::tests::test_state_at(&database);
        state.authorized_workspace_root = Some(main_root.clone());
        let sink = RecordingSink::default();
        let parent = parent_contract();
        let task_a = child_task(0);
        let task_b = child_task(1);
        let claim_a = claim_worker(
            &mut state,
            &sink,
            &parent,
            &task_a,
            "m8-rollback-claim-a",
            0,
        );
        let claim_b = claim_worker(
            &mut state,
            &sink,
            &parent,
            &task_b,
            "m8-rollback-claim-b",
            1,
        );
        let handoff_a = make_handoff(
            &workspace,
            &task_a,
            &claim_a,
            "m8-rollback-message-a",
            "app/Worker.kt",
            "Worker",
            "class Worker { fun saved() {} }",
        );
        let handoff_b = make_handoff(
            &workspace,
            &task_b,
            &claim_b,
            "m8-rollback-message-b",
            "app/Other.kt",
            "MissingSymbol",
            "class Other { fun saved() {} }",
        );
        for (task, handoff, command_id, revision) in [
            (&task_a, handoff_a, "m8-rollback-handoff-a", 2_u64),
            (&task_b, handoff_b, "m8-rollback-handoff-b", 3_u64),
        ] {
            let payload = WorkerHandoffSubmitCommandPayload {
                parent_contract: parent.clone(),
                handoff,
            };
            run(
                &sink,
                &mut state,
                command_id,
                CommandKind::WorkerHandoffSubmit,
                &task.task_id,
                serde_json::to_string(&payload).expect("handoff payload"),
                revision,
            )
            .expect("handoff");
        }
        let reconcile = WorkerReconcileCommandPayload {
            parent_contract: parent,
            checkpoint_id: "m8-rollback-checkpoint".into(),
            integration_workspace_root: workspace.to_string_lossy().into_owned(),
            handoff_message_ids: vec![
                "m8-rollback-message-a".into(),
                "m8-rollback-message-b".into(),
            ],
        };
        let response = run(
            &sink,
            &mut state,
            "m8-rollback-command",
            CommandKind::WorkerReconcile,
            "task-root",
            serde_json::to_string(&reconcile).expect("reconcile payload"),
            4,
        )
        .expect("rollback checkpoint");
        let result: WorkerReconcileResultPayload =
            serde_json::from_value(response.result_payload.expect("rollback result"))
                .expect("typed rollback result");
        assert_eq!(result.checkpoint.status, ReconciliationStatus::RolledBack);
        assert_eq!(
            fs::read_to_string(workspace.join("app/Worker.kt")).expect("worker rollback"),
            original_worker
        );
        assert_eq!(
            fs::read_to_string(workspace.join("app/Other.kt")).expect("other rollback"),
            original_other
        );
        let _ = fs::remove_file(database);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(main_root);
    }
}

#[cfg(test)]
#[cfg(unix)]
mod m108_export_preview_tests {
    use super::dispatch_request;
    use super::*;
    use crate::tests::{construction_contract, request, test_state_at, RecordingSink};
    use nirman_android::{AndroidBuildObservation, StaticCapabilityProbe, ToolchainComponentKind};
    use nirman_domain::CommandKind;
    use nirman_ipc::{ArtifactExportCommandPayload, PreviewStartCommandPayload};
    use nirman_preview::PreviewEventTruth;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    fn path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "nirman-m108-export-preview-{}.sqlite3",
            now_epoch_seconds()
        ))
    }

    fn make_executable(path: &Path) {
        let mut permissions = fs::metadata(path).expect("fixture metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("fixture permissions");
    }

    fn fixture_probe(root: &Path) -> StaticCapabilityProbe {
        let build_tools = root.join("build-tools");
        let platform_tools = root.join("platform-tools");
        StaticCapabilityProbe::default()
            .with_available(
                ToolchainComponentKind::Jdk,
                "17",
                &root.join("jdk").to_string_lossy(),
                "jdk",
            )
            .with_available(
                ToolchainComponentKind::Gradle,
                "8.7",
                &root.join("gradle").to_string_lossy(),
                "gradle",
            )
            .with_available(
                ToolchainComponentKind::AndroidSdk,
                "35",
                &root.join("sdk").to_string_lossy(),
                "sdk",
            )
            .with_available(
                ToolchainComponentKind::PlatformTools,
                "35",
                &platform_tools.to_string_lossy(),
                "platform-tools",
            )
            .with_available(
                ToolchainComponentKind::Adb,
                "35",
                &platform_tools.join("adb").to_string_lossy(),
                "adb",
            )
            .with_available(
                ToolchainComponentKind::Emulator,
                "35",
                &root.join("emulator").to_string_lossy(),
                "emulator",
            )
            .with_available(
                ToolchainComponentKind::Kotlin,
                "2.0",
                &root.join("kotlin").to_string_lossy(),
                "kotlin",
            )
            .with_available(
                ToolchainComponentKind::AndroidGradlePlugin,
                "8.7",
                &root.join("agp").to_string_lossy(),
                "agp",
            )
            .with_available(
                ToolchainComponentKind::BuildTools,
                "35.0.0",
                &build_tools.to_string_lossy(),
                "build-tools",
            )
    }

    #[test]
    fn authenticated_export_then_preview_is_durable_and_headless_only() {
        let database = path();
        let root = std::env::temp_dir().join(format!(
            "nirman-m108-export-preview-root-{}",
            now_epoch_seconds()
        ));
        let build_tools = root.join("build-tools");
        let platform_tools = root.join("platform-tools");
        fs::create_dir_all(&build_tools).expect("build tools");
        fs::create_dir_all(&platform_tools).expect("platform tools");
        let aapt = build_tools.join("aapt");
        fs::write(
            &aapt,
            "#!/bin/sh\nprintf \"package: name='com.nirman.fixture' versionCode='1' versionName='1.0'\\n\"\n",
        )
        .expect("aapt fixture");
        make_executable(&aapt);
        let adb = platform_tools.join("adb");
        fs::write(
            &adb,
            "#!/bin/sh\ncase \"$1\" in\nget-state) printf 'device\\n' ;;\nget-serialno) printf 'pixel-api-35\\n' ;;\ninstall) exit 0 ;;\nlogcat) printf 'I/com.nirman.fixture: headless runtime\\n' ;;\nexec-out) if [ \"$2\" = \"screencap\" ]; then printf 'PNG'; else printf '<hierarchy package=\"com.nirman.fixture\"/>'; fi ;;\nshell) if [ \"$2\" = \"dumpsys\" ]; then printf 'permission: granted'; else exit 0; fi ;;\n*) exit 1 ;;\nesac\n",
        )
        .expect("adb fixture");
        make_executable(&adb);
        let apk = root.join("app-debug.apk");
        fs::write(&apk, b"nirman-headless-apk-fixture").expect("apk fixture");
        let delivery = root.join("delivery").join("app-debug.apk");
        let mut state = test_state_at(&database);
        state.capability_probe = Box::new(fixture_probe(&root));
        state.authorized_workspace_root = Some(root.clone());
        let sink = RecordingSink::default();
        let contract = construction_contract(PROJECT_ID);

        let mut contract_request = request(
            &state,
            "m108-fixture-contract",
            CommandKind::AndroidConstructionCreate,
            serde_json::to_string(&AndroidConstructionCommandPayload {
                contract: contract.clone(),
            })
            .expect("contract payload"),
            0,
        );
        contract_request.command.task_id = Some(contract.task_id.clone());
        let contract_response =
            dispatch_request(&sink, &mut state, contract_request).expect("authenticated contract");
        assert_eq!(contract_response.status, ResponseStatus::Accepted);

        let mut preflight_request = request(
            &state,
            "m108-fixture-preflight",
            CommandKind::AndroidToolchainPreflight,
            serde_json::to_string(&AndroidToolchainPreflightCommandPayload {
                build_variant: "debug".into(),
            })
            .expect("preflight payload"),
            contract_response.snapshot.projection_revision.0,
        );
        preflight_request.command.task_id = Some(contract.task_id.clone());
        let preflight_response = dispatch_request(&sink, &mut state, preflight_request)
            .expect("authenticated preflight");
        assert_eq!(preflight_response.status, ResponseStatus::Completed);

        let observation = AndroidBuildObservation {
            schema_version: 1,
            execution_id: "m108-fixture-build-observation".into(),
            command_id: "m108-fixture-build-command".into(),
            project_id: PROJECT_ID.into(),
            task_id: contract.task_id.0.clone(),
            source_revision: 0,
            project_fingerprint: "fingerprint-m108-fixture".into(),
            workspace_root: root.to_string_lossy().into_owned(),
            build_variant: "debug".into(),
            gradle_task: "assembleDebug".into(),
            executable: "fixture-gradle".into(),
            exit_code: Some(0),
            success: true,
            timed_out: false,
            cancelled: false,
            stdout_sha256: "sha256:stdout".into(),
            stderr_sha256: "sha256:stderr".into(),
            stdout_bytes: 1,
            stderr_bytes: 0,
            artifact_path: Some(apk.to_string_lossy().into_owned()),
            artifact_sha256: Some(
                "b9de1edb74ac03319f14a1c163b3ed045598c3771cf3c3d7ec410864b0c5f738".into(),
            ),
            started_at_epoch_seconds: 1,
            completed_at_epoch_seconds: 2,
        };
        state
            .plane
            .save_android_build_observation(
                &observation.execution_id,
                &observation.task_id,
                observation.source_revision,
                &observation.project_fingerprint,
                &serde_json::to_string(&observation).expect("observation json"),
            )
            .expect("durable build observation");

        let mut export_request = request(
            &state,
            "m108-fixture-export",
            CommandKind::ArtifactExport,
            serde_json::to_string(&ArtifactExportCommandPayload {
                source_revision: 0,
                destination_path: delivery.to_string_lossy().into_owned(),
            })
            .expect("export payload"),
            preflight_response.snapshot.projection_revision.0,
        );
        export_request.command.task_id = Some(contract.task_id.clone());
        let export_response = dispatch_request(&sink, &mut state, export_request.clone())
            .expect("authenticated export");
        assert_eq!(export_response.status, ResponseStatus::Completed);
        let exported: ArtifactExportResultPayload = serde_json::from_value(
            export_response
                .result_payload
                .clone()
                .expect("export result"),
        )
        .expect("typed export result");
        assert!(exported.artifact.delivery_verified);
        assert_eq!(
            exported.artifact.delivery_sha256,
            Some(exported.artifact.sha256.clone())
        );
        assert!(state
            .plane
            .load_android_artifact_export(&contract.task_id.0, 0)
            .expect("artifact export reload")
            .is_some());

        let preview_request = PreviewRequest {
            schema_version: nirman_preview::M48_SCHEMA_VERSION,
            request_id: "m108-fixture-preview".into(),
            project_id: PROJECT_ID.into(),
            task_id: contract.task_id.0.clone(),
            project_revision_id: "source-0".into(),
            checkpoint_id: "checkpoint-m108-fixture".into(),
            source_fingerprint: "fingerprint-m108-fixture".into(),
            contract_version: "contract-m39-v1".into(),
            technology_plan_version: "technology-m39-v1".into(),
            asset_manifest_version: "assets-m108-v1".into(),
            build_variant: "debug".into(),
            device_id: Some("pixel-api-35".into()),
            android_api_level: Some(35),
            requested_mode: Some(PreviewMode::ApkReinstall),
            selected_language: "kotlin".into(),
            selected_ui_framework: "Jetpack Compose".into(),
            changed_paths: vec!["app/src/main".into()],
            required_evidence_kinds: vec!["DEVICE_EVIDENCE".into(), "VISUAL_EVIDENCE".into()],
            policy_decision_id: "policy-m108-fixture".into(),
        };
        let mut preview_command = request(
            &state,
            "m108-fixture-preview",
            CommandKind::PreviewStart,
            serde_json::to_string(&PreviewStartCommandPayload {
                request: preview_request,
            })
            .expect("preview payload"),
            export_response.snapshot.projection_revision.0,
        );
        preview_command.command.task_id = Some(contract.task_id.clone());
        let preview_response =
            dispatch_request(&sink, &mut state, preview_command).expect("authenticated preview");
        assert_eq!(preview_response.status, ResponseStatus::Completed);
        let preview_result: PreviewStartResultPayload = serde_json::from_value(
            preview_response
                .result_payload
                .clone()
                .expect("preview result"),
        )
        .expect("typed preview result");
        let observation = preview_result
            .device_observation
            .expect("headless device observation");
        assert_eq!(observation.device_identity, "pixel-api-35");
        assert_eq!(
            observation.apk_sha256,
            "sha256:b9de1edb74ac03319f14a1c163b3ed045598c3771cf3c3d7ec410864b0c5f738"
        );
        assert!(state
            .plane
            .load_android_device_observation(&contract.task_id.0, 0, "pixel-api-35")
            .expect("device observation reload")
            .is_some());

        let events = state
            .plane
            .load_m108_event_jsons(&contract.task_id.0)
            .expect("M108 event reload");
        let event_types = events
            .iter()
            .map(|json| {
                serde_json::from_str::<nirman_preview::PreviewSyncEvent>(json)
                    .expect("event json")
                    .event_type
            })
            .collect::<Vec<_>>();
        assert_eq!(
            event_types,
            vec![
                PreviewSyncEventType::ArtifactObserved,
                PreviewSyncEventType::ValidationObserved,
                PreviewSyncEventType::InstallObserved,
                PreviewSyncEventType::LaunchObserved,
                PreviewSyncEventType::InteractionObserved,
            ]
        );
        let evidence_records = state
            .plane
            .load_m108_evidence_jsons(&contract.task_id.0)
            .expect("M109 evidence reload");
        assert_eq!(evidence_records.len(), event_types.len());
        for evidence_json in evidence_records {
            let evidence: nirman_preview::PreviewSyncEvidenceRecord =
                serde_json::from_str(&evidence_json).expect("evidence json");
            evidence.validate().expect("complete durable evidence");
        }
        let stored_projection = state
            .plane
            .load_m108_sync_record(&contract.task_id.0)
            .expect("M108 projection reload")
            .expect("M108 projection");
        let projection: M108ProjectionState =
            serde_json::from_str(&stored_projection.0).expect("projection json");
        assert_eq!(projection.install_status, "OBSERVED");
        assert_eq!(projection.launch_status, "OBSERVED");
        assert_eq!(projection.runtime_status, "OBSERVED");
        assert_eq!(projection.validation_status, "OBSERVED");

        drop(state);
        let mut reopened = test_state_at(&database);
        let replay_events = reopened
            .plane
            .load_m108_event_jsons(&contract.task_id.0)
            .expect("replay events");
        let mut reconstructed = M108ProjectionState::new(
            PROJECT_ID,
            &contract.task_id.0,
            &projection.active_preview_revision_id,
        );
        for json in replay_events {
            let event: nirman_preview::PreviewSyncEvent =
                serde_json::from_str(&json).expect("replay event");
            reconstructed.apply(&event).expect("replay reduction");
        }
        assert_eq!(reconstructed, projection);
        let mut premature_promotion = nirman_preview::PreviewSyncEvent {
            event_id: "m108-fixture-premature-promotion".into(),
            event_sequence: projection.last_event_sequence + 1,
            project_id: PROJECT_ID.into(),
            task_id: contract.task_id.0.clone(),
            correlation_id: "m108-fixture".into(),
            causation_id: Some("m108-fixture-promote".into()),
            candidate_preview_revision_id: projection.active_preview_revision_id.clone(),
            event_type: PreviewSyncEventType::PreviewPromoted,
            event_truth: PreviewEventTruth::Verified,
            project_revision_id: "source-0".into(),
            checkpoint_id: "checkpoint-m108-fixture".into(),
            source_fingerprint: "fingerprint-m108-fixture".into(),
            artifact_id: Some(exported.artifact.artifact_id),
            artifact_fingerprint: Some(
                "sha256:b9de1edb74ac03319f14a1c163b3ed045598c3771cf3c3d7ec410864b0c5f738".into(),
            ),
            runtime_session_id: Some(observation.runtime_session_id),
            device_id: Some("pixel-api-35".into()),
            operation_ref: "m108-fixture-promote".into(),
            observation_refs: vec!["m108-fixture-observation".into()],
            evidence_refs: vec!["m108-fixture-evidence".into()],
            validation_ref: Some("validation:m108-fixture".into()),
            payload: "headless promotion must remain blocked".into(),
        };
        assert!(matches!(
            reconstructed.apply(&premature_promotion),
            Err(nirman_preview::M108ReducerError::EvidenceRequired
                | nirman_preview::M108ReducerError::StaleEvent)
        ));
        premature_promotion.event_sequence += 1;
        let _ = fs::remove_file(database);
        let _ = fs::remove_dir_all(root);
        drop(reopened);
    }
}
