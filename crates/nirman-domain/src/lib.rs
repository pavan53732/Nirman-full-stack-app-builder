//! Canonical, dependency-light domain values shared by Nirman runtime crates.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectId(pub String);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub String);

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(pub u64);

/// Durable, secret-free accounting metadata for one provider request.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderUsageRecord {
    pub request_id: String,
    pub correlation_id: String,
    pub project_id: ProjectId,
    pub provider_id: String,
    pub model_id: String,
    pub started_at_epoch_seconds: u64,
    pub duration_ms: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub outcome: String,
}

/// Secret-free durable record for one M46 structured mutation transaction.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MutationTransactionRecord {
    pub transaction_id: String,
    pub command_id: String,
    pub operation_id: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub worker_id: String,
    pub workspace_root: String,
    pub checkpoint_id: String,
    pub base_revision: Revision,
    pub resulting_revision: Revision,
    pub base_project_fingerprint: String,
    pub resulting_project_fingerprint: Option<String>,
    pub capability_digest: String,
    pub fence_token: u64,
    pub state: String,
    pub changed_paths_json: Option<String>,
    pub evidence_json: Option<String>,
    pub started_at_epoch_seconds: u64,
    pub completed_at_epoch_seconds: Option<u64>,
}

/// Secret-free durable record for one authorized M44 provider-bridge execution.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderExecutionRecord {
    pub execution_id: String,
    pub request_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub worker_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub protocol: String,
    pub environment_lock_hash: String,
    pub environment_snapshot_id: String,
    pub state: String,
    pub outcome: String,
    pub response_json: Option<String>,
    pub error_kind: Option<String>,
    pub started_at_epoch_seconds: u64,
    pub duration_ms: u64,
}

