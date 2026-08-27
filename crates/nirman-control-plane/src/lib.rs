//! Authoritative local control-plane primitives for M2.
//!
//! The in-memory `ControlPlane` owns command acceptance and projection rules.
//! `DurableControlPlane` wires that path to the SQLite ledger used for restart
//! recovery. Frontend code receives snapshots and events; it never owns truth.

#![forbid(unsafe_code)]

use nirman_domain::{
    next_revision, BackgroundContinuityState, CommandEnvelope, CommandKind, ControlEvent,
    DomainError, MutationTransactionRecord, PreviewTruth, ProductLifecycleState, ProjectId,
    ProjectionSnapshot, Revision, TaskId,
};
use nirman_storage::Ledger;
use std::path::Path;

pub fn deadline_elapsed(deadline_epoch_seconds: Option<u64>, now_epoch_seconds: u64) -> bool {
    deadline_epoch_seconds.is_some_and(|deadline| deadline <= now_epoch_seconds)
}

#[derive(Clone, Debug)]
pub struct ControlPlane {
    project_id: ProjectId,
    projection: ProjectionSnapshot,
    events: Vec<ControlEvent>,
    accepted_idempotency_keys: Vec<String>,
    next_sequence: u64,
}

impl ControlPlane {
    pub fn new(project_id: ProjectId) -> Self {
        Self::from_snapshot(
            ProjectionSnapshot {
                project_id: project_id.clone(),
                projection_revision: Revision(0),
                task_state: ProductLifecycleState::Created,
                continuity_state: BackgroundContinuityState::ActiveBackground,
                preview_truth: PreviewTruth::Predicted,
                current_source_revision: Revision(0),
                last_event_sequence: 0,
                last_known_good_ref: None,
            },
            Vec::new(),
        )
    }

    pub fn from_snapshot(snapshot: ProjectionSnapshot, events: Vec<ControlEvent>) -> Self {
        let project_id = snapshot.project_id.clone();
        let next_sequence = events
            .iter()
            .map(|event| event.sequence)
            .max()
            .unwrap_or(snapshot.last_event_sequence)
            .saturating_add(1);
        Self {
            project_id,
            projection: snapshot,
            events,
            accepted_idempotency_keys: Vec::new(),
            next_sequence,
        }
    }

    pub fn snapshot(&self) -> ProjectionSnapshot {
        self.projection.clone()
    }

    pub fn events_after(&self, sequence: u64) -> Vec<ControlEvent> {
        self.events
            .iter()
            .filter(|event| event.sequence > sequence)
            .cloned()
            .collect()
    }

