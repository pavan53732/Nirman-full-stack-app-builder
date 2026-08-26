//! Typed local IPC envelopes shared by the desktop UI and control plane.

#![forbid(unsafe_code)]

use nirman_domain::{CommandEnvelope, ControlEvent, ProjectionSnapshot, Revision};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthContext {
    pub installation_id: String,
    pub user_scope: String,
    pub project_scope: String,
    pub schema_version: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRequest {
    pub auth: AuthContext,
    pub command: CommandEnvelope,
    pub correlation_id: String,
    pub causation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResponse {
    pub correlation_id: String,
    pub projection_revision: Revision,
    pub snapshot: ProjectionSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlPlaneErrorCode {
    AuthenticationFailed,
    SchemaMismatch,
    PermissionDenied,
    StaleProjection,
    DuplicateCommand,
    DependencyUnavailable,
    InvalidCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub correlation_id: String,
    pub code: ControlPlaneErrorCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSubscription {
    pub auth: AuthContext,
    pub project_id: String,
    pub after_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventBatch {
    pub projection_revision: Revision,
    pub events: Vec<ControlEvent>,
    pub has_gap: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectionReceiver {
    snapshot: Option<ProjectionSnapshot>,
    last_event_sequence: u64,
    rejected_events: u64,
}

impl ProjectionReceiver {
    pub fn observe_snapshot(&mut self, snapshot: ProjectionSnapshot) -> bool {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|current| current.projection_revision >= snapshot.projection_revision)
        {
            return false;
        }
        self.last_event_sequence = snapshot.last_event_sequence;
        self.snapshot = Some(snapshot);
        true
    }

    pub fn observe_event(&mut self, event: &ControlEvent) -> bool {
        if event.sequence != self.last_event_sequence.saturating_add(1) {
            self.rejected_events = self.rejected_events.saturating_add(1);
            return false;
        }
        self.last_event_sequence = event.sequence;
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
    use nirman_domain::{CommandKind, ProjectId, Revision};

    #[test]
    fn projection_receiver_rejects_duplicate_and_out_of_order_events() {
        let project = ProjectId("project-1".into());
        let snapshot = ProjectionSnapshot {
            project_id: project.clone(),
            projection_revision: Revision(1),
            task_state: nirman_domain::ProductLifecycleState::Planning,
            continuity_state: nirman_domain::BackgroundContinuityState::ActiveBackground,
            preview_truth: nirman_domain::PreviewTruth::Requested,
            current_source_revision: Revision(1),
            last_event_sequence: 1,
            last_known_good_ref: None,
        };
        let mut receiver = ProjectionReceiver::default();
        assert!(receiver.observe_snapshot(snapshot));
        let event = ControlEvent {
            event_id: "event-2".into(),
            sequence: 2,
            project_id: project.clone(),
            task_id: None,
            kind: "PauseTask".into(),
            payload: String::new(),
            source_revision: Revision(1),
        };
        assert!(receiver.observe_event(&event));
        assert!(!receiver.observe_event(&event));
        assert_eq!(receiver.rejected_events(), 1);
    }

    #[test]
    fn request_and_subscription_share_project_scoped_auth_context() {
        let auth = AuthContext {
            installation_id: "install-1".into(),
            user_scope: "local-user".into(),
            project_scope: "project-1".into(),
            schema_version: 1,
        };
        let request = CommandRequest {
            auth: auth.clone(),
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
        };
        let subscription = EventSubscription {
            auth,
            project_id: "project-1".into(),
            after_sequence: 0,
        };
        assert_eq!(request.auth.project_scope, subscription.project_id);
    }
}
