//! First vertical-slice control-plane core.
//!
//! This crate intentionally keeps the M0/M2 domain path dependency-light. SQLite,
//! authenticated IPC, process supervision, and Android adapters are layered on
//! later without moving authority into the frontend or model layer.

#![forbid(unsafe_code)]

use nirman_domain::{
    next_revision, BackgroundContinuityState, CommandEnvelope, CommandKind, ControlEvent,
    DomainError, PreviewTruth, ProductLifecycleState, ProjectId, ProjectionSnapshot, Revision,
    TaskId,
};

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
        Self {
            projection: ProjectionSnapshot {
                project_id: project_id.clone(),
                projection_revision: Revision(0),
                task_state: ProductLifecycleState::Created,
                continuity_state: BackgroundContinuityState::ActiveBackground,
                preview_truth: PreviewTruth::Predicted,
                current_source_revision: Revision(0),
                last_event_sequence: 0,
                last_known_good_ref: None,
            },
            project_id,
            events: Vec::new(),
            accepted_idempotency_keys: Vec::new(),
            next_sequence: 1,
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
            CommandKind::CancelTask => {
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
        self.next_sequence += 1;
        self.projection.last_event_sequence = event.sequence;
        self.events.push(event);
        Ok(self.snapshot())
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

    #[test]
    fn instruction_creates_revision_event_and_requested_preview() {
        let mut plane = ControlPlane::new(ProjectId("project-0001".into()));
        let snapshot = plane.accept(instruction(0, "cmd-1")).expect("accepted");
        assert_eq!(snapshot.current_source_revision, Revision(1));
        assert_eq!(snapshot.preview_truth, PreviewTruth::Requested);
        assert_eq!(plane.events_after(0).len(), 1);
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