    pub fn accept(&mut self, command: CommandEnvelope) -> Result<ProjectionSnapshot, DomainError> {
        if command.project_id != self.project_id {
            return Err(DomainError::StaleProjection {
                expected: self.projection.projection_revision,
                current: self.projection.projection_revision,
            });
        }
        if command.expected_projection_revision != self.projection.projection_revision {
            return Err(DomainError::StaleProjection {
                expected: command.expected_projection_revision,
                current: self.projection.projection_revision,
            });
        }
        if let Some(key) = &command.idempotency_key {
            if self
                .accepted_idempotency_keys
                .iter()
                .any(|existing| existing == key)
            {
                return Err(DomainError::DuplicateCommand);
            }
        }
        if matches!(
            command.kind,
            CommandKind::SubmitInstruction | CommandKind::TaskStart
        ) && command.payload.trim().is_empty()
        {
            return Err(DomainError::EmptyInstruction);
        }

        match command.kind {
            CommandKind::ProjectOpen => {}
            CommandKind::TaskStart | CommandKind::SubmitInstruction => {
                self.projection.task_state = ProductLifecycleState::Planning;
                self.projection.preview_truth = PreviewTruth::Requested;
                self.projection.current_source_revision =
                    next_revision(self.projection.current_source_revision);
            }
            CommandKind::TaskCancel | CommandKind::CancelTask => {
                if self.projection.task_state == ProductLifecycleState::Completed {
                    return Err(DomainError::InvalidTransition);
                }
                self.projection.task_state = ProductLifecycleState::Cancelled;
            }
            CommandKind::TaskResume | CommandKind::ResumeTask => {
                if !matches!(
                    self.projection.task_state,
                    ProductLifecycleState::Paused | ProductLifecycleState::SafelyFailed
                ) {
                    return Err(DomainError::InvalidTransition);
                }
                self.projection.task_state = ProductLifecycleState::Planning;
            }
            CommandKind::WorkspaceApplyPatch => {
                self.projection.task_state = ProductLifecycleState::Implementing;
                self.projection.current_source_revision =
                    next_revision(self.projection.current_source_revision);
            }
            CommandKind::PreviewStart => {
                self.projection.task_state = ProductLifecycleState::Previewing;
                self.projection.preview_truth = PreviewTruth::Requested;
            }
            CommandKind::PreviewStop => {
                self.projection.preview_truth = PreviewTruth::Stale;
            }
            CommandKind::PreviewPromote
            | CommandKind::ArtifactExport
            | CommandKind::ProviderTest
            | CommandKind::SettingsUpdateProvider
            | CommandKind::AndroidConstructionCreate
            | CommandKind::AndroidToolchainPreflight
            | CommandKind::AndroidRequirementEvaluate
            | CommandKind::AndroidSynthesisBuild
            | CommandKind::ProviderExecute => {}
            CommandKind::ValidationRun => {
                self.projection.task_state = ProductLifecycleState::Validating;
            }
            CommandKind::ArtifactBuild => {
                self.projection.task_state = ProductLifecycleState::Packaging;
            }
            CommandKind::Reconnect => {
                self.projection.continuity_state = BackgroundContinuityState::ActiveBackground;
            }
            CommandKind::PauseTask => {
                if matches!(
                    self.projection.task_state,
                    ProductLifecycleState::Created
                        | ProductLifecycleState::Completed
                        | ProductLifecycleState::Cancelled
                        | ProductLifecycleState::SafelyFailed
                ) {
                    return Err(DomainError::InvalidTransition);
                }
                self.projection.task_state = ProductLifecycleState::Paused;
            }
        }

        if let Some(key) = command.idempotency_key {
            self.accepted_idempotency_keys.push(key);
        }
        self.projection.projection_revision = next_revision(self.projection.projection_revision);
        let event = ControlEvent {
            event_id: command.command_id,
            sequence: self.next_sequence,
            project_id: self.project_id.clone(),
            task_id: command.task_id.or_else(|| Some(TaskId("task-0001".into()))),
            kind: format!("{:?}", command.kind),
            payload: command.payload,
            source_revision: self.projection.current_source_revision,
        };
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.projection.last_event_sequence = event.sequence;
        self.events.push(event);
        Ok(self.snapshot())
    }

    fn latest_event(&self) -> Option<ControlEvent> {
        self.events.last().cloned()
    }

    fn record_timeout(
        &mut self,
        command_id: &str,
        reason: &str,
    ) -> Result<ProjectionSnapshot, DomainError> {
        if matches!(
            self.projection.task_state,
            ProductLifecycleState::Completed | ProductLifecycleState::Cancelled
        ) {
            return Err(DomainError::InvalidTransition);
        }
        self.projection.task_state = ProductLifecycleState::SafelyFailed;
        self.projection.projection_revision = next_revision(self.projection.projection_revision);
        let event = ControlEvent {
            event_id: format!("timeout:{command_id}"),
            sequence: self.next_sequence,
            project_id: self.project_id.clone(),
            task_id: Some(TaskId("task-0001".into())),
            kind: "CommandTimedOut".into(),
            payload: serde_json::json!({"commandId": command_id, "reason": reason}).to_string(),
            source_revision: self.projection.current_source_revision,
        };
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.projection.last_event_sequence = event.sequence;
        self.events.push(event);
        Ok(self.snapshot())
    }
}

#[derive(Debug)]
pub enum DurableControlPlaneError {
    Domain(DomainError),
    Storage(rusqlite::Error),
    IdempotencyConflict,
    CorruptCommandResult(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableDispatchOutcome {
    Accepted {
        snapshot: ProjectionSnapshot,
        event: ControlEvent,
    },
    Duplicate {
        snapshot: ProjectionSnapshot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableTimeoutOutcome {
    Recorded {
        snapshot: ProjectionSnapshot,
        event: ControlEvent,
    },
    Duplicate {
        snapshot: ProjectionSnapshot,
    },
}

impl From<DomainError> for DurableControlPlaneError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<rusqlite::Error> for DurableControlPlaneError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error)
    }
}

#[derive(Debug)]
pub struct DurableControlPlane {
    plane: ControlPlane,
    ledger: Ledger,
    worker_cancel_requested: bool,
    checkpoint_id: Option<String>,
}

impl DurableControlPlane {
    pub fn open(path: impl AsRef<Path>, project_id: ProjectId) -> Result<Self, rusqlite::Error> {
        let ledger = Ledger::open(path)?;
        Self::restore(ledger, project_id)
    }

