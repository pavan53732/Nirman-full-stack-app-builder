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
    Implementing,
    Paused,
    Previewing,
    Validating,
    Recovering,
    Packaging,
    Completed,
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
    ProviderExecute,
    SubmitInstruction,
    Reconnect,
    PauseTask,
    ResumeTask,
    CancelTask,
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
