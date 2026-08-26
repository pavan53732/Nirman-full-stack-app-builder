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

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_domain::{CommandKind, ProjectId, Revision};

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