    pub fn in_memory(project_id: ProjectId) -> Result<Self, rusqlite::Error> {
        let ledger = Ledger::open_in_memory()?;
        Self::restore(ledger, project_id)
    }

    fn restore(ledger: Ledger, project_id: ProjectId) -> Result<Self, rusqlite::Error> {
        let snapshot = ledger
            .load_projection(&project_id)?
            .unwrap_or_else(|| ControlPlane::new(project_id.clone()).snapshot());
        let events = ledger.events_after(&project_id, 0)?;
        let checkpoint_id = ledger.latest_checkpoint_id(&project_id)?;
        Ok(Self {
            plane: ControlPlane::from_snapshot(snapshot, events),
            ledger,
            worker_cancel_requested: false,
            checkpoint_id,
        })
    }

    pub fn snapshot(&self) -> ProjectionSnapshot {
        self.plane.snapshot()
    }

    pub fn replay_after(&self, sequence: u64) -> Result<Vec<ControlEvent>, rusqlite::Error> {
        self.ledger
            .events_after(&self.snapshot().project_id, sequence)
    }

    pub fn load_provider_profile(
        &self,
        provider_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger
            .load_provider_profile(&self.snapshot().project_id, provider_id)
    }

    pub fn load_android_construction_contract(
        &self,
        task_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger
            .load_android_construction_contract(&self.snapshot().project_id, task_id)
    }

    pub fn load_android_toolchain_preflight(
        &self,
        task_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger
            .load_android_toolchain_preflight(&self.snapshot().project_id, task_id)
    }

    pub fn save_m108_sync_record(
        &self,
        task_id: &str,
        projection_json: &str,
        evidence_json: &str,
        last_event_sequence: u64,
    ) -> Result<(), rusqlite::Error> {
        self.ledger.save_m108_sync_record(
            &self.snapshot().project_id,
            task_id,
            projection_json,
            evidence_json,
            last_event_sequence,
        )
    }

    pub fn load_m108_sync_record(
        &self,
        task_id: &str,
    ) -> Result<Option<(String, String, u64)>, rusqlite::Error> {
        self.ledger
            .load_m108_sync_record(&self.snapshot().project_id, task_id)
    }

    pub fn load_android_device_observation_for_source(
        &self,
        task_id: &str,
        source_revision: u64,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger.load_android_device_observation_for_source(
            &self.snapshot().project_id,
            task_id,
            source_revision,
        )
    }

    pub fn load_android_synthesis_build(
        &self,
        task_id: &str,
        source_revision: u64,
    ) -> Result<Option<(String, String, String, String, String)>, rusqlite::Error> {
        self.ledger.load_android_synthesis_build(
            &self.snapshot().project_id,
            task_id,
            source_revision,
        )
    }

    pub fn save_android_build_observation(
        &self,
        execution_id: &str,
        task_id: &str,
        source_revision: u64,
        project_fingerprint: &str,
        record_json: &str,
    ) -> Result<(), rusqlite::Error> {
        self.ledger.save_android_build_observation(
            execution_id,
            &self.snapshot().project_id,
            task_id,
            source_revision,
            project_fingerprint,
            record_json,
        )
    }

    pub fn load_android_build_observation(
        &self,
        task_id: &str,
        source_revision: u64,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger.load_android_build_observation(
            &self.snapshot().project_id,
            task_id,
            source_revision,
        )
    }

    pub fn save_android_artifact_export(
        &self,
        export_id: &str,
        task_id: &str,
        source_revision: u64,
        destination_path: &str,
        record_json: &str,
    ) -> Result<(), rusqlite::Error> {
        self.ledger.save_android_artifact_export(
            export_id,
            &self.snapshot().project_id,
            task_id,
            source_revision,
            destination_path,
            record_json,
        )
    }

    pub fn load_android_artifact_export(
        &self,
        task_id: &str,
        source_revision: u64,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger.load_android_artifact_export(
            &self.snapshot().project_id,
            task_id,
            source_revision,
        )
    }

    pub fn save_android_device_observation(
        &self,
        observation_id: &str,
        task_id: &str,
        source_revision: u64,
        device_identity: &str,
        record_json: &str,
    ) -> Result<(), rusqlite::Error> {
        self.ledger.save_android_device_observation(
            observation_id,
            &self.snapshot().project_id,
            task_id,
            source_revision,
            device_identity,
            record_json,
        )
    }

