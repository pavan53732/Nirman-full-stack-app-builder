//! Authoritative local control-plane primitives for M2.
//!
//! The in-memory `ControlPlane` owns command acceptance and projection rules.
//! `DurableControlPlane` wires that path to the SQLite ledger used for restart
//! recovery. Frontend code receives snapshots and events; it never owns truth.

#![forbid(unsafe_code)]

use nirman_domain::{
    next_revision, BackgroundContinuityState, CommandEnvelope, CommandKind, ControlEvent,
    DomainError, PreviewTruth, ProductLifecycleState, ProjectId, ProjectionSnapshot, Revision,
    TaskId,
};
use nirman_storage::Ledger;
use std::path::Path;

#[derive(Debug)]
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
        if matches!(command.kind, CommandKind::SubmitInstruction)
            && command.payload.trim().is_empty()
        {
            return Err(DomainError::EmptyInstruction);
        }

        match command.kind {
            CommandKind::SubmitInstruction => {
                self.projection.task_state = ProductLifecycleState::Planning;
                self.projection.preview_truth = PreviewTruth::Requested;
                self.projection.current_source_revision =
                    next_revision(self.projection.current_source_revision);
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
            CommandKind::ResumeTask => {
                if self.projection.task_state != ProductLifecycleState::Paused {
                    return Err(DomainError::InvalidTransition);
                }
                self.projection.task_state = ProductLifecycleState::Planning;
            }
            CommandKind::CancelTask => {
                if self.projection.task_state == ProductLifecycleState::Completed {
                    return Err(DomainError::InvalidTransition);
                }
                self.projection.task_state = ProductLifecycleState::Cancelled;
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
}

#[derive(Debug)]
pub enum DurableControlPlaneError {
    Domain(DomainError),
    Storage(rusqlite::Error),
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

    pub fn dispatch(
        &mut self,
        command: CommandEnvelope,
    ) -> Result<ProjectionSnapshot, DurableControlPlaneError> {
        let snapshot = self.plane.accept(command)?;
        let event = self
            .plane
            .latest_event()
            .expect("accepted command always emits one event");
        self.ledger.commit_event_and_projection(&event, &snapshot)?;
        Ok(snapshot)
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
