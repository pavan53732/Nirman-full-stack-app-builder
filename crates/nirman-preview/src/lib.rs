#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

pub const M48_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewMode {
    IncrementalEmulatorInstall,
    ComposeReload,
    ReactNativeExpoRefresh,
    ApkReinstall,
    PhysicalDevice,
    HeadlessSmokeTest,
    Diagnostic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewExecutionTruth {
    Predicted,
    Simulated,
    Requested,
    Observed,
    Verified,
    Stale,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewLifecycleState {
    NotRequested,
    RequestAuthorized,
    Building,
    BuildObserved,
    Installing,
    InstallObserved,
    Launching,
    RunningObserved,
    InteractionObserved,
    Validating,
    PromotedCurrent,
    FailedCandidate,
    Stale,
    Invalidated,
    Recovering,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewStatus {
    Pending,
    Observed,
    Failed,
    Stale,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewRequest {
    pub schema_version: u16,
    pub request_id: String,
    pub project_id: String,
    pub task_id: String,
    pub project_revision_id: String,
    pub checkpoint_id: String,
    pub source_fingerprint: String,
    pub contract_version: String,
    pub technology_plan_version: String,
    pub asset_manifest_version: String,
    pub build_variant: String,
    pub device_id: Option<String>,
    pub android_api_level: Option<u32>,
    pub requested_mode: Option<PreviewMode>,
    pub selected_language: String,
    pub selected_ui_framework: String,
    pub changed_paths: Vec<String>,
    pub required_evidence_kinds: Vec<String>,
    pub policy_decision_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewFallbackSelection {
    pub schema_version: u16,
    pub request_id: String,
    pub mode: PreviewMode,
    pub reason: String,
    pub selection_rank: u8,
    pub runtime_observation_required: bool,
    pub evidence_kinds: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewRevision {
    pub schema_version: u16,
    pub preview_revision_id: String,
    pub project_id: String,
    pub task_id: String,
    pub project_revision_id: String,
    pub checkpoint_id: String,
    pub source_fingerprint: String,
    pub artifact_id: Option<String>,
    pub artifact_fingerprint: Option<String>,
    pub device_id: Option<String>,
    pub android_api_level: Option<u32>,
    pub build_variant: String,
    pub preview_mode: PreviewMode,
    pub technology_plan_version: String,
    pub asset_manifest_version: String,
    pub lifecycle_state: PreviewLifecycleState,
    pub execution_truth: PreviewExecutionTruth,
    pub status: PreviewStatus,
    pub build_status: String,
    pub install_status: String,
    pub launch_status: String,
    pub runtime_status: String,
    pub validation_status: String,
    pub evidence_ids: Vec<String>,
    pub created_at_epoch_seconds: u64,
    pub observed_at_epoch_seconds: Option<u64>,
    pub invalidated_at_epoch_seconds: Option<u64>,
    pub invalidated_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewProjection {
    pub schema_version: u16,
    pub project_id: String,
    pub task_id: String,
    pub active_last_known_good: Option<PreviewRevision>,
    pub candidate: Option<PreviewRevision>,
    pub stale_reasons: Vec<String>,
}

pub const M108_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewSyncEventType {
    IntentAccepted,
    ContractValidated,
    PlanRecorded,
    CheckpointCreated,
    SourceRevisionCommitted,
    BuildRequested,
    BuildObserved,
    ArtifactObserved,
    InstallRequested,
    InstallObserved,
    LaunchObserved,
    InteractionObserved,
    ObservationCaptured,
    ValidationObserved,
    CandidateFailed,
    PreviewInvalidated,
    PreviewPromoted,
    StreamGap,
    StreamReconnected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewEventTruth {
    Predicted,
    Simulated,
    Requested,
    Observed,
    Verified,
    Stale,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewSyncEvent {
    pub event_id: String,
    pub event_sequence: u64,
    pub project_id: String,
    pub task_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub candidate_preview_revision_id: String,
    pub event_type: PreviewSyncEventType,
    pub event_truth: PreviewEventTruth,
    pub project_revision_id: String,
    pub checkpoint_id: String,
    pub source_fingerprint: String,
    pub artifact_id: Option<String>,
    pub artifact_fingerprint: Option<String>,
    pub runtime_session_id: Option<String>,
    pub device_id: Option<String>,
    pub operation_ref: String,
    pub observation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub validation_ref: Option<String>,
    pub payload: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewSyncEvidenceRecord {
    pub evidence_id: String,
    pub event_sequence_start: u64,
    pub event_sequence_end: u64,
    pub projection_revision: u64,
    pub preview_revision_id: String,
    pub artifact_fingerprint: Option<String>,
    pub device_id: Option<String>,
    pub observation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub validation_refs: Vec<String>,
    pub truth: PreviewEventTruth,
}

impl PreviewSyncEvent {
    pub fn validate(&self) -> Result<(), PreviewError> {
        for (field, value) in [
            ("eventId", &self.event_id),
            ("projectId", &self.project_id),
            ("taskId", &self.task_id),
            ("correlationId", &self.correlation_id),
            (
                "candidatePreviewRevisionId",
                &self.candidate_preview_revision_id,
            ),
            ("projectRevisionId", &self.project_revision_id),
            ("checkpointId", &self.checkpoint_id),
            ("sourceFingerprint", &self.source_fingerprint),
            ("operationRef", &self.operation_ref),
        ] {
            if value.trim().is_empty() {
                return Err(PreviewError::EmptyField(field));
            }
        }
        if self.event_sequence == 0 {
            return Err(PreviewError::IdentityMismatch("eventSequence"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewPromotionEligibility {
    pub eligible: bool,
    pub reason: String,
    pub required_observations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewError {
    EmptyField(&'static str),
    UnsupportedSchemaVersion,
    IdentityMismatch(&'static str),
    InvalidTransition {
        from: PreviewLifecycleState,
        to: PreviewLifecycleState,
    },
    StaleRevision(Vec<String>),
    RuntimeObservationRequired,
    CandidateCannotReplaceKnownGood,
}

impl fmt::Display for PreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "M48 field is empty: {field}"),
            Self::UnsupportedSchemaVersion => {
                formatter.write_str("M48 schema version is unsupported")
            }
            Self::IdentityMismatch(field) => write!(formatter, "M48 identity mismatch: {field}"),
            Self::InvalidTransition { from, to } => {
                write!(
                    formatter,
                    "M48 invalid preview transition: {from:?} -> {to:?}"
                )
            }
            Self::StaleRevision(reasons) => {
                write!(
                    formatter,
                    "M48 preview revision is stale: {}",
                    reasons.join(",")
                )
            }
            Self::RuntimeObservationRequired => {
                formatter.write_str("M48 runtime observation is required")
            }
            Self::CandidateCannotReplaceKnownGood => formatter
                .write_str("M48 failed or stale candidate cannot replace last-known-good preview"),
        }
    }
}
impl std::error::Error for PreviewError {}

impl PreviewRequest {
    pub fn validate(&self) -> Result<(), PreviewError> {
        if self.schema_version != M48_SCHEMA_VERSION {
            return Err(PreviewError::UnsupportedSchemaVersion);
        }
        for (field, value) in [
            ("requestId", self.request_id.as_str()),
            ("projectId", self.project_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("projectRevisionId", self.project_revision_id.as_str()),
            ("checkpointId", self.checkpoint_id.as_str()),
            ("sourceFingerprint", self.source_fingerprint.as_str()),
            ("contractVersion", self.contract_version.as_str()),
            (
                "technologyPlanVersion",
                self.technology_plan_version.as_str(),
            ),
            ("assetManifestVersion", self.asset_manifest_version.as_str()),
            ("buildVariant", self.build_variant.as_str()),
            ("selectedLanguage", self.selected_language.as_str()),
            ("selectedUiFramework", self.selected_ui_framework.as_str()),
            ("policyDecisionId", self.policy_decision_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(PreviewError::EmptyField(field));
            }
        }
        if self.android_api_level == Some(0) {
            return Err(PreviewError::EmptyField("androidApiLevel"));
        }
        if self
            .device_id
            .as_ref()
            .is_some_and(|device| device.trim().is_empty())
        {
            return Err(PreviewError::EmptyField("deviceId"));
        }
        if matches!(
            self.requested_mode,
            Some(
                PreviewMode::IncrementalEmulatorInstall
                    | PreviewMode::ComposeReload
                    | PreviewMode::ReactNativeExpoRefresh
                    | PreviewMode::ApkReinstall
                    | PreviewMode::PhysicalDevice
            )
        ) && (self.device_id.is_none() || self.android_api_level.is_none())
        {
            return Err(PreviewError::IdentityMismatch(
                "device-bound preview mode requires deviceId and androidApiLevel",
            ));
        }
        Ok(())
    }
}

impl PreviewRevision {
    pub fn validate(&self) -> Result<(), PreviewError> {
        if self.schema_version != M48_SCHEMA_VERSION {
            return Err(PreviewError::UnsupportedSchemaVersion);
        }
        for (field, value) in [
            ("previewRevisionId", self.preview_revision_id.as_str()),
            ("projectId", self.project_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("projectRevisionId", self.project_revision_id.as_str()),
            ("checkpointId", self.checkpoint_id.as_str()),
            ("sourceFingerprint", self.source_fingerprint.as_str()),
            ("buildVariant", self.build_variant.as_str()),
            (
                "technologyPlanVersion",
                self.technology_plan_version.as_str(),
            ),
            ("assetManifestVersion", self.asset_manifest_version.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(PreviewError::EmptyField(field));
            }
        }
        if self.device_id.is_some() != self.android_api_level.is_some()
            && self.preview_mode != PreviewMode::HeadlessSmokeTest
            && self.preview_mode != PreviewMode::Diagnostic
        {
            return Err(PreviewError::IdentityMismatch("deviceId/androidApiLevel"));
        }
        if self.execution_truth == PreviewExecutionTruth::Observed
            && !matches!(
                self.lifecycle_state,
                PreviewLifecycleState::RunningObserved
                    | PreviewLifecycleState::InteractionObserved
                    | PreviewLifecycleState::Validating
                    | PreviewLifecycleState::PromotedCurrent
            )
        {
            return Err(PreviewError::RuntimeObservationRequired);
        }
        Ok(())
    }

    pub fn stale_reasons(&self, request: &PreviewRequest) -> Vec<String> {
        let mut reasons = Vec::new();
        if self.project_id != request.project_id {
            reasons.push("project-id".into());
        }
        if self.task_id != request.task_id {
            reasons.push("task-id".into());
        }
        if self.project_revision_id != request.project_revision_id {
            reasons.push("project-revision".into());
        }
        if self.checkpoint_id != request.checkpoint_id {
            reasons.push("checkpoint".into());
        }
        if self.source_fingerprint != request.source_fingerprint {
            reasons.push("source-fingerprint".into());
        }
        if self.technology_plan_version != request.technology_plan_version {
            reasons.push("technology-plan".into());
        }
        if self.asset_manifest_version != request.asset_manifest_version {
            reasons.push("asset-manifest".into());
        }
        if self.build_variant != request.build_variant {
            reasons.push("build-variant".into());
        }
        if self.device_id != request.device_id {
            reasons.push("device-id".into());
        }
        if self.android_api_level != request.android_api_level {
            reasons.push("android-api-level".into());
        }
        reasons
    }

    pub fn can_promote(&self) -> PreviewPromotionEligibility {
        let mut missing = Vec::new();
        if self.execution_truth != PreviewExecutionTruth::Observed {
            missing.push("observed-execution-truth".into());
        }
        if self.build_status != "OBSERVED_SUCCESS" {
            missing.push("build-observation".into());
        }
        if self.install_status != "OBSERVED_SUCCESS" {
            missing.push("install-observation".into());
        }
        if self.launch_status != "OBSERVED_SUCCESS" {
            missing.push("launch-observation".into());
        }
        if self.runtime_status != "OBSERVED_RUNNING" {
            missing.push("runtime-observation".into());
        }
        if self.validation_status != "OBSERVED_PASS" {
            missing.push("validation-observation".into());
        }
        PreviewPromotionEligibility {
            eligible: missing.is_empty(),
            reason: if missing.is_empty() {
                "all required runtime observations are present".into()
            } else {
                "candidate is not promotable without current supervised observations".into()
            },
            required_observations: missing,
        }
    }
}

pub fn select_fallback(request: &PreviewRequest) -> Result<PreviewFallbackSelection, PreviewError> {
    request.validate()?;
    let framework = request.selected_ui_framework.to_ascii_lowercase();
    let changed_only_resources = !request.changed_paths.is_empty()
        && request.changed_paths.iter().all(|path| {
            let path = path.to_ascii_lowercase();
            path.contains("/res/") || path.contains("\\res\\") || path.ends_with(".xml")
        });
    let (mode, reason, rank, runtime_required, evidence_kinds) =
        if let Some(mode) = &request.requested_mode {
            (
                mode.clone(),
                "explicit policy-selected preview mode".into(),
                1,
                !matches!(
                    mode,
                    PreviewMode::HeadlessSmokeTest | PreviewMode::Diagnostic
                ),
                vec!["PROCESS_EVIDENCE".into()],
            )
        } else if framework.contains("compose") && changed_only_resources {
            (
                PreviewMode::ComposeReload,
                "Compose-compatible resource-only change".into(),
                1,
                true,
                vec!["PROCESS_EVIDENCE".into(), "VISUAL_EVIDENCE".into()],
            )
        } else if framework.contains("react") || framework.contains("expo") {
            (
                PreviewMode::ReactNativeExpoRefresh,
                "selected Android technology plan uses React Native or Expo".into(),
                1,
                true,
                vec!["PROCESS_EVIDENCE".into(), "DEVICE_EVIDENCE".into()],
            )
        } else if request.device_id.is_some() {
            (
                PreviewMode::IncrementalEmulatorInstall,
                "device-bound source revision can use incremental installation".into(),
                2,
                true,
                vec!["PROCESS_EVIDENCE".into(), "DEVICE_EVIDENCE".into()],
            )
        } else {
            (
                PreviewMode::HeadlessSmokeTest,
                "no eligible connected preview device is declared".into(),
                3,
                false,
                vec!["PROCESS_EVIDENCE".into(), "TEST_EVIDENCE".into()],
            )
        };
    Ok(PreviewFallbackSelection {
        schema_version: M48_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        mode,
        reason,
        selection_rank: rank,
        runtime_observation_required: runtime_required,
        evidence_kinds,
    })
}

pub fn bind_preview_revision(
    request: &PreviewRequest,
    selection: &PreviewFallbackSelection,
    preview_revision_id: &str,
    now_epoch_seconds: u64,
) -> Result<PreviewRevision, PreviewError> {
    request.validate()?;
    if selection.request_id != request.request_id {
        return Err(PreviewError::IdentityMismatch("requestId"));
    }
    if preview_revision_id.trim().is_empty() {
        return Err(PreviewError::EmptyField("previewRevisionId"));
    }
    let revision = PreviewRevision {
        schema_version: M48_SCHEMA_VERSION,
        preview_revision_id: preview_revision_id.into(),
        project_id: request.project_id.clone(),
        task_id: request.task_id.clone(),
        project_revision_id: request.project_revision_id.clone(),
        checkpoint_id: request.checkpoint_id.clone(),
        source_fingerprint: request.source_fingerprint.clone(),
        artifact_id: None,
        artifact_fingerprint: None,
        device_id: request.device_id.clone(),
        android_api_level: request.android_api_level,
        build_variant: request.build_variant.clone(),
        preview_mode: selection.mode.clone(),
        technology_plan_version: request.technology_plan_version.clone(),
        asset_manifest_version: request.asset_manifest_version.clone(),
        lifecycle_state: PreviewLifecycleState::RequestAuthorized,
        execution_truth: PreviewExecutionTruth::Requested,
        status: PreviewStatus::Pending,
        build_status: "NOT_OBSERVED".into(),
        install_status: "NOT_OBSERVED".into(),
        launch_status: "NOT_OBSERVED".into(),
        runtime_status: "NOT_OBSERVED".into(),
        validation_status: "NOT_OBSERVED".into(),
        evidence_ids: Vec::new(),
        created_at_epoch_seconds: now_epoch_seconds,
        observed_at_epoch_seconds: None,
        invalidated_at_epoch_seconds: None,
        invalidated_reason: None,
    };
    revision.validate()?;
    Ok(revision)
}

pub fn transition_preview(
    revision: &mut PreviewRevision,
    next: PreviewLifecycleState,
) -> Result<(), PreviewError> {
    let valid = match (&revision.lifecycle_state, &next) {
        (PreviewLifecycleState::NotRequested, PreviewLifecycleState::RequestAuthorized)
        | (PreviewLifecycleState::RequestAuthorized, PreviewLifecycleState::Building)
        | (PreviewLifecycleState::Building, PreviewLifecycleState::BuildObserved)
        | (PreviewLifecycleState::BuildObserved, PreviewLifecycleState::Installing)
        | (PreviewLifecycleState::Installing, PreviewLifecycleState::InstallObserved)
        | (PreviewLifecycleState::InstallObserved, PreviewLifecycleState::Launching)
        | (PreviewLifecycleState::Launching, PreviewLifecycleState::RunningObserved)
        | (PreviewLifecycleState::RunningObserved, PreviewLifecycleState::InteractionObserved)
        | (PreviewLifecycleState::InteractionObserved, PreviewLifecycleState::Validating)
        | (PreviewLifecycleState::Validating, PreviewLifecycleState::PromotedCurrent)
        | (_, PreviewLifecycleState::FailedCandidate)
        | (_, PreviewLifecycleState::Stale)
        | (_, PreviewLifecycleState::Invalidated)
        | (_, PreviewLifecycleState::Recovering)
        | (PreviewLifecycleState::Recovering, PreviewLifecycleState::RequestAuthorized) => true,
        _ => false,
    };
    if !valid {
        return Err(PreviewError::InvalidTransition {
            from: revision.lifecycle_state.clone(),
            to: next,
        });
    }
    revision.lifecycle_state = next.clone();
    match next {
        PreviewLifecycleState::BuildObserved => revision.build_status = "OBSERVED_SUCCESS".into(),
        PreviewLifecycleState::InstallObserved => {
            revision.install_status = "OBSERVED_SUCCESS".into()
        }
        PreviewLifecycleState::RunningObserved => {
            revision.launch_status = "OBSERVED_SUCCESS".into();
            revision.runtime_status = "OBSERVED_RUNNING".into();
            revision.execution_truth = PreviewExecutionTruth::Observed;
            revision.status = PreviewStatus::Observed;
        }
        PreviewLifecycleState::Validating => revision.validation_status = "RUNNING".into(),
        PreviewLifecycleState::PromotedCurrent => {
            revision.validation_status = "OBSERVED_PASS".into();
            revision.execution_truth = PreviewExecutionTruth::Verified;
            revision.status = PreviewStatus::Observed;
        }
        PreviewLifecycleState::FailedCandidate => revision.status = PreviewStatus::Failed,
        PreviewLifecycleState::Stale => {
            revision.status = PreviewStatus::Stale;
            revision.execution_truth = PreviewExecutionTruth::Stale;
        }
        PreviewLifecycleState::Invalidated => {
            revision.status = PreviewStatus::Invalidated;
            revision.execution_truth = PreviewExecutionTruth::Invalidated;
        }
        _ => {}
    }
    Ok(())
}

impl PreviewProjection {
    pub fn new(project_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            schema_version: M48_SCHEMA_VERSION,
            project_id: project_id.into(),
            task_id: task_id.into(),
            active_last_known_good: None,
            candidate: None,
            stale_reasons: Vec::new(),
        }
    }

    pub fn apply_candidate(
        &mut self,
        candidate: PreviewRevision,
        current_request: &PreviewRequest,
    ) -> Result<(), PreviewError> {
        candidate.validate()?;
        if candidate.project_id != self.project_id || candidate.task_id != self.task_id {
            return Err(PreviewError::IdentityMismatch("project/task"));
        }
        let stale = candidate.stale_reasons(current_request);
        if !stale.is_empty() {
            self.stale_reasons = stale.clone();
            return Err(PreviewError::StaleRevision(stale));
        }
        self.candidate = Some(candidate);
        Ok(())
    }

    pub fn promote_candidate(&mut self) -> Result<(), PreviewError> {
        let candidate = self
            .candidate
            .as_ref()
            .ok_or(PreviewError::CandidateCannotReplaceKnownGood)?;
        if !candidate.can_promote().eligible {
            return Err(PreviewError::RuntimeObservationRequired);
        }
        self.active_last_known_good = self.candidate.take();
        Ok(())
    }

    pub fn preserve_known_good_on_failure(&mut self) -> Result<(), PreviewError> {
        let candidate = self
            .candidate
            .as_mut()
            .ok_or(PreviewError::CandidateCannotReplaceKnownGood)?;
        transition_preview(candidate, PreviewLifecycleState::FailedCandidate)
    }

    pub fn mark_active_stale(&mut self, current_request: &PreviewRequest) {
        self.stale_reasons = self
            .active_last_known_good
            .as_ref()
            .map(|revision| revision.stale_reasons(current_request))
            .unwrap_or_default();
        if !self.stale_reasons.is_empty() {
            if let Some(active) = self.active_last_known_good.as_mut() {
                let _ = transition_preview(active, PreviewLifecycleState::Stale);
            }
        }
    }

    pub fn identity_fingerprint(&self) -> String {
        let mut parts = BTreeSet::new();
        if let Some(active) = &self.active_last_known_good {
            parts.insert(format!(
                "active:{}:{}",
                active.project_revision_id, active.source_fingerprint
            ));
        }
        if let Some(candidate) = &self.candidate {
            parts.insert(format!(
                "candidate:{}:{}",
                candidate.project_revision_id, candidate.source_fingerprint
            ));
        }
        parts.into_iter().collect::<Vec<_>>().join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PreviewRequest {
        PreviewRequest {
            schema_version: M48_SCHEMA_VERSION,
            request_id: "preview-request-1".into(),
            project_id: "project-1".into(),
            task_id: "task-1".into(),
            project_revision_id: "revision-1".into(),
            checkpoint_id: "checkpoint-1".into(),
            source_fingerprint: "source-1".into(),
            contract_version: "contract-1".into(),
            technology_plan_version: "technology-1".into(),
            asset_manifest_version: "assets-1".into(),
            build_variant: "debug".into(),
            device_id: Some("emulator-1".into()),
            android_api_level: Some(35),
            requested_mode: None,
            selected_language: "kotlin".into(),
            selected_ui_framework: "Jetpack Compose".into(),
            changed_paths: vec!["app/src/main/res/values/strings.xml".into()],
            required_evidence_kinds: vec!["DEVICE_EVIDENCE".into(), "VISUAL_EVIDENCE".into()],
            policy_decision_id: "policy-1".into(),
        }
    }

    #[test]
    fn fallback_selection_is_deterministic_and_technology_aware() {
        let selection = select_fallback(&request()).expect("selection");
        assert_eq!(selection.mode, PreviewMode::ComposeReload);
        assert_eq!(selection.selection_rank, 1);
        assert!(selection.runtime_observation_required);
        let serialized = serde_json::to_string(&selection).expect("selection JSON");
        assert_eq!(
            selection,
            serde_json::from_str(&serialized).expect("reload")
        );
    }

    #[test]
    fn revision_binding_and_stale_detection_cover_all_identity_dimensions() {
        let request = request();
        let selection = select_fallback(&request).expect("selection");
        let revision =
            bind_preview_revision(&request, &selection, "preview-revision-1", 10).expect("binding");
        assert_eq!(revision.project_revision_id, request.project_revision_id);
        assert!(revision.can_promote().required_observations.len() >= 5);
        let mut newer = request.clone();
        newer.source_fingerprint = "source-2".into();
        newer.device_id = Some("emulator-2".into());
        let reasons = revision.stale_reasons(&newer);
        assert!(reasons.contains(&"source-fingerprint".into()));
        assert!(reasons.contains(&"device-id".into()));
    }

    #[test]
    fn lifecycle_requires_observations_and_preserves_last_known_good() {
        let request = request();
        let selection = select_fallback(&request).expect("selection");
        let mut good = bind_preview_revision(&request, &selection, "good", 1).expect("good");
        for next in [
            PreviewLifecycleState::Building,
            PreviewLifecycleState::BuildObserved,
            PreviewLifecycleState::Installing,
            PreviewLifecycleState::InstallObserved,
            PreviewLifecycleState::Launching,
            PreviewLifecycleState::RunningObserved,
            PreviewLifecycleState::InteractionObserved,
            PreviewLifecycleState::Validating,
        ] {
            transition_preview(&mut good, next).expect("valid transition");
        }
        good.validation_status = "OBSERVED_PASS".into();
        let mut projection = PreviewProjection::new("project-1", "task-1");
        projection
            .apply_candidate(good, &request)
            .expect("candidate");
        projection.promote_candidate().expect("promotion");
        let active_id = projection
            .active_last_known_good
            .as_ref()
            .expect("active")
            .preview_revision_id
            .clone();
        let failed = bind_preview_revision(&request, &selection, "failed", 2).expect("failed");
        projection
            .apply_candidate(failed, &request)
            .expect("candidate");
        projection
            .preserve_known_good_on_failure()
            .expect("failure");
        assert_eq!(
            projection
                .active_last_known_good
                .as_ref()
                .unwrap()
                .preview_revision_id,
            active_id
        );
        assert_eq!(
            projection.candidate.as_ref().unwrap().status,
            PreviewStatus::Failed
        );
    }

    #[test]
    fn predicted_or_simulated_revision_cannot_promote() {
        let request = request();
        let selection = select_fallback(&request).expect("selection");
        let revision =
            bind_preview_revision(&request, &selection, "predicted", 1).expect("revision");
        let mut projection = PreviewProjection::new("project-1", "task-1");
        projection
            .apply_candidate(revision, &request)
            .expect("candidate");
        assert_eq!(
            projection
                .promote_candidate()
                .expect_err("blocked promotion"),
            PreviewError::RuntimeObservationRequired
        );
    }
}