    pub fn load_android_device_observation(
        &self,
        task_id: &str,
        source_revision: u64,
        device_identity: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger.load_android_device_observation(
            &self.snapshot().project_id,
            task_id,
            source_revision,
            device_identity,
        )
    }

    pub fn dispatch_with_result_and_m4(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        m4: (&str, u64, &str, &str, &str, &str, &str, &str),
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let fingerprint = serde_json::to_string(&command).expect("command serialization");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json)
                .map_err(|e| DurableControlPlaneError::CorruptCommandResult(e.to_string()))?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }
        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate.latest_event().expect("accepted command event");
        let snapshot_json = serde_json::to_string(&snapshot).expect("snapshot serialization");
        self.ledger.commit_event_projection_and_command_and_m4(
            &event,
            &snapshot,
            &command.command_id,
            command.idempotency_key.as_deref(),
            &fingerprint,
            correlation_id,
            &snapshot_json,
            m4,
        )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn load_android_requirement_manifest(
        &self,
        task_id: &str,
        source_revision: u64,
    ) -> Result<Option<(String, String, String)>, rusqlite::Error> {
        self.ledger.load_android_requirement_manifest(
            &self.snapshot().project_id,
            task_id,
            source_revision,
        )
    }

    pub fn load_preview_revision(
        &self,
        preview_revision_id: &str,
    ) -> Result<Option<(String, String)>, rusqlite::Error> {
        self.ledger
            .load_preview_revision(&self.snapshot().project_id, preview_revision_id)
    }

    pub fn load_preview_projection(
        &self,
        task_id: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.ledger
            .load_preview_projection(&self.snapshot().project_id, task_id)
    }

    pub fn record_mutation_transaction(
        &self,
        record: &MutationTransactionRecord,
    ) -> Result<(), rusqlite::Error> {
        self.ledger.record_mutation_transaction(record)
    }
    pub fn load_mutation_transaction(
        &self,
        transaction_id: &str,
    ) -> Result<Option<MutationTransactionRecord>, rusqlite::Error> {
        self.ledger.mutation_transaction(transaction_id)
    }
    pub fn record_provider_execution(
        &self,
        record: &nirman_domain::ProviderExecutionRecord,
    ) -> Result<(), rusqlite::Error> {
        self.ledger.record_provider_execution(record)
    }

    pub fn load_provider_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<nirman_domain::ProviderExecutionRecord>, rusqlite::Error> {
        self.ledger.provider_execution(execution_id)
    }

    pub fn retention_floor(&self) -> Result<Option<u64>, rusqlite::Error> {
        self.ledger.retention_floor(&self.snapshot().project_id)
    }

    pub fn replay_after_with_gap(
        &self,
        sequence: u64,
    ) -> Result<(Vec<ControlEvent>, bool), rusqlite::Error> {
        let floor = self.retention_floor()?;
        let events = self.replay_after(sequence)?;
        let retention_gap =
            floor.is_some_and(|first_available| sequence.saturating_add(1) < first_available);
        let sequence_gap = events
            .first()
            .is_some_and(|event| event.sequence > sequence.saturating_add(1));
        Ok((events, retention_gap || sequence_gap))
    }

    pub fn set_retention_floor(
        &self,
        first_available_sequence: u64,
    ) -> Result<(), rusqlite::Error> {
        self.ledger
            .set_retention_floor(&self.snapshot().project_id, first_available_sequence)
    }

    pub fn dispatch(
        &mut self,
        command: CommandEnvelope,
    ) -> Result<ProjectionSnapshot, DurableControlPlaneError> {
        match self.dispatch_with_result(command, "internal")? {
            DurableDispatchOutcome::Accepted { snapshot, .. }
            | DurableDispatchOutcome::Duplicate { snapshot } => Ok(snapshot),
        }
    }

    pub fn dispatch_with_result(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        self.dispatch_with_result_and_provider_profile(command, correlation_id, None)
    }

