//! Canonical, dependency-light domain values shared by Nirman runtime crates.

#![forbid(unsafe_code)]

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductLifecycleState {
    Created,
    Planning,
    Implementing,
    Previewing,
    Validating,
    Recovering,
    Packaging,
    Completed,
    UserRequired,
    SafelyFailed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewTruth {
    Predicted,
    Requested,
    Observed,
    Verified,
    Stale,
    Invalidated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    SubmitInstruction,
    Reconnect,
    CancelTask,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub project_id: ProjectId,
    pub task_id: Option<TaskId>,
    pub kind: CommandKind,
    pub payload: String,
    pub expected_projection_revision: Revision,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlEvent {
    pub event_id: String,
    pub sequence: u64,
    pub project_id: ProjectId,
    pub task_id: Option<TaskId>,
    pub kind: String,
    pub payload: String,
    pub source_revision: Revision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