pub const ANDROID_CONSTRUCTION_CONTRACT_SCHEMA: &str = "nirman.android_construction_contract.v1";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AndroidConstructionContract {
    pub schema_version: u16,
    pub contract_id: String,
    pub project_id: ProjectId,
    pub target_platforms: Vec<String>,
    pub task_id: TaskId,
    pub user_intent: String,
    pub screenshots: Vec<VisualReferenceInput>,
    pub assets: Vec<AssetReferenceInput>,
    pub features: Vec<ConstructionRequirement>,
    pub ui: Vec<ConstructionRequirement>,
    pub data: Vec<ConstructionRequirement>,
    pub integrations: Vec<ConstructionRequirement>,
    pub technology_plan: AndroidTechnologyPlan,
    pub android_requirements: Vec<ConstructionRequirement>,
    pub device_matrix: Vec<AndroidDeviceProfile>,
    pub validation_model: ValidationModel,
    pub artifact_model: ArtifactModel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualReferenceInput {
    pub reference_id: String,
    pub source_path: String,
    pub image_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetReferenceInput {
    pub asset_id: String,
    pub source_path: String,
    pub content_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionRequirement {
    pub requirement_id: String,
    pub statement: String,
    pub origin: RequirementOrigin,
    pub source_reference_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementOrigin {
    #[serde(rename = "user_fact")]
    UserFact,
    #[serde(rename = "model_proposal")]
    ModelProposal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AndroidTechnologyPlan {
    pub plan_id: String,
    pub task_id: TaskId,
    pub requested_capabilities: Vec<String>,
    pub visual_requirements: Vec<String>,
    pub selected_languages: Vec<String>,
    pub selected_ui_frameworks: Vec<String>,
    pub selected_runtime_layers: Vec<String>,
    pub selected_native_modules: Vec<String>,
    pub selected_build_plugins: Vec<String>,
    pub selected_device_apis: Vec<String>,
    pub selected_libraries: Vec<String>,
    pub compatibility_constraints: Vec<String>,
    pub rejected_alternatives: Vec<String>,
    pub required_toolchains: Vec<String>,
    pub validation_plan: Vec<String>,
    pub confidence: Option<String>,
    pub revision: Revision,
}

pub struct AndroidResolverRequest<'a> {
    pub contract: &'a AndroidConstructionContract,
    pub source_revision: Revision,
    pub workspace_root: &'a str,
    pub project_fingerprint: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidResolverError {
    InvalidContract,
    UnsupportedPlatform,
    EmptyField(&'static str),
    StaleRevision,
}

pub trait AndroidTechnologyResolver {
    fn resolve(
        &self,
        request: &AndroidResolverRequest<'_>,
    ) -> Result<AndroidTechnologyPlan, AndroidResolverError>;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AndroidDeviceProfile {
    pub device_id: String,
    pub name: String,
    pub platform_version: String,
    pub api_level: u32,
    pub architecture: String,
    pub width: u32,
    pub height: u32,
    pub density: u32,
    pub orientation: String,
    pub locale: String,
    pub permissions: Vec<String>,
    pub network_profile: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationModel {
    pub required_checks: Vec<String>,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactModel {
    pub required_artifact: ArtifactKind,
    pub aab_declared: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    #[serde(rename = "apk")]
    Apk,
    #[serde(rename = "aab")]
    Aab,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidConstructionContractError {
    InvalidJson,
    UnsupportedSchemaVersion,
    EmptyField(&'static str),
    InvalidTargetPlatforms,
    TaskMismatch,
    DuplicateIdentifier,
    InvalidReference,
    ProposalMissingSource,
    InvalidTechnologyPlan,
    InvalidDeviceMatrix,
    InvalidValidationModel,
    InvalidArtifactModel,
}

impl fmt::Display for AndroidConstructionContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidJson => "android construction contract JSON is invalid",
            Self::UnsupportedSchemaVersion => {
                "android construction contract schema version is unsupported"
            }
            Self::EmptyField(_) => "android construction contract has a required field missing",
            Self::InvalidTargetPlatforms => {
                "android construction contract targetPlatforms must equal [android]"
            }
            Self::TaskMismatch => {
                "android technology plan task identity does not match the contract"
            }
            Self::DuplicateIdentifier => {
                "android construction contract contains a duplicate identifier"
            }
            Self::InvalidReference => {
                "android construction contract contains an invalid source reference"
            }
            Self::ProposalMissingSource => "model proposal must cite a source reference",
            Self::InvalidTechnologyPlan => "android technology plan is incomplete",
            Self::InvalidDeviceMatrix => "android device matrix is invalid",
            Self::InvalidValidationModel => "android validation model is incomplete",
            Self::InvalidArtifactModel => "android artifact model must require an APK",
        };
        if let Self::EmptyField(_) = self {
            return f.write_str(message);
        }
        f.write_str(message)
    }
}

impl std::error::Error for AndroidConstructionContractError {}

impl AndroidConstructionContract {
    pub fn validate(&self) -> Result<(), AndroidConstructionContractError> {
        if self.schema_version != 1 {
            return Err(AndroidConstructionContractError::UnsupportedSchemaVersion);
        }
        for (value, field) in [
            (&self.contract_id, "contractId"),
            (&self.project_id.0, "projectId"),
            (&self.task_id.0, "taskId"),
            (&self.user_intent, "userIntent"),
        ] {
            if value.trim().is_empty() {
                return Err(AndroidConstructionContractError::EmptyField(field));
            }
        }
        if self.target_platforms != vec!["android".to_owned()] {
            return Err(AndroidConstructionContractError::InvalidTargetPlatforms);
        }
        if self.technology_plan.task_id != self.task_id {
            return Err(AndroidConstructionContractError::TaskMismatch);
        }
        if self.features.is_empty() || self.ui.is_empty() {
            return Err(AndroidConstructionContractError::EmptyField("features/ui"));
        }
        let mut source_ids = BTreeSet::new();
        for reference in &self.screenshots {
            source_ids.insert(reference.reference_id.clone());
        }
        for asset in &self.assets {
            source_ids.insert(asset.asset_id.clone());
        }
        validate_requirements(
            self.features
                .iter()
                .chain(self.ui.iter())
                .chain(self.data.iter())
                .chain(self.integrations.iter())
                .chain(self.android_requirements.iter()),
            &source_ids,
        )?;
        if self.screenshots.iter().any(|reference| {
            reference.reference_id.trim().is_empty()
                || reference.source_path.trim().is_empty()
                || reference.image_hash.trim().is_empty()
        }) || self.assets.iter().any(|reference| {
            reference.asset_id.trim().is_empty()
                || reference.source_path.trim().is_empty()
                || reference.content_hash.trim().is_empty()
        }) {
            return Err(AndroidConstructionContractError::InvalidReference);
        }
        if self.technology_plan.plan_id.trim().is_empty()
            || self.technology_plan.requested_capabilities.is_empty()
            || self.technology_plan.required_toolchains.is_empty()
            || self.technology_plan.validation_plan.is_empty()
            || self.technology_plan.selected_languages.is_empty()
                && self.technology_plan.selected_ui_frameworks.is_empty()
                && self.technology_plan.selected_runtime_layers.is_empty()
        {
            return Err(AndroidConstructionContractError::InvalidTechnologyPlan);
        }
        if self.device_matrix.is_empty()
            || self.device_matrix.iter().any(|device| {
                device.device_id.trim().is_empty()
                    || device.name.trim().is_empty()
                    || device.platform_version.trim().is_empty()
                    || device.api_level == 0
                    || device.architecture.trim().is_empty()
                    || device.width == 0
                    || device.height == 0
                    || device.density == 0
                    || device.orientation.trim().is_empty()
                    || device.locale.trim().is_empty()
                    || device.network_profile.trim().is_empty()
            })
        {
            return Err(AndroidConstructionContractError::InvalidDeviceMatrix);
        }
        if self.validation_model.required_checks.is_empty()
            || self.validation_model.acceptance_criteria.is_empty()
            || self
                .validation_model
                .required_checks
                .iter()
                .chain(self.validation_model.acceptance_criteria.iter())
                .any(|value| value.trim().is_empty())
        {
            return Err(AndroidConstructionContractError::InvalidValidationModel);
        }
        if self.artifact_model.required_artifact != ArtifactKind::Apk {
            return Err(AndroidConstructionContractError::InvalidArtifactModel);
        }
        Ok(())
    }

    pub fn from_json(input: &str) -> Result<Self, AndroidConstructionContractError> {
        let contract: Self = serde_json::from_str(input)
            .map_err(|_| AndroidConstructionContractError::InvalidJson)?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn migrated_json(input: &str) -> Result<String, AndroidConstructionContractError> {
        let contract = Self::from_json(input)?;
        serde_json::to_string(&contract).map_err(|_| AndroidConstructionContractError::InvalidJson)
    }
}

fn validate_requirements<'a, I>(
    requirements: I,
    source_ids: &BTreeSet<String>,
) -> Result<(), AndroidConstructionContractError>
where
    I: IntoIterator<Item = &'a ConstructionRequirement>,
{
    let mut identifiers = BTreeSet::new();
    for requirement in requirements {
        if requirement.requirement_id.trim().is_empty() || requirement.statement.trim().is_empty() {
            return Err(AndroidConstructionContractError::EmptyField("requirement"));
        }
        if !identifiers.insert(requirement.requirement_id.clone()) {
            return Err(AndroidConstructionContractError::DuplicateIdentifier);
        }
        if requirement.origin == RequirementOrigin::ModelProposal
            && requirement.source_reference_ids.is_empty()
        {
            return Err(AndroidConstructionContractError::ProposalMissingSource);
        }
        if requirement
            .source_reference_ids
            .iter()
            .any(|source_id| !source_ids.contains(source_id))
        {
            return Err(AndroidConstructionContractError::InvalidReference);
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AndroidConstructionCommandPayload {
    pub contract: AndroidConstructionContract,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductLifecycleState {
    Created,
    Planning,
    Synthesizing,
    Implementing,
    Paused,
    Previewing,
    Validating,
    Recovering,
    Packaging,
    Completed,
    Blocked,
    UserRequired,
    SafelyFailed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundContinuityState {
    ActiveBackground,
    UiDisconnected,
    HostSuspended,
    HostOffline,
    DeviceUnavailable,
    ProviderUnavailable,
    Recovering,
    Reconciling,
    UserRequired,
    SafelyFailed,
    Completed,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewTruth {
    Predicted,
    Simulated,
    Requested,
    Observed,
    Verified,
    Stale,
    Invalidated,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    ProjectOpen,
    TaskStart,
    TaskCancel,
    TaskResume,
    WorkspaceApplyPatch,
    PreviewStart,
    PreviewStop,
    PreviewPromote,
    ValidationRun,
    ArtifactBuild,
    ArtifactExport,
    ProviderTest,
    SettingsUpdateProvider,
    AndroidConstructionCreate,
    AndroidToolchainPreflight,
    AndroidRequirementEvaluate,
    AndroidSynthesisBuild,
    AndroidProjectScaffold,
    AgentLoopRun,
    ProviderExecute,
    SubmitInstruction,
    Reconnect,
    PauseTask,
    ResumeTask,
    CancelTask,
    WorkerTaskClaim,
    WorkerHandoffSubmit,
    WorkerHandoffAcknowledge,
    WorkerReconcile,
    WorkerStep,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub project_id: ProjectId,
    pub task_id: Option<TaskId>,
    pub kind: CommandKind,
    pub payload: String,
    pub expected_projection_revision: Revision,
    pub idempotency_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ControlEvent {
    pub event_id: String,
    pub sequence: u64,
    pub project_id: ProjectId,
    pub task_id: Option<TaskId>,
    pub kind: String,
    pub payload: String,
    pub source_revision: Revision,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProjectionSnapshot {
    pub project_id: ProjectId,
    pub projection_revision: Revision,
    pub task_state: ProductLifecycleState,
    pub continuity_state: BackgroundContinuityState,
    pub preview_truth: PreviewTruth,
    pub current_source_revision: Revision,
    pub last_event_sequence: u64,
    pub last_known_good_ref: Option<String>,
    /// AGENTS §8 typed worker coordination projection (M8). Absent on
    /// snapshots produced by the in-memory plane and on persisted core
    /// snapshots; the durable control plane refreshes it from the ledger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_projection: Option<WorkerProjectionSummary>,
    /// AGENTS §8 typed artifact projection (latest durable Android build
    /// observation and its validated artifact identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_projection: Option<ArtifactProjectionSummary>,
    /// AGENTS §8 typed evidence projection (durable runtime-evidence census).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_projection: Option<EvidenceProjectionSummary>,
    /// AGENTS §8 typed delivery projection (latest durable APK delivery).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_projection: Option<DeliveryProjectionSummary>,
}

/// Worker coordination (M8) projection summary: the durable census of
/// coordination tasks, claims, handoffs, and acknowledgements for the
/// project, plus the still-open task identifiers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct WorkerProjectionSummary {
    pub task_count: u32,
    pub claim_count: u32,
    pub handoff_count: u32,
    pub acknowledged_handoff_count: u32,
    /// Distinct worker roles named by the durable coordination tasks.
    pub roles: Vec<String>,
    /// Coordination task ids that have not produced an acknowledged handoff
    /// yet (still being worked or awaiting reconciliation).
    pub open_task_ids: Vec<String>,
}

/// Artifact projection summary: the latest durable Android build
/// observation, including the validated artifact identity when the build
/// produced one.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArtifactProjectionSummary {
    pub task_id: String,
    pub source_revision: u64,
    pub build_variant: String,
    pub build_success: bool,
    pub timed_out: bool,
    pub cancelled: bool,
    pub artifact_path: Option<String>,
    pub artifact_sha256: Option<String>,
    pub project_fingerprint: String,
}

/// Evidence projection summary: the durable runtime-evidence census for the
/// project across M108 preview-sync events, captured evidence records, and
/// device observations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct EvidenceProjectionSummary {
    pub m108_event_count: u32,
    pub m108_evidence_count: u32,
    pub device_observation_count: u32,
    pub latest_observation_id: Option<String>,
    pub latest_device_identity: Option<String>,
}

/// Delivery projection summary: the latest durable APK delivery record,
/// including export state, destination identity, artifact fingerprint, and
/// the post-copy verification reference (AGENTS §8 `deliveryProjection`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeliveryProjectionSummary {
    pub delivery_id: String,
    pub task_id: String,
    pub source_revision: u64,
    /// DeliveryState name (PENDING/COPYING/COPIED/RECONCILING/VERIFIED/...).
    pub state: String,
    pub delivery_kind: String,
    pub destination_kind: String,
    pub destination_path: String,
    pub artifact_fingerprint: Option<String>,
    pub post_copy_verified: bool,
    pub copy_uncertain: bool,
    pub reconciliation_reference: Option<String>,
    pub failure_evidence_id: Option<String>,
    pub deployment_delivery: Option<String>,
    pub checkpoint_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    EmptyInstruction,
    StaleProjection {
        expected: Revision,
        current: Revision,
    },
    DuplicateCommand,
    InvalidTransition,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInstruction => write!(f, "instruction must not be empty"),
            Self::StaleProjection { expected, current } => write!(
                f,
                "stale projection: expected {expected:?}, current {current:?}"
            ),
            Self::DuplicateCommand => write!(f, "command was already accepted"),
            Self::InvalidTransition => write!(f, "invalid lifecycle transition"),
        }
    }
}

impl std::error::Error for DomainError {}

pub fn next_revision(revision: Revision) -> Revision {
    Revision(revision.0.saturating_add(1))
}

// ------------------------------------------------- M11 Android capability registry

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AndroidCapabilityRegistry {
    pub schema_version: u16,
    pub registry_id: String,
    pub compositions: Vec<TechnologyComposition>,
    pub toolchain_locks: Vec<ToolchainLock>,
    pub device_matrix: Vec<DeviceMatrixEntry>,
    pub fixtures: Vec<FixtureRecord>,
    pub known_exclusions: Vec<KnownExclusion>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TechnologyComposition {
    pub composition_id: String,
    pub language: String,
    pub ui_framework: String,
    pub runtime_layer: String,
    pub native_modules: Vec<String>,
    pub build_plugins: Vec<String>,
    pub device_apis: Vec<String>,
    pub mixed_architecture: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainLock {
    pub component: String,
    pub locked_version: String,
    pub compatible_range: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMatrixEntry {
    pub profile_id: String,
    pub form_factor: String,
    pub api_levels: Vec<u32>,
    pub composition_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FixtureRecord {
    pub fixture_id: String,
    pub composition_id: String,
    pub evidence_status: String,
    pub last_verified_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnownExclusion {
    pub exclusion_id: String,
    pub description: String,
    pub rationale: String,
}

pub const ANDROID_CAPABILITY_REGISTRY_SCHEMA: &str = "nirman.android_capability_registry.v1";

// ------------------------------------------------- M11 logs, install, reload status (work item 4)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AndroidLogEntry {
    pub schema_version: u16,
    pub entry_id: String,
    pub tag: String,
    pub level: LogEntryLevel,
    pub message: String,
    pub recorded_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogEntryLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Assert,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallStatus {
    pub schema_version: u16,
    pub device_id: String,
    pub package_name: String,
    pub state: InstallState,
    pub installed_at_epoch_seconds: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallState {
    Installing,
    Installed,
    Failed,
    Uninstalled,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReloadStatus {
    pub schema_version: u16,
    pub device_id: String,
    pub state: ReloadState,
    pub reloaded_at_epoch_seconds: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReloadState {
    Restarting,
    Restarted,
    Failed,
    Idle,
    Unknown,
}

pub const ANDROID_LOG_ENTRY_SCHEMA: &str = "nirman.android_log_entry.v1";
pub const INSTALL_STATUS_SCHEMA: &str = "nirman.install_status.v1";
pub const RELOAD_STATUS_SCHEMA: &str = "nirman.reload_status.v1";

// ------------------------------------------------- M11 APK delivery contract (work item 5)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackagingProfile {
    pub profile_id: String,
    pub artifact_kinds: Vec<ArtifactKind>,
    pub signing_required: bool,
    pub destination_kind: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApkDeliveryRecord {
    pub schema_version: u16,
    pub delivery_id: String,
    pub artifact_id: String,
    pub project_id: String,
    pub task_id: String,
    pub source_revision: u64,
    pub packaging_profile_id: String,
    pub artifact_kind: ArtifactKind,
    pub destination_path: String,
    pub destination_kind: String,
    pub request_fingerprint: String,
    pub idempotency_key: String,
    pub sha256: String,
    pub byte_count: u64,
    pub state: DeliveryState,
    /// Absolute path of the source artifact that was copied (spec §74.3
    /// `sourcePathReference`). Older records deserialize as `None`.
    #[serde(default)]
    pub source_path: Option<String>,
    /// sha256 of the source artifact, distinct from the destination hash in
    /// `sha256` (spec §74.3 `sourceArtifactHash`).
    #[serde(default)]
    pub source_sha256: Option<String>,
    /// Whether the durable post-copy verification (destination hash equality)
    /// has passed (spec §74.3 `postCopyCheck`).
    #[serde(default)]
    pub post_copy_verified: bool,
    /// Reference resolving an `Unknown`/interrupted copy back to the
    /// reconciliation decision (spec §74.3 `reconciliationReference`).
    #[serde(default)]
    pub reconciliation_reference: Option<String>,
    /// Evidence id produced when a copy fails (spec §74.3 `failureEvidenceId`).
    #[serde(default)]
    pub failure_evidence_id: Option<String>,
    /// REQUIRED_APK | DECLARED_AAB_OPTIONAL | SOURCE_ACCESS_ONLY
    /// (spec §74.3 `deploymentDelivery`).
    #[serde(default)]
    pub deployment_delivery: Option<String>,
    /// Durable checkpoint binding for the exported revision
    /// (spec §74.3 `checkpointId`).
    #[serde(default)]
    pub checkpoint_id: Option<String>,
    pub created_at_epoch_seconds: u64,
    pub completed_at_epoch_seconds: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryState {
    Pending,
    Copying,
    Copied,
    Reconciling,
    Verified,
    Failed,
    Blocked,
    Unknown,
}

pub const PACKAGING_PROFILE_SCHEMA: &str = "nirman.packaging_profile.v1";
pub const APK_DELIVERY_RECORD_SCHEMA: &str = "nirman.apk_delivery_record.v1";

// ------------------------------------------------- M11 signing configuration (work item 6)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SigningConfig {
    pub config_id: String,
    pub keystore_reference: String,
    pub key_alias: String,
    pub signing_scheme: SigningScheme,
    pub keystore_password_reference: Option<String>,
    pub key_password_reference: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SigningScheme {
    V1,
    V2,
    V1V2,
    Unknown,
}

pub const SIGNING_CONFIG_SCHEMA: &str = "nirman.signing_config.v1";

// ------------------------------------------------- M11 external-effect record (work item 4 generalization)
// Canonical ExternalEffectRecord + reconciliation lifecycle (ADR-203, TA §20.3).
// Generalization of the export UNKNOWN -> RECONCILING pattern to every external
// side effect (ADB, provider, signing, filesystem, process, remote API).

pub const EXTERNAL_EFFECT_RECORD_SCHEMA: &str = "nirman.external_effect_record.v1";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalEffectRecord {
    pub schema_version: u16,
    pub effect_id: String,
    pub operation_type: String,
    pub target_identity: String,
    pub request_fingerprint: String,
    pub authority_grant_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub request_state: ExternalEffectRequestState,
    #[serde(default)]
    pub response_reference: Option<String>,
    #[serde(default)]
    pub compensation_plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation_state: Option<String>,
    #[serde(default)]
    pub local_transaction_id: Option<String>,
    pub reconciliation_state: ExternalEffectReconciliationState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalEffectRequestState {
    NotSent,
    Sent,
    Acknowledged,
    Unknown,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalEffectReconciliationState {
    KnownSuccess,
    KnownFailure,
    Unknown,
    Reconciling,
    Resolved,
}

// ------------------------------------------------- M11 diagnostics schemas

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AndroidDiagnostic {
    pub schema_version: u16,
    pub diagnostic_id: String,
    pub kind: DiagnosticKind,
    pub status: DiagnosticStatus,
    pub message: String,
    pub remediation: Option<String>,
    pub detected_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticKind {
    Java,
    AndroidSdk,
    Emulator,
    Device,
    PackageManager,
    Gradle,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticStatus {
    Pass,
    Fail,
    Warn,
    Skipped,
    Unknown,
}

pub const ANDROID_DIAGNOSTIC_SCHEMA: &str = "nirman.android_diagnostic.v1";

// ------------------------------------------------- M11 device-manager abstraction

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSession {
    pub device_id: String,
    pub form_factor: String,
    pub api_level: u32,
    pub connection_state: ConnectionState,
    pub is_emulator: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Unauthorized,
    Offline,
    Unknown,
}

pub trait DeviceManager: Send + Sync {
    fn list_devices(&self) -> Vec<DeviceSession>;
    fn get_device(&self, device_id: &str) -> Option<DeviceSession>;
    fn connect(&mut self, device_id: &str) -> Result<(), AndroidResolverError>;
    fn disconnect(&mut self, device_id: &str) -> Result<(), AndroidResolverError>;
}

pub struct InMemoryDeviceManager {
    devices: Vec<DeviceSession>,
}

impl InMemoryDeviceManager {
    pub fn new(devices: Vec<DeviceSession>) -> Self {
        Self { devices }
    }
}

impl DeviceManager for InMemoryDeviceManager {
    fn list_devices(&self) -> Vec<DeviceSession> {
        self.devices.clone()
    }
    fn get_device(&self, device_id: &str) -> Option<DeviceSession> {
        self.devices
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned()
    }
    fn connect(&mut self, device_id: &str) -> Result<(), AndroidResolverError> {
        let device = self
            .devices
            .iter_mut()
            .find(|d| d.device_id == device_id)
            .ok_or(AndroidResolverError::EmptyField("device_id"))?;
        device.connection_state = ConnectionState::Connected;
        Ok(())
    }
    fn disconnect(&mut self, device_id: &str) -> Result<(), AndroidResolverError> {
        let device = self
            .devices
            .iter_mut()
            .find(|d| d.device_id == device_id)
            .ok_or(AndroidResolverError::EmptyField("device_id"))?;
        device.connection_state = ConnectionState::Disconnected;
        Ok(())
    }
}

pub const DEVICE_SESSION_SCHEMA: &str = "nirman.device_session.v1";

pub fn compose_technology_plan(
    registry: &AndroidCapabilityRegistry,
    composition_id: &str,
    task_id: TaskId,
    revision: Revision,
) -> Result<AndroidTechnologyPlan, AndroidResolverError> {
    let composition = registry
        .compositions
        .iter()
        .find(|c| c.composition_id == composition_id)
        .ok_or(AndroidResolverError::EmptyField("composition_id"))?;
    Ok(AndroidTechnologyPlan {
        plan_id: format!("plan-{composition_id}"),
        task_id,
        requested_capabilities: vec![],
        visual_requirements: vec![],
        selected_languages: vec![composition.language.clone()],
        selected_ui_frameworks: vec![composition.ui_framework.clone()],
        selected_runtime_layers: vec![composition.runtime_layer.clone()],
        selected_native_modules: composition.native_modules.clone(),
        selected_build_plugins: composition.build_plugins.clone(),
        selected_device_apis: composition.device_apis.clone(),
        selected_libraries: vec![],
        compatibility_constraints: vec![],
        rejected_alternatives: vec![],
        required_toolchains: vec![],
        validation_plan: vec![],
        confidence: None,
        revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_capability_registry_round_trips_serde() {
        let registry = AndroidCapabilityRegistry {
            schema_version: 1,
            registry_id: "registry-test".into(),
            compositions: vec![TechnologyComposition {
                composition_id: "compose-kotlin".into(),
                language: "kotlin".into(),
                ui_framework: "jetpack-compose".into(),
                runtime_layer: "art".into(),
                native_modules: vec!["camera".into()],
                build_plugins: vec!["kotlin-kapt".into()],
                device_apis: vec!["camera2".into()],
                mixed_architecture: false,
            }],
            toolchain_locks: vec![ToolchainLock {
                component: "gradle".into(),
                locked_version: "8.4".into(),
                compatible_range: ">= 8.0, < 9.0".into(),
            }],
            device_matrix: vec![DeviceMatrixEntry {
                profile_id: "phone-api-35".into(),
                form_factor: "phone".into(),
                api_levels: vec![35],
                composition_ids: vec!["compose-kotlin".into()],
            }],
            fixtures: vec![FixtureRecord {
                fixture_id: "fixture-compose-kotlin".into(),
                composition_id: "compose-kotlin".into(),
                evidence_status: "VERIFIED".into(),
                last_verified_at_epoch_seconds: 1_700_000_000,
            }],
            known_exclusions: vec![KnownExclusion {
                exclusion_id: "no-aab-without-keystore".into(),
                description: "AAB requires signing keystore".into(),
                rationale: "AAB is only produced when PackagingProfile requires APK_AND_AAB".into(),
            }],
        };
        let json = serde_json::to_string(&registry).expect("serialize");
        let back: AndroidCapabilityRegistry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(registry, back);
    }

    #[test]
    fn compose_technology_plan_from_registry() {
        let registry = AndroidCapabilityRegistry {
            schema_version: 1,
            registry_id: "registry-test".into(),
            compositions: vec![TechnologyComposition {
                composition_id: "compose-kotlin".into(),
                language: "kotlin".into(),
                ui_framework: "jetpack-compose".into(),
                runtime_layer: "art".into(),
                native_modules: vec!["camera".into()],
                build_plugins: vec!["kotlin-kapt".into()],
                device_apis: vec!["camera2".into()],
                mixed_architecture: false,
            }],
            toolchain_locks: vec![],
            device_matrix: vec![],
            fixtures: vec![],
            known_exclusions: vec![],
        };
        let plan = compose_technology_plan(
            &registry,
            "compose-kotlin",
            TaskId("task-1".into()),
            Revision(0),
        )
        .expect("compose");
        assert_eq!(plan.plan_id, "plan-compose-kotlin");
        assert_eq!(plan.selected_languages, vec!["kotlin"]);
        assert_eq!(plan.selected_ui_frameworks, vec!["jetpack-compose"]);
        assert_eq!(plan.selected_native_modules, vec!["camera"]);
    }

    #[test]
    fn android_diagnostic_round_trips_serde() {
        let diagnostic = AndroidDiagnostic {
            schema_version: 1,
            diagnostic_id: "diag-1".into(),
            kind: DiagnosticKind::AndroidSdk,
            status: DiagnosticStatus::Pass,
            message: "Android SDK found".into(),
            remediation: None,
            detected_at_epoch_seconds: 1_700_000_000,
        };
        let json = serde_json::to_string(&diagnostic).expect("serialize");
        let back: AndroidDiagnostic = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(diagnostic, back);
    }

    #[test]
    fn device_session_round_trips_serde() {
        let session = DeviceSession {
            device_id: "pixel-35".into(),
            form_factor: "phone".into(),
            api_level: 35,
            connection_state: ConnectionState::Connected,
            is_emulator: true,
        };
        let json = serde_json::to_string(&session).expect("serialize");
        let back: DeviceSession = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session, back);
    }

    #[test]
    fn in_memory_device_manager_lists_and_mutates() {
        let session = DeviceSession {
            device_id: "pixel-35".into(),
            form_factor: "phone".into(),
            api_level: 35,
            connection_state: ConnectionState::Disconnected,
            is_emulator: true,
        };
        let mut manager = InMemoryDeviceManager::new(vec![session.clone()]);
        assert_eq!(manager.list_devices().len(), 1);
        assert_eq!(
            manager.get_device("pixel-35").unwrap().connection_state,
            ConnectionState::Disconnected
        );
        manager.connect("pixel-35").expect("connect");
        assert_eq!(
            manager.get_device("pixel-35").unwrap().connection_state,
            ConnectionState::Connected
        );
        manager.disconnect("pixel-35").expect("disconnect");
        assert_eq!(
            manager.get_device("pixel-35").unwrap().connection_state,
            ConnectionState::Disconnected
        );
    }

    #[test]
    fn android_log_entry_round_trips_serde() {
        let entry = AndroidLogEntry {
            schema_version: 1,
            entry_id: "log-1".into(),
            tag: "TestTag".into(),
            level: LogEntryLevel::Info,
            message: "test message".into(),
            recorded_at_epoch_seconds: 1_700_000_000,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: AndroidLogEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(entry, back);
    }

    #[test]
    fn install_status_round_trips_serde() {
        let status = InstallStatus {
            schema_version: 1,
            device_id: "pixel-35".into(),
            package_name: "com.example.app".into(),
            state: InstallState::Installed,
            installed_at_epoch_seconds: Some(1_700_000_000),
            error_message: None,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let back: InstallStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, back);
    }

    #[test]
    fn reload_status_round_trips_serde() {
        let status = ReloadStatus {
            schema_version: 1,
            device_id: "pixel-35".into(),
            state: ReloadState::Restarted,
            reloaded_at_epoch_seconds: Some(1_700_000_000),
            error_message: None,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let back: ReloadStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, back);
    }

    #[test]
    fn packaging_profile_round_trips_serde() {
        let profile = PackagingProfile {
            profile_id: "profile-debug".into(),
            artifact_kinds: vec![ArtifactKind::Apk],
            signing_required: false,
            destination_kind: "LOCAL_WINDOWS_FILESYSTEM".into(),
        };
        let json = serde_json::to_string(&profile).expect("serialize");
        let back: PackagingProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, back);
    }

    #[test]
    fn apk_delivery_record_round_trips_serde() {
        let record = ApkDeliveryRecord {
            schema_version: 1,
            delivery_id: "delivery-1".into(),
            artifact_id: "artifact-1".into(),
            project_id: "project-1".into(),
            task_id: "task-1".into(),
            source_revision: 0,
            packaging_profile_id: "profile-debug".into(),
            artifact_kind: ArtifactKind::Apk,
            destination_path: "C:/build/app.apk".into(),
            destination_kind: "LOCAL_WINDOWS_FILESYSTEM".into(),
            request_fingerprint: "fp-1".into(),
            idempotency_key: "idem-1".into(),
            sha256: "sha256:abc".into(),
            byte_count: 1024,
            state: DeliveryState::Verified,
            source_path: Some("C:/build/app-release.apk".into()),
            source_sha256: Some("sha256:abc".into()),
            post_copy_verified: true,
            reconciliation_reference: None,
            failure_evidence_id: None,
            deployment_delivery: Some("REQUIRED_APK".into()),
            checkpoint_id: Some("checkpoint-1".into()),
            created_at_epoch_seconds: 1_700_000_000,
            completed_at_epoch_seconds: Some(1_700_000_100),
            error_message: None,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: ApkDeliveryRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, back);
    }

    #[test]
    fn apk_delivery_record_deserializes_legacy_v1_without_export_provenance() {
        let legacy = serde_json::json!({
            "schemaVersion": 1,
            "deliveryId": "delivery-legacy",
            "artifactId": "artifact-1",
            "projectId": "project-1",
            "taskId": "task-1",
            "sourceRevision": 0,
            "packagingProfileId": "profile-debug",
            "artifactKind": "apk",
            "destinationPath": "C:/build/app.apk",
            "destinationKind": "LOCAL_WINDOWS_FILESYSTEM",
            "requestFingerprint": "fp-1",
            "idempotencyKey": "idem-1",
            "sha256": "sha256:abc",
            "byteCount": 1024,
            "state": "COPIED",
            "createdAtEpochSeconds": 1_700_000_000,
            "completedEpochSeconds": null,
            "errorMessage": null
        });
        let record: ApkDeliveryRecord =
            serde_json::from_value(legacy).expect("legacy record deserializes");
        assert_eq!(record.state, DeliveryState::Copied);
        assert!(!record.post_copy_verified);
        assert_eq!(record.source_path, None);
        assert_eq!(record.deployment_delivery, None);
    }

    #[test]
    fn signing_config_round_trips_serde() {
        let config = SigningConfig {
            config_id: "signing-debug".into(),
            keystore_reference: "keychain://nirman/keystore/debug".into(),
            key_alias: "debug-key".into(),
            signing_scheme: SigningScheme::V1V2,
            keystore_password_reference: Some("keychain://nirman/keystore/debug/password".into()),
            key_password_reference: Some("keychain://nirman/key/debug/password".into()),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let back: SigningConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, back);
    }

    #[test]
    fn external_effect_record_round_trips_serde() {
        let record = ExternalEffectRecord {
            schema_version: 1,
            effect_id: "effect-1".into(),
            operation_type: "ADB_INSTALL".into(),
            target_identity: "emulator-5554".into(),
            request_fingerprint: "fp-1".into(),
            authority_grant_id: Some("grant-1".into()),
            idempotency_key: Some("idem-1".into()),
            request_state: ExternalEffectRequestState::Acknowledged,
            response_reference: None,
            compensation_plan: None,
            compensation_state: None,
            local_transaction_id: Some("txn-1".into()),
            reconciliation_state: ExternalEffectReconciliationState::Reconciling,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: ExternalEffectRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, back);
        assert!(json.contains("\"reconciliationState\":\"RECONCILING\""));
        assert!(json.contains("\"requestState\":\"ACKNOWLEDGED\""));
    }
}