    pub fn dispatch_with_result_and_android_contract(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        contract: Option<(&str, &str, &str, u16)>,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::to_string(&command)
            .expect("CommandEnvelope serialization must remain infallible");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }

        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate
            .latest_event()
            .expect("accepted command always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger
            .commit_event_projection_and_command_and_android_contract(
                &event,
                &snapshot,
                &command.command_id,
                command.idempotency_key.as_deref(),
                &request_fingerprint,
                correlation_id,
                &snapshot_json,
                contract,
            )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn dispatch_with_result_and_android_toolchain_preflight(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        preflight: Option<(&str, &str, &str, &str, Option<&str>, &str)>,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::to_string(&command)
            .expect("CommandEnvelope serialization must remain infallible");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }

        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate
            .latest_event()
            .expect("accepted command always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger
            .commit_event_projection_and_command_and_android_toolchain_preflight(
                &event,
                &snapshot,
                &command.command_id,
                command.idempotency_key.as_deref(),
                &request_fingerprint,
                correlation_id,
                &snapshot_json,
                preflight,
            )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn dispatch_with_result_and_preview_revision(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        preview: Option<(&str, &str, &str, &str, &str, &str)>,
        projection: Option<(&str, &str, &str)>,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::to_string(&command)
            .expect("CommandEnvelope serialization must remain infallible");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }
        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate
            .latest_event()
            .expect("accepted command always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger
            .commit_event_projection_and_command_and_preview_revision(
                &event,
                &snapshot,
                &command.command_id,
                command.idempotency_key.as_deref(),
                &request_fingerprint,
                correlation_id,
                &snapshot_json,
                preview,
                projection,
            )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn dispatch_with_result_and_android_requirement_manifest(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        manifest: Option<(&str, &str, u64, &str, &str, Option<&str>)>,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::to_string(&command)
            .expect("CommandEnvelope serialization must remain infallible");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }
        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate
            .latest_event()
            .expect("accepted command always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger
            .commit_event_projection_and_command_and_android_requirement_manifest(
                &event,
                &snapshot,
                &command.command_id,
                command.idempotency_key.as_deref(),
                &request_fingerprint,
                correlation_id,
                &snapshot_json,
                manifest,
            )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn dispatch_with_result_and_mutation_transaction(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        mutation_transaction: &MutationTransactionRecord,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::to_string(&command)
            .expect("CommandEnvelope serialization must remain infallible");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }
        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate
            .latest_event()
            .expect("accepted command always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger
            .commit_event_projection_command_and_mutation_transaction(
                &event,
                &snapshot,
                &command.command_id,
                command.idempotency_key.as_deref(),
                &request_fingerprint,
                correlation_id,
                &snapshot_json,
                mutation_transaction,
            )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn dispatch_with_result_and_provider_profile(
        &mut self,
        command: CommandEnvelope,
        correlation_id: &str,
        provider_profile: Option<(&str, &str)>,
    ) -> Result<DurableDispatchOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::to_string(&command)
            .expect("CommandEnvelope serialization must remain infallible");
        if let Some(previous) = self.ledger.load_command_result(
            &project_id,
            &command.command_id,
            command.idempotency_key.as_deref(),
        )? {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableDispatchOutcome::Duplicate { snapshot });
        }

        let mut candidate = self.plane.clone();
        let snapshot = candidate.accept(command.clone())?;
        let event = candidate
            .latest_event()
            .expect("accepted command always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger
            .commit_event_projection_and_command_and_provider_profile(
                &event,
                &snapshot,
                &command.command_id,
                command.idempotency_key.as_deref(),
                &request_fingerprint,
                correlation_id,
                &snapshot_json,
                provider_profile,
            )?;
        self.plane = candidate;
        Ok(DurableDispatchOutcome::Accepted { snapshot, event })
    }

    pub fn record_timeout(
        &mut self,
        timeout_id: &str,
        command_id: &str,
        correlation_id: &str,
        reason: &str,
    ) -> Result<DurableTimeoutOutcome, DurableControlPlaneError> {
        let project_id = self.snapshot().project_id;
        let request_fingerprint = serde_json::json!({
            "timeoutId": timeout_id,
            "commandId": command_id,
            "reason": reason,
        })
        .to_string();
        if let Some(previous) = self
            .ledger
            .load_command_result(&project_id, timeout_id, None)?
        {
            if previous.request_fingerprint != request_fingerprint {
                return Err(DurableControlPlaneError::IdempotencyConflict);
            }
            let snapshot = serde_json::from_str(&previous.snapshot_json).map_err(|error| {
                DurableControlPlaneError::CorruptCommandResult(error.to_string())
            })?;
            return Ok(DurableTimeoutOutcome::Duplicate { snapshot });
        }

        let mut candidate = self.plane.clone();
        let snapshot = candidate.record_timeout(command_id, reason)?;
        let event = candidate
            .latest_event()
            .expect("recorded timeout always emits one event");
        let snapshot_json = serde_json::to_string(&snapshot)
            .expect("ProjectionSnapshot serialization must remain infallible");
        self.ledger.commit_event_projection_and_command(
            &event,
            &snapshot,
            timeout_id,
            None,
            &request_fingerprint,
            correlation_id,
            &snapshot_json,
        )?;
        self.plane = candidate;
        Ok(DurableTimeoutOutcome::Recorded { snapshot, event })
    }

    pub fn checkpoint(&mut self, checkpoint_id: impl Into<String>) -> Result<(), rusqlite::Error> {
        let checkpoint_id = checkpoint_id.into();
        self.ledger.save_checkpoint(
            &self.snapshot().project_id,
            &checkpoint_id,
            &self.snapshot(),
        )?;
        self.checkpoint_id = Some(checkpoint_id);
        Ok(())
    }

    pub fn request_worker_cancel(&mut self) {
        self.worker_cancel_requested = true;
    }

    pub fn worker_cancel_requested(&self) -> bool {
        self.worker_cancel_requested
    }

    pub fn checkpoint_id(&self) -> Option<&str> {
        self.checkpoint_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction(revision: u64, id: &str) -> CommandEnvelope {
        CommandEnvelope {
            command_id: id.into(),
            project_id: ProjectId("project-0001".into()),
            task_id: None,
            kind: CommandKind::SubmitInstruction,
            payload: "Build an Android notes app".into(),
            expected_projection_revision: Revision(revision),
            idempotency_key: Some(id.into()),
        }
    }

    fn command(revision: u64, id: &str, kind: CommandKind) -> CommandEnvelope {
        CommandEnvelope {
            command_id: id.into(),
            project_id: ProjectId("project-0001".into()),
            task_id: Some(TaskId("task-0001".into())),
            kind,
            payload: String::new(),
            expected_projection_revision: Revision(revision),
            idempotency_key: Some(id.into()),
        }
    }

    #[test]
    fn instruction_creates_revision_event_and_requested_preview() {
        let mut plane = ControlPlane::new(ProjectId("project-0001".into()));
        let snapshot = plane.accept(instruction(0, "cmd-1")).expect("accepted");
        assert_eq!(snapshot.current_source_revision, Revision(1));
        assert_eq!(snapshot.preview_truth, PreviewTruth::Requested);
        assert_eq!(plane.events_after(0).len(), 1);
    }

    #[test]
    fn pause_resume_and_cancel_are_authoritative_transitions() {
        let mut plane = ControlPlane::new(ProjectId("project-0001".into()));
        plane.accept(instruction(0, "cmd-1")).expect("accepted");
        let paused = plane
            .accept(command(1, "cmd-2", CommandKind::PauseTask))
            .expect("paused");
        assert_eq!(paused.task_state, ProductLifecycleState::Paused);
        let resumed = plane
            .accept(command(2, "cmd-3", CommandKind::ResumeTask))
            .expect("resumed");
        assert_eq!(resumed.task_state, ProductLifecycleState::Planning);
        let cancelled = plane
            .accept(command(3, "cmd-4", CommandKind::CancelTask))
            .expect("cancelled");
        assert_eq!(cancelled.task_state, ProductLifecycleState::Cancelled);
    }

    #[test]
    fn durable_dispatch_survives_restore() {
        let mut durable =
            DurableControlPlane::in_memory(ProjectId("project-0001".into())).expect("ledger");
        durable.dispatch(instruction(0, "cmd-1")).expect("dispatch");
        durable.checkpoint("checkpoint-1").expect("checkpoint");
        durable.request_worker_cancel();
        assert_eq!(durable.snapshot().last_event_sequence, 1);
        assert_eq!(durable.checkpoint_id(), Some("checkpoint-1"));
        assert!(durable.worker_cancel_requested());
    }

    #[test]
    fn stale_revision_is_rejected() {
        let mut plane = ControlPlane::new(ProjectId("project-0001".into()));
        plane.accept(instruction(0, "cmd-1")).expect("accepted");
        assert!(matches!(
            plane.accept(instruction(0, "cmd-2")),
            Err(DomainError::StaleProjection { .. })
        ));
    }
}

#[cfg(test)]
mod durable_m115_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nirman-{name}-{nonce}.sqlite3"))
    }

    fn instruction(revision: u64, id: &str, payload: &str) -> CommandEnvelope {
        CommandEnvelope {
            command_id: id.into(),
            project_id: ProjectId("project-0001".into()),
            task_id: None,
            kind: CommandKind::SubmitInstruction,
            payload: payload.into(),
            expected_projection_revision: Revision(revision),
            idempotency_key: Some(format!("idem-{id}")),
        }
    }

    #[test]
    fn durable_idempotency_returns_previous_result_after_restart() {
        let database = database_path("idempotency");
        let command = instruction(0, "cmd-1", "Build an Android notes app");
        {
            let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
                .expect("open ledger");
            let accepted = plane
                .dispatch_with_result(command.clone(), "corr-1")
                .expect("accepted");
            assert!(matches!(accepted, DurableDispatchOutcome::Accepted { .. }));
            let duplicate = plane
                .dispatch_with_result(command.clone(), "corr-2")
                .expect("duplicate result");
            assert!(matches!(
                duplicate,
                DurableDispatchOutcome::Duplicate { .. }
            ));
            assert_eq!(plane.replay_after(0).expect("replay").len(), 1);
        }
        {
            let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
                .expect("reload ledger");
            let duplicate = plane
                .dispatch_with_result(command, "corr-3")
                .expect("duplicate after restart");
            assert!(matches!(
                duplicate,
                DurableDispatchOutcome::Duplicate { .. }
            ));
            assert_eq!(plane.snapshot().projection_revision, Revision(1));
        }
        let conflicting = instruction(0, "cmd-1", "Different instruction");
        let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("reload for conflict");
        assert!(matches!(
            plane.dispatch_with_result(conflicting, "corr-4"),
            Err(DurableControlPlaneError::IdempotencyConflict)
        ));
        let _ = fs::remove_file(database);
    }

    #[cfg(unix)]
    #[test]
    fn failed_sqlite_commit_does_not_advance_live_or_reloaded_projection() {
        use std::os::unix::fs::PermissionsExt;

        let directory = database_path("rollback-dir");
        fs::create_dir(&directory).expect("create private temp directory");
        let database = directory.join("ledger.sqlite3");
        let parent = directory.as_path();
        let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("open ledger");
        let before = plane.snapshot();
        let original_permissions = fs::metadata(parent).expect("metadata").permissions();
        let mut read_only = original_permissions.clone();
        read_only.set_mode(0o500);
        fs::set_permissions(parent, read_only).expect("make directory read-only");
        let result = plane.dispatch_with_result(
            instruction(0, "cmd-rollback", "Build an Android notes app"),
            "corr-rollback",
        );
        fs::set_permissions(parent, original_permissions).expect("restore directory permissions");
        assert!(matches!(result, Err(DurableControlPlaneError::Storage(_))));
        assert_eq!(plane.snapshot(), before);
        let reloaded = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("reload ledger");
        assert_eq!(reloaded.snapshot(), before);
        let _ = fs::remove_file(&database);
        let _ = fs::remove_dir(&directory);
    }
}

#[cfg(test)]
mod m115_command_surface_tests {
    use super::*;

    fn command(revision: u64, id: &str, kind: CommandKind, payload: &str) -> CommandEnvelope {
        CommandEnvelope {
            command_id: id.into(),
            project_id: ProjectId("project-0001".into()),
            task_id: None,
            kind,
            payload: payload.into(),
            expected_projection_revision: Revision(revision),
            idempotency_key: Some(format!("idem-{id}")),
        }
    }

    #[test]
    fn every_m115_command_kind_has_an_executable_control_plane_path() {
        let mut plane = ControlPlane::new(ProjectId("project-0001".into()));
        let commands = [
            CommandKind::ProjectOpen,
            CommandKind::TaskStart,
            CommandKind::WorkspaceApplyPatch,
            CommandKind::PreviewStart,
            CommandKind::PreviewStop,
            CommandKind::ValidationRun,
            CommandKind::ArtifactBuild,
            CommandKind::ArtifactExport,
            CommandKind::ProviderTest,
            CommandKind::SettingsUpdateProvider,
            CommandKind::PauseTask,
            CommandKind::TaskResume,
            CommandKind::TaskCancel,
            CommandKind::Reconnect,
            CommandKind::SubmitInstruction,
            CommandKind::ResumeTask,
            CommandKind::CancelTask,
        ];
        let command_count = commands.len();
        for (revision, kind) in commands.into_iter().enumerate() {
            if matches!(
                kind,
                CommandKind::TaskStart | CommandKind::SubmitInstruction
            ) {
                plane.projection.task_state = ProductLifecycleState::Planning;
            }
            if matches!(kind, CommandKind::TaskResume | CommandKind::ResumeTask) {
                plane.projection.task_state = ProductLifecycleState::Paused;
            }
            let accepted = plane.accept(command(
                revision as u64,
                &format!("m115-{revision}"),
                kind,
                if matches!(
                    kind,
                    CommandKind::TaskStart | CommandKind::SubmitInstruction
                ) {
                    "Build an Android app"
                } else {
                    "m115"
                },
            ));
            assert!(
                accepted.is_ok(),
                "M115 command {kind:?} was not executable: {accepted:?}"
            );
        }
        assert_eq!(
            plane.snapshot().projection_revision,
            Revision(command_count as u64)
        );
    }
}

#[cfg(test)]
mod m115_timeout_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nirman-timeout-{nonce}.sqlite3"))
    }

    #[test]
    fn timeout_before_admission_has_no_durable_mutation() {
        let database = database_path();
        let plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("open ledger");
        let before = plane.snapshot();
        assert!(deadline_elapsed(Some(100), 100));
        assert!(!deadline_elapsed(Some(101), 100));
        assert_eq!(plane.snapshot(), before);
        assert!(plane.replay_after(0).expect("replay").is_empty());
        let _ = fs::remove_file(database);
    }

    #[test]
    fn timeout_after_acceptance_cannot_overwrite_the_accepted_result() {
        let database = database_path();
        let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("open ledger");
        let command = CommandEnvelope {
            command_id: "command-after-acceptance".into(),
            project_id: ProjectId("project-0001".into()),
            task_id: None,
            kind: CommandKind::SubmitInstruction,
            payload: "Build an Android app".into(),
            expected_projection_revision: Revision(0),
            idempotency_key: Some("idem-after-acceptance".into()),
        };
        let accepted = plane
            .dispatch_with_result(command.clone(), "corr")
            .expect("accepted");
        let accepted_snapshot = match accepted {
            DurableDispatchOutcome::Accepted { snapshot, .. } => snapshot,
            DurableDispatchOutcome::Duplicate { .. } => panic!("first dispatch cannot duplicate"),
        };
        let timed_out = plane
            .record_timeout(
                "timeout-command-after-acceptance",
                &command.command_id,
                "corr",
                "late watchdog",
            )
            .expect("timeout record");
        assert!(matches!(timed_out, DurableTimeoutOutcome::Recorded { .. }));
        let reloaded = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("reload ledger");
        assert_eq!(reloaded.replay_after(0).expect("replay").len(), 2);
        assert_eq!(accepted_snapshot.projection_revision, Revision(1));
        assert_eq!(reloaded.snapshot().projection_revision, Revision(2));
        assert!(matches!(
            reloaded.snapshot().task_state,
            ProductLifecycleState::SafelyFailed
        ));
        let mut recovered = reloaded;
        let recovery = recovered
            .dispatch(CommandEnvelope {
                command_id: "command-recover".into(),
                project_id: ProjectId("project-0001".into()),
                task_id: None,
                kind: CommandKind::TaskResume,
                payload: "recover after timeout".into(),
                expected_projection_revision: Revision(2),
                idempotency_key: Some("idem-recover".into()),
            })
            .expect("timeout recovery");
        assert_eq!(recovery.task_state, ProductLifecycleState::Planning);
        let _ = fs::remove_file(database);
    }

    #[test]
    fn timeout_record_is_idempotent_after_restart() {
        let database = database_path();
        let first = {
            let mut plane = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
                .expect("open ledger");
            plane
                .record_timeout(
                    "timeout-replay",
                    "command-replay",
                    "corr",
                    "deadline elapsed",
                )
                .expect("record timeout")
        };
        assert!(matches!(first, DurableTimeoutOutcome::Recorded { .. }));
        let mut reopened = DurableControlPlane::open(&database, ProjectId("project-0001".into()))
            .expect("reopen ledger");
        let duplicate = reopened
            .record_timeout(
                "timeout-replay",
                "command-replay",
                "corr",
                "deadline elapsed",
            )
            .expect("idempotent timeout");
        assert!(matches!(duplicate, DurableTimeoutOutcome::Duplicate { .. }));
        assert_eq!(reopened.replay_after(0).expect("replay").len(), 1);
        let _ = fs::remove_file(database);
    }
}
