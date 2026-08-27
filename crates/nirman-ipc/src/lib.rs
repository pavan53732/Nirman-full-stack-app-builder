#![forbid(unsafe_code)]

use nirman_android::{
    AndroidBuildObservation, AndroidRequirementManifest, AndroidSynthesisPlan, RepairSelection,
};
use nirman_artifacts::ApkArtifact;
use nirman_domain::{
    AndroidConstructionContract, CommandEnvelope, CommandKind, ControlEvent, ProjectId,
    ProjectionSnapshot, Revision, TaskId,
};
use nirman_evidence::AndroidDeviceObservation;
use nirman_preview::{PreviewFallbackSelection, PreviewRequest, PreviewRevision};
use nirman_project::{MutationEvidence, MutationFileResult, MutationOperation};
use nirman_workers::{
    CoordinationTask, M8ReconciliationCheckpoint, WorkerContract, WorkerHandoffAcknowledgement,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL_SCHEMA_VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuthContext {
    pub installation_id: String,
    pub user_scope: String,
    pub project_scope: String,
    pub schema_version: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommandRequest {
    pub protocol_schema_version: u16,
    pub auth: AuthContext,
    pub command: CommandEnvelope,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub deadline_epoch_seconds: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ResponseStatus {
    Accepted,
    Completed,
    Rejected,
    Duplicate,
    Stale,
    Cancelled,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EventRange {
    pub first_sequence: u64,
    pub last_sequence: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommandResponse {
    pub response_id: String,
    pub command_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub project_id: ProjectId,
    pub task_id: Option<TaskId>,
    pub status: ResponseStatus,
    pub result_schema_ref: Option<String>,
    pub projection_snapshot_ref: Option<String>,
    pub projection_revision: Revision,
    pub snapshot: ProjectionSnapshot,
    pub event_range: Option<EventRange>,
    pub result_payload: Option<Value>,
    pub authority_decision_ref: String,
    pub created_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ErrorCategory {
    Authentication,
    Authorization,
    Scope,
    Validation,
    StaleProjection,
    Idempotency,
    NotFound,
    Conflict,
    Environment,
    Provider,
    Device,
    Timeout,
    Cancellation,
    ReplayGap,
    Unavailable,
    Internal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ControlPlaneErrorCode {
    AuthenticationFailed,
    SchemaMismatch,
    PermissionDenied,
    StaleProjection,
    DuplicateCommand,
    IdempotencyConflict,
    DependencyUnavailable,
    InvalidCommand,
    CancellationRejected,
    Timeout,
    ReplayGap,
    SubscriptionNotFound,
    Backpressure,
}

pub fn authorize_registry_capability(
    request: &CommandRequest,
) -> Result<(), ControlPlaneErrorCode> {
    let entry = command_registry()
        .into_iter()
        .find(|entry| entry.command_kind == request.command.kind && entry.supported)
        .ok_or(ControlPlaneErrorCode::InvalidCommand)?;
    if request.auth.user_scope != "local-user"
        || request.auth.project_scope != request.command.project_id.0
        || entry.required_capability.trim().is_empty()
    {
        return Err(ControlPlaneErrorCode::PermissionDenied);
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub error_id: String,
    pub command_id: Option<String>,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub code: ControlPlaneErrorCode,
    pub category: ErrorCategory,
    pub safe_message: String,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
    pub recovery_action: Option<String>,
    pub diagnostic_ref: Option<String>,
    pub authority_decision_ref: String,
    pub sensitive_data_omitted: bool,
    pub created_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Requested,
    Active,
    Paused,
    Gap,
    Closed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum BackpressurePolicy {
    PauseOnLimit,
    RejectOverLimit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EventSubscription {
    pub subscription_id: String,
    pub connection_id: String,
    pub auth: AuthContext,
    pub project_id: String,
    pub task_id: Option<String>,
    pub from_event_sequence: u64,
    pub snapshot_revision: Option<Revision>,
    pub requested_projection_kinds: Vec<String>,
    pub acknowledged_event_sequence: u64,
    pub heartbeat_interval_seconds: u64,
    pub max_batch_size: usize,
    pub backpressure_policy: BackpressurePolicy,
    pub status: SubscriptionStatus,
    pub correlation_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EventBatch {
    pub subscription_id: String,
    pub projection_revision: Revision,
    pub from_event_sequence: u64,
    pub next_event_sequence: u64,
    pub events: Vec<ControlEvent>,
    pub has_gap: bool,
    pub status: SubscriptionStatus,
}

pub fn acknowledge_event_subscription(
    subscription: &mut EventSubscription,
    auth: &AuthContext,
    correlation_id: &str,
    acknowledged_sequence: u64,
    durable_last_sequence: u64,
) -> Result<(), ControlPlaneErrorCode> {
    if subscription.status != SubscriptionStatus::Active {
        return Err(ControlPlaneErrorCode::Backpressure);
    }
    if subscription.auth != *auth || subscription.correlation_id != correlation_id {
        return Err(ControlPlaneErrorCode::AuthenticationFailed);
    }
    if acknowledged_sequence < subscription.acknowledged_event_sequence {
        return Err(ControlPlaneErrorCode::ReplayGap);
    }
    if acknowledged_sequence > durable_last_sequence {
        return Err(ControlPlaneErrorCode::ReplayGap);
    }
    subscription.acknowledged_event_sequence = acknowledged_sequence;
    Ok(())
}

pub trait EventSink {
    fn emit_batch(&self, batch: EventBatch) -> Result<(), ()>;
}

pub fn publish_control_event<S: EventSink>(
    subscriptions: &mut BTreeMap<String, EventSubscription>,
    sink: &S,
    projection_revision: Revision,
    after_sequence: u64,
    event: &ControlEvent,
) {
    let active: Vec<EventSubscription> = subscriptions
        .values()
        .filter(|subscription| subscription.status == SubscriptionStatus::Active)
        .cloned()
        .collect();
    for subscription in active {
        let batch = EventBatch {
            subscription_id: subscription.subscription_id.clone(),
            projection_revision,
            from_event_sequence: after_sequence,
            next_event_sequence: event.sequence,
            events: vec![event.clone()],
            has_gap: false,
            status: SubscriptionStatus::Active,
        };
        if subscription.max_batch_size < batch.events.len() || sink.emit_batch(batch).is_err() {
            if let Some(stored) = subscriptions.get_mut(&subscription.subscription_id) {
                stored.status = SubscriptionStatus::Paused;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceApplyPatchCommandPayload {
    pub worker_id: String,
    pub operation_id: String,
    pub base_revision: u64,
    pub base_project_fingerprint: String,
    pub workspace_root: String,
    pub allowed_paths: BTreeSet<String>,
    pub owned_paths: BTreeSet<String>,
    pub touched_paths: Vec<String>,
    pub base_file_hashes: BTreeMap<String, String>,
    pub mutation_budget: u32,
    pub dependency_policy: String,
    pub capability_digest: String,
    pub fence_token: u64,
    pub evidence_required: bool,
    pub isolated_transaction: bool,
    pub whole_file_fallback: bool,
    pub operation: MutationOperation,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceApplyPatchResultPayload {
    pub operation_id: String,
    pub project_fingerprint: String,
    pub changed_files: Vec<MutationFileResult>,
    pub evidence: MutationEvidence,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerTaskClaimCommandPayload {
    pub parent_contract: WorkerContract,
    pub task: CoordinationTask,
    pub now_epoch_seconds: u64,
    pub lease_duration_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerTaskClaimResultPayload {
    pub task_id: String,
    pub worker_id: String,
    pub lease_id: String,
    pub fence_token: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerHandoffSubmitCommandPayload {
    pub parent_contract: WorkerContract,
    pub handoff: nirman_workers::WorkerHandoffRecord,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerHandoffSubmitResultPayload {
    pub message_id: String,
    pub task_id: String,
    pub worker_id: String,
    pub source_revision: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerHandoffAcknowledgeCommandPayload {
    pub parent_contract: WorkerContract,
    pub acknowledgement_id: String,
    pub message_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerHandoffAcknowledgeResultPayload {
    pub acknowledgement: WorkerHandoffAcknowledgement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerReconcileCommandPayload {
    pub parent_contract: WorkerContract,
    pub checkpoint_id: String,
    pub integration_workspace_root: String,
    pub handoff_message_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkerReconcileResultPayload {
    pub checkpoint: M8ReconciliationCheckpoint,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderTestCommandPayload {
    pub provider_id: String,
    pub prompt: String,
    pub max_output_tokens: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderExecuteCommandPayload {
    pub provider_id: String,
    pub worker_id: String,
    pub prompt: String,
    pub max_output_tokens: Option<u64>,
    pub max_context_tokens: u64,
    pub privacy_classification: String,
    pub tool_policy: String,
    pub stream: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderExecuteResultPayload {
    pub execution_id: String,
    pub request_id: String,
    pub correlation_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub environment_lock_hash: String,
    pub environment_snapshot_id: String,
    pub state: String,
    pub outcome: String,
    pub text: Option<String>,
    pub error_kind: Option<String>,
    pub events: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SettingsUpdateProviderCommandPayload {
    pub profile: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderTestResultPayload {
    pub provider_id: String,
    pub model_id: String,
    pub request_id: String,
    pub correlation_id: String,
    pub provider_request_id: Option<String>,
    pub text: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidToolchainPreflightCommandPayload {
    pub build_variant: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidToolchainPreflightResultPayload {
    pub preflight_id: String,
    pub status: String,
    pub lock_hash: Option<String>,
    pub environment_snapshot_id: String,
    pub capability_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidConstructionResultPayload {
    pub contract_id: String,
    pub synthesis_plan: Option<AndroidSynthesisPlan>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidSynthesisBuildCommandPayload {
    pub contract: AndroidConstructionContract,
    pub source_revision: u64,
    pub workspace_root: String,
    pub project_fingerprint: String,
    pub build_variant: String,
    pub gradle_task: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidSynthesisBuildResultPayload {
    pub contract_id: String,
    pub synthesis_plan: AndroidSynthesisPlan,
    pub build_request: nirman_android::AndroidBuildRequest,
    pub toolchain_lock_hash: String,
    pub environment_snapshot_id: String,
    pub native_build_observed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArtifactBuildCommandPayload {
    pub source_revision: u64,
    pub workspace_root: String,
    pub project_fingerprint: String,
    pub build_variant: String,
    pub gradle_task: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArtifactBuildResultPayload {
    pub observation: AndroidBuildObservation,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArtifactExportCommandPayload {
    pub source_revision: u64,
    pub destination_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArtifactExportResultPayload {
    pub artifact: ApkArtifact,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidRequirementEvaluateCommandPayload {
    pub workspace_root: String,
    pub project_fingerprint: String,
    pub source_revision: u64,
    pub failure: Option<RepairFailurePayload>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RepairFailurePayload {
    pub classifier: String,
    pub detail: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidRequirementEvaluateResultPayload {
    pub manifest: AndroidRequirementManifest,
    pub repair_selection: Option<RepairSelection>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PreviewStartCommandPayload {
    pub request: PreviewRequest,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PreviewStartResultPayload {
    pub selection: PreviewFallbackSelection,
    pub revision: PreviewRevision,
    pub device_observation: Option<AndroidDeviceObservation>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionBootstrap {
    pub subscription: EventSubscription,
    pub snapshot: ProjectionSnapshot,
    pub batch: EventBatch,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionAcknowledgement {
    pub auth: AuthContext,
    pub subscription_id: String,
    pub acknowledged_event_sequence: u64,
    pub correlation_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionControl {
    pub auth: AuthContext,
    pub subscription_id: String,
    pub correlation_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommandRegistryEntry {
    pub command_kind: CommandKind,
    pub canonical_kind: String,
    pub supported: bool,
    pub request_schema_ref: String,
    pub response_schema_ref: String,
    pub required_authority: String,
    pub required_capability: String,
    pub project_scope: String,
    pub transaction_domain: String,
    pub idempotency_policy: String,
    pub timeout_policy: String,
    pub cancellation_policy: String,
    pub emitted_event_types: Vec<String>,
    pub projection_effects: Vec<String>,
    pub error_codes: Vec<ControlPlaneErrorCode>,
    pub sensitive_fields: Vec<String>,
}

pub fn command_registry() -> Vec<CommandRegistryEntry> {
    let common_errors = vec![
        ControlPlaneErrorCode::AuthenticationFailed,
        ControlPlaneErrorCode::SchemaMismatch,
        ControlPlaneErrorCode::PermissionDenied,
        ControlPlaneErrorCode::StaleProjection,
        ControlPlaneErrorCode::IdempotencyConflict,
        ControlPlaneErrorCode::DependencyUnavailable,
        ControlPlaneErrorCode::Timeout,
    ];
    [
        (
            CommandKind::ProjectOpen,
            "project.open",
            "WorkspaceAuthority",
            "project.open",
            "Project projection",
            "local",
        ),
        (
            CommandKind::TaskStart,
            "task.start",
            "LifecycleAuthority",
            "task.start",
            "Task and worker projection",
            "local",
        ),
        (
            CommandKind::TaskCancel,
            "task.cancel",
            "LifecycleAuthority",
            "task.cancel",
            "Cancellation projection",
            "local",
        ),
        (
            CommandKind::TaskResume,
            "task.resume",
            "RecoveryAuthority",
            "task.resume",
            "Task and continuity projection",
            "local",
        ),
        (
            CommandKind::WorkspaceApplyPatch,
            "workspace.apply_patch",
            "WorkspaceAuthority",
            "workspace.apply_patch",
            "Revision projection",
            "local",
        ),
        (
            CommandKind::PreviewStart,
            "preview.start",
            "PreviewCoordinator",
            "preview.start",
            "Preview candidate projection",
            "device",
        ),
        (
            CommandKind::PreviewStop,
            "preview.stop",
            "LifecycleAuthority",
            "preview.stop",
            "Preview lifecycle projection",
            "device",
        ),
        (
            CommandKind::PreviewPromote,
            "preview.promote",
            "PreviewPromotionGate",
            "preview.promote",
            "Promotion projection",
            "device",
        ),
        (
            CommandKind::ValidationRun,
            "validation.run",
            "ValidationAuthority",
            "validation.run",
            "Validation projection",
            "local-or-device",
        ),
        (
            CommandKind::ArtifactBuild,
            "artifact.build",
            "ArtifactAuthority",
            "artifact.build",
            "Build and artifact projection",
            "local",
        ),
        (
            CommandKind::ArtifactExport,
            "artifact.export",
            "ArtifactAuthority",
            "artifact.export",
            "Delivery projection",
            "local",
        ),
        (
            CommandKind::ProviderTest,
            "provider.test",
            "ProviderOperationalityAuthority",
            "provider.test",
            "Provider projection",
            "external",
        ),
        (
            CommandKind::SettingsUpdateProvider,
            "settings.update_provider",
            "PolicyAuthority",
            "settings.update_provider",
            "Settings projection",
            "local",
        ),
        (
            CommandKind::ProviderExecute,
            "provider.execute",
            "ProviderBridgeAuthority",
            "provider.execute",
            "Provider execution projection",
            "external",
        ),
        (
            CommandKind::AndroidConstructionCreate,
            "android.construction.create",
            "ConstructionContractAuthority",
            "android.construction.create",
            "Android construction contract projection",
            "local",
        ),
        (
            CommandKind::AndroidToolchainPreflight,
            "android.toolchain.preflight",
            "ToolchainAuthority",
            "android.toolchain.preflight",
            "Android toolchain and environment projection",
            "local",
        ),
        (
            CommandKind::AndroidRequirementEvaluate,
            "android.requirements.evaluate",
            "AndroidRequirementAuthority",
            "android.requirements.evaluate",
            "Android requirement manifest and repair-selection projection",
            "local",
        ),
        (
            CommandKind::AndroidSynthesisBuild,
            "android.synthesis.build",
            "AndroidSynthesisAuthority",
            "android.synthesis.build",
            "Android synthesis and build provenance projection",
            "local",
        ),
        (
            CommandKind::WorkerTaskClaim,
            "worker.task.claim",
            "WorkerCoordinationAuthority",
            "worker.task.claim",
            "Worker lease and coordination projection",
            "local",
        ),
        (
            CommandKind::WorkerHandoffSubmit,
            "worker.handoff.submit",
            "WorkerCoordinationAuthority",
            "worker.handoff.submit",
            "Worker handoff projection",
            "local",
        ),
        (
            CommandKind::WorkerHandoffAcknowledge,
            "worker.handoff.acknowledge",
            "WorkerCoordinationAuthority",
            "worker.handoff.acknowledge",
            "Worker handoff acknowledgement projection",
            "local",
        ),
        (
            CommandKind::WorkerReconcile,
            "worker.reconcile",
            "WorkerCoordinationAuthority",
            "worker.reconcile",
            "Transactional integration checkpoint projection",
            "local",
        ),
    ]
    .into_iter()
    .map(
        |(kind, canonical_kind, authority, capability, effect, transaction_domain)| {
            CommandRegistryEntry {
                command_kind: kind,
                canonical_kind: canonical_kind.into(),
                supported: true,
                request_schema_ref: "nirman.command_request.v1".into(),
                response_schema_ref: "nirman.command_response.v1".into(),
                required_authority: authority.into(),
                required_capability: capability.into(),
                project_scope: "authenticated-project".into(),
                transaction_domain: transaction_domain.into(),
                idempotency_policy: "persisted-request-fingerprint".into(),
                timeout_policy: "bounded-local-command".into(),
                cancellation_policy: "durable-lifecycle-cancellation".into(),
                emitted_event_types: vec![format!("{:?}", kind)],
                projection_effects: vec![effect.into()],
                error_codes: common_errors.clone(),
                sensitive_fields: vec!["payload".into(), "diagnosticRef".into()],
            }
        },
    )
    .chain(
        [
            (
                CommandKind::Reconnect,
                "connection.reconnect",
                "RecoveryAuthority",
                "connection.reconnect",
                "Continuity projection",
            ),
            (
                CommandKind::PauseTask,
                "task.pause",
                "LifecycleAuthority",
                "task.pause",
                "Task projection",
            ),
        ]
        .into_iter()
        .map(
            |(kind, canonical_kind, authority, capability, effect)| CommandRegistryEntry {
                command_kind: kind,
                canonical_kind: canonical_kind.into(),
                supported: true,
                request_schema_ref: "nirman.command_request.v1".into(),
                response_schema_ref: "nirman.command_response.v1".into(),
                required_authority: authority.into(),
                required_capability: capability.into(),
                project_scope: "authenticated-project".into(),
                transaction_domain: "local".into(),
                idempotency_policy: "persisted-request-fingerprint".into(),
                timeout_policy: "bounded-local-command".into(),
                cancellation_policy: "durable-lifecycle-cancellation".into(),
                emitted_event_types: vec![format!("{:?}", kind)],
                projection_effects: vec![effect.into()],
                error_codes: common_errors.clone(),
                sensitive_fields: vec!["payload".into(), "diagnosticRef".into()],
            },
        ),
    )
    .collect()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidServiceIntegration {
    pub request_schema_ref: String,
    pub response_schema_ref: String,
    pub error_schema_ref: String,
    pub auth_state: String,
    pub credential_reference: String,
    pub base_endpoint_identity: String,
    pub datastore_owner: String,
    pub offline_policy: String,
    pub retry_policy: String,
    pub timeout_policy: String,
    pub idempotency_policy: String,
    pub token_refresh_policy: String,
    pub privacy_policy: String,
    pub network_policy: String,
    pub functional_scenario_ids: Vec<String>,
}

impl AndroidServiceIntegration {
    pub fn validate(&self) -> Result<(), &'static str> {
        let required = [
            &self.request_schema_ref,
            &self.response_schema_ref,
            &self.error_schema_ref,
            &self.auth_state,
            &self.credential_reference,
            &self.base_endpoint_identity,
            &self.datastore_owner,
            &self.offline_policy,
            &self.retry_policy,
            &self.timeout_policy,
            &self.idempotency_policy,
            &self.token_refresh_policy,
            &self.privacy_policy,
            &self.network_policy,
        ];
        if required.iter().any(|value| value.trim().is_empty())
            || self.functional_scenario_ids.is_empty()
        {
            return Err("AndroidServiceIntegration is incomplete");
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidServiceAdapter {
    pub integration: AndroidServiceIntegration,
}

impl AndroidServiceAdapter {
    pub fn new(integration: AndroidServiceIntegration) -> Result<Self, &'static str> {
        integration.validate()?;
        Ok(Self { integration })
    }

    pub fn normalize_error(
        &self,
        kind: AndroidServiceErrorKind,
        safe_message: impl Into<String>,
        retryable: bool,
        idempotency_key: Option<String>,
    ) -> AndroidServiceError {
        normalize_android_service_error(kind, safe_message, retryable, idempotency_key)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AndroidServiceErrorKind {
    Authentication,
    Timeout,
    Offline,
    InvalidResponse,
    Application,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidServiceError {
    pub kind: AndroidServiceErrorKind,
    pub safe_message: String,
    pub retryable: bool,
    pub idempotency_key: Option<String>,
}

pub fn normalize_android_service_error(
    kind: AndroidServiceErrorKind,
    safe_message: impl Into<String>,
    retryable: bool,
    idempotency_key: Option<String>,
) -> AndroidServiceError {
    AndroidServiceError {
        kind,
        safe_message: safe_message.into(),
        retryable,
        idempotency_key,
    }
}

#[derive(Clone, Debug)]
pub struct AuthenticatedSession {
    context: AuthContext,
    session_nonce: String,
    expires_at_epoch_seconds: u64,
}

impl AuthenticatedSession {
    pub fn issue(context: AuthContext, session_nonce: impl Into<String>, ttl_seconds: u64) -> Self {
        let now = now_epoch_seconds();
        Self {
            context,
            session_nonce: session_nonce.into(),
            expires_at_epoch_seconds: now.saturating_add(ttl_seconds),
        }
    }

    pub fn authorize_context(
        &self,
        auth: &AuthContext,
        project_id: &str,
        correlation_id: &str,
    ) -> Result<(), ControlPlaneErrorCode> {
        if now_epoch_seconds() >= self.expires_at_epoch_seconds {
            return Err(ControlPlaneErrorCode::AuthenticationFailed);
        }
        if auth.schema_version != PROTOCOL_SCHEMA_VERSION {
            return Err(ControlPlaneErrorCode::SchemaMismatch);
        }
        if auth != &self.context || project_id != self.context.project_scope {
            return Err(ControlPlaneErrorCode::PermissionDenied);
        }
        if correlation_id != self.session_nonce {
            return Err(ControlPlaneErrorCode::AuthenticationFailed);
        }
        Ok(())
    }

    pub fn authorize(&self, request: &CommandRequest) -> Result<(), ControlPlaneErrorCode> {
        if request.protocol_schema_version != PROTOCOL_SCHEMA_VERSION {
            return Err(ControlPlaneErrorCode::SchemaMismatch);
        }
        self.authorize_context(
            &request.auth,
            &request.command.project_id.0,
            &request.correlation_id,
        )
    }

    pub fn context(&self) -> &AuthContext {
        &self.context
    }

    pub fn expires_at_epoch_seconds(&self) -> u64 {
        self.expires_at_epoch_seconds
    }
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Clone, Debug, Default)]
pub struct ProjectionReceiver {
    snapshot: Option<ProjectionSnapshot>,
    last_event_sequence: u64,
    events: BTreeMap<u64, ControlEvent>,
    rejected_events: u64,
}

impl ProjectionReceiver {
    pub fn observe_snapshot(&mut self, snapshot: ProjectionSnapshot) -> bool {
        if self.snapshot.as_ref().is_some_and(|current| {
            current.project_id != snapshot.project_id
                || current.projection_revision >= snapshot.projection_revision
                || snapshot.last_event_sequence < self.last_event_sequence
        }) {
            return false;
        }
        self.last_event_sequence = snapshot.last_event_sequence;

        self.snapshot = Some(snapshot);
        true
    }

    pub fn observe_event(&mut self, event: &ControlEvent) -> bool {
        if self
            .snapshot
            .as_ref()
            .map_or(true, |snapshot| snapshot.project_id != event.project_id)
        {
            self.rejected_events = self.rejected_events.saturating_add(1);
            return false;
        }
        if event.sequence <= self.last_event_sequence {
            self.rejected_events = self.rejected_events.saturating_add(1);
            return false;
        }
        if event.sequence != self.last_event_sequence.saturating_add(1) {
            self.rejected_events = self.rejected_events.saturating_add(1);
            return false;
        }
        self.last_event_sequence = event.sequence;
        self.events.insert(event.sequence, event.clone());
        true
    }

    pub fn snapshot(&self) -> Option<&ProjectionSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn rejected_events(&self) -> u64 {
        self.rejected_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_domain::{BackgroundContinuityState, PreviewTruth};

    fn context() -> AuthContext {
        AuthContext {
            installation_id: "install-1".into(),
            user_scope: "local-user".into(),
            project_scope: "project-1".into(),
            schema_version: PROTOCOL_SCHEMA_VERSION,
        }
    }

    #[test]
    fn session_rejects_schema_scope_and_expiry_failures() {
        let session = AuthenticatedSession::issue(context(), "corr-1", 60);
        let request = CommandRequest {
            protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
            auth: context(),
            command: CommandEnvelope {
                command_id: "cmd-1".into(),
                project_id: ProjectId("project-1".into()),
                task_id: None,
                kind: CommandKind::Reconnect,
                payload: String::new(),
                expected_projection_revision: Revision(0),
                idempotency_key: None,
            },
            correlation_id: "corr-1".into(),
            causation_id: None,
            deadline_epoch_seconds: None,
        };
        session.authorize(&request).expect("valid session");
        let mut wrong_schema = request.clone();
        wrong_schema.protocol_schema_version = 99;
        assert_eq!(
            session.authorize(&wrong_schema),
            Err(ControlPlaneErrorCode::SchemaMismatch)
        );
        let mut wrong_project = request;
        wrong_project.command.project_id = ProjectId("other-project".into());
        assert_eq!(
            session.authorize(&wrong_project),
            Err(ControlPlaneErrorCode::PermissionDenied)
        );
    }

    #[test]
    fn projection_receiver_rejects_identity_gaps_and_duplicates() {
        let project = ProjectId("project-1".into());
        let snapshot = ProjectionSnapshot {
            project_id: project.clone(),
            projection_revision: Revision(1),
            task_state: nirman_domain::ProductLifecycleState::Planning,
            continuity_state: BackgroundContinuityState::ActiveBackground,
            preview_truth: PreviewTruth::Requested,
            current_source_revision: Revision(1),
            last_event_sequence: 0,
            last_known_good_ref: None,
        };
        let mut receiver = ProjectionReceiver::default();
        assert!(receiver.observe_snapshot(snapshot));
        let event = ControlEvent {
            event_id: "event-1".into(),
            sequence: 1,
            project_id: project.clone(),
            task_id: None,
            kind: "SubmitInstruction".into(),
            payload: "Build".into(),
            source_revision: Revision(1),
        };
        assert!(receiver.observe_event(&event));
        assert!(!receiver.observe_event(&event));
        assert!(!receiver.observe_event(&ControlEvent {
            sequence: 3,
            ..event.clone()
        }));
        assert!(!receiver.observe_event(&ControlEvent {
            project_id: ProjectId("other".into()),
            sequence: 2,
            ..event
        }));
        assert_eq!(receiver.rejected_events(), 3);
    }

    #[test]
    fn registry_and_android_adapter_are_typed() {
        assert_eq!(command_registry().len(), 24);
        assert!(command_registry().iter().all(|entry| entry.supported));
        assert!(command_registry()
            .iter()
            .any(|entry| entry.canonical_kind == "artifact.export"));
        let m47 = command_registry()
            .into_iter()
            .find(|entry| entry.command_kind == CommandKind::AndroidRequirementEvaluate)
            .expect("M47 command registry entry");
        assert_eq!(m47.required_authority, "AndroidRequirementAuthority");
        assert_eq!(m47.required_capability, "android.requirements.evaluate");
        let integration = AndroidServiceIntegration {
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
        };
        let adapter =
            AndroidServiceAdapter::new(integration).expect("valid Android service adapter");
        let error = adapter.normalize_error(
            AndroidServiceErrorKind::Timeout,
            "service timed out",
            true,
            Some("idem-1".into()),
        );
        assert!(error.retryable);
        assert_eq!(error.idempotency_key.as_deref(), Some("idem-1"));
    }
}

#[cfg(test)]
mod m115_subscription_bridge_tests {
    use super::*;
    use nirman_domain::{ControlEvent, ProjectId, Revision};
    use std::cell::RefCell;

    struct TestSink {
        fail: bool,
        batches: RefCell<Vec<EventBatch>>,
    }

    impl EventSink for TestSink {
        fn emit_batch(&self, batch: EventBatch) -> Result<(), ()> {
            if self.fail {
                Err(())
            } else {
                self.batches.borrow_mut().push(batch);
                Ok(())
            }
        }
    }

    fn subscription(id: &str, connection: &str) -> EventSubscription {
        EventSubscription {
            subscription_id: id.into(),
            connection_id: connection.into(),
            auth: AuthContext {
                installation_id: "install".into(),
                user_scope: "local-user".into(),
                project_scope: "project-1".into(),
                schema_version: PROTOCOL_SCHEMA_VERSION,
            },
            project_id: "project-1".into(),
            task_id: None,
            from_event_sequence: 0,
            snapshot_revision: Some(Revision(0)),
            requested_projection_kinds: vec!["task".into()],
            acknowledged_event_sequence: 0,
            heartbeat_interval_seconds: 15,
            max_batch_size: 1,
            backpressure_policy: BackpressurePolicy::PauseOnLimit,
            status: SubscriptionStatus::Active,
            correlation_id: "corr".into(),
        }
    }

    #[test]
    fn host_bridge_pauses_on_emit_failure_and_fresh_subscription_recovers() {
        let event = ControlEvent {
            event_id: "event-1".into(),
            sequence: 1,
            project_id: ProjectId("project-1".into()),
            task_id: None,
            kind: "TaskStart".into(),
            payload: "Build".into(),
            source_revision: Revision(1),
        };
        let mut subscriptions = BTreeMap::new();
        subscriptions.insert("sub-old".into(), subscription("sub-old", "conn-old"));
        let failing = TestSink {
            fail: true,
            batches: RefCell::new(Vec::new()),
        };
        publish_control_event(&mut subscriptions, &failing, Revision(1), 0, &event);
        assert_eq!(subscriptions["sub-old"].status, SubscriptionStatus::Paused);
        assert!(failing.batches.borrow().is_empty());

        subscriptions.remove("sub-old");
        subscriptions.insert("sub-new".into(), subscription("sub-new", "conn-new"));
        let healthy = TestSink {
            fail: false,
            batches: RefCell::new(Vec::new()),
        };
        publish_control_event(&mut subscriptions, &healthy, Revision(1), 0, &event);
        let batches = healthy.batches.borrow();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].events.len(), 1);
        assert_eq!(batches[0].from_event_sequence, 0);
        assert_eq!(batches[0].next_event_sequence, 1);
        assert_eq!(subscriptions["sub-new"].status, SubscriptionStatus::Active);

        let mut paused = subscriptions
            .remove("sub-new")
            .expect("reconnected subscription");
        paused.status = SubscriptionStatus::Paused;
        let auth = paused.auth.clone();
        assert_eq!(
            acknowledge_event_subscription(&mut paused, &auth, "corr", 1, 1),
            Err(ControlPlaneErrorCode::Backpressure)
        );
        paused.status = SubscriptionStatus::Active;
        acknowledge_event_subscription(&mut paused, &auth, "corr", 1, 1)
            .expect("active subscription accepts ACK");
        assert_eq!(paused.acknowledged_event_sequence, 1);
        assert_eq!(
            acknowledge_event_subscription(&mut paused, &auth, "corr", 0, 1),
            Err(ControlPlaneErrorCode::ReplayGap)
        );
        assert_eq!(
            acknowledge_event_subscription(&mut paused, &auth, "corr", 2, 1),
            Err(ControlPlaneErrorCode::ReplayGap)
        );
    }
}

#[cfg(test)]
mod m115_capability_tests {
    use super::*;
    use nirman_domain::{CommandEnvelope, CommandKind, ProjectId, Revision};

    fn request(user_scope: &str, project_scope: &str) -> CommandRequest {
        CommandRequest {
            protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
            auth: AuthContext {
                installation_id: "install".into(),
                user_scope: user_scope.into(),
                project_scope: project_scope.into(),
                schema_version: PROTOCOL_SCHEMA_VERSION,
            },
            command: CommandEnvelope {
                command_id: "command".into(),
                project_id: ProjectId("project-1".into()),
                task_id: None,
                kind: CommandKind::TaskStart,
                payload: "Build".into(),
                expected_projection_revision: Revision(0),
                idempotency_key: Some("idempotency".into()),
            },
            correlation_id: "corr".into(),
            causation_id: None,
            deadline_epoch_seconds: None,
        }
    }

    #[test]
    fn registry_capability_authorization_is_exact_and_minimal() {
        assert!(authorize_registry_capability(&request("local-user", "project-1")).is_ok());
        assert_eq!(
            authorize_registry_capability(&request("other-user", "project-1")),
            Err(ControlPlaneErrorCode::PermissionDenied)
        );
        assert_eq!(
            authorize_registry_capability(&request("local-user", "other-project")),
            Err(ControlPlaneErrorCode::PermissionDenied)
        );
    }
}
