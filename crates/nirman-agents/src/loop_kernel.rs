//! Agent execution kernel (M58): the durable agent-loop state machine.
//!
//! Implements the OBSERVE → UNDERSTAND → PLAN → SELECT_ACTION → AUTHORIZE →
//! EXECUTE → OBSERVE_RESULT → UPDATE_STATE → EVALUATE_PROGRESS cycle from the
//! technical architecture §58.2. The kernel is a pure reducer pair: it never
//! executes actions itself and never touches the filesystem. The embedder
//! executes [`AgentAction`]s through the real authorities (policy gate,
//! workspace mutation, gradle execution) and feeds [`LoopObservation`]s back,
//! which is what keeps LifecycleAuthority, PolicyAuthority, and ToolBroker
//! non-delegable.
//!
//! Safety properties enforced here:
//! - only [`AgentLoopReducer::reduce`] commits lifecycle transitions
//! - a bounded iteration budget terminates any loop (no infinite cycling)
//! - a substantially identical failed attempt is never retried as-is: the
//!   kernel demands a strategy variation, and rejects the replan too when the
//!   variation budget is exhausted
//! - every record is JSON-serializable and replayable

use serde::{Deserialize, Serialize};
use std::fmt;

pub const AGENT_LOOP_SCHEMA_VERSION: u16 = 1;
pub const AGENT_LOOP_SCHEMA_REF: &str = "nirman.agent_loop_record.v1";
pub const DEFAULT_ITERATION_BUDGET: u32 = 8;
pub const DEFAULT_MAX_CONSECUTIVE_FAILURES: u32 = 3;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentLoopState {
    Running,
    Suspended,
    Complete,
    Failed,
    Exhausted,
    Cancelled,
}

impl AgentLoopState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Complete | Self::Failed | Self::Exhausted | Self::Cancelled
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentLoopPhase {
    Observe,
    Understand,
    Plan,
    SelectAction,
    Authorize,
    Execute,
    ObserveResult,
    UpdateState,
    EvaluateProgress,
}

impl AgentLoopPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "OBSERVE",
            Self::Understand => "UNDERSTAND",
            Self::Plan => "PLAN",
            Self::SelectAction => "SELECT_ACTION",
            Self::Authorize => "AUTHORIZE",
            Self::Execute => "EXECUTE",
            Self::ObserveResult => "OBSERVE_RESULT",
            Self::UpdateState => "UPDATE_STATE",
            Self::EvaluateProgress => "EVALUATE_PROGRESS",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopContinuation {
    Continue,
    Validate,
    Recover,
    Delegate,
    Replan,
    Complete,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressStatus {
    NotStarted,
    OnTrack,
    Recovering,
    Replanning,
    Complete,
    Failed,
    Exhausted,
    Cancelled,
}

/// Durable agent loop record — spec §58.3 `AgentLoopRecord` plus the
/// iteration bookkeeping the reducer needs to enforce budgets.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AgentLoopRecord {
    pub schema_version: u16,
    pub loop_id: String,
    pub session_id: String,
    pub task_id: String,
    pub agent_instance_id: String,
    pub state: AgentLoopState,
    pub state_version: u64,
    pub goal_revision: u64,
    pub plan_revision: u64,
    pub project_revision: u64,
    pub last_observation_id: Option<String>,
    pub last_proposal_id: Option<String>,
    pub progress_status: ProgressStatus,
    pub retry_strategy: String,
    pub cancellation_scope: String,
    pub created_at_epoch_seconds: u64,
    pub updated_at_epoch_seconds: u64,
    pub phase: AgentLoopPhase,
    pub iteration: u32,
    pub iteration_budget: u32,
    pub consecutive_failures: u32,
    pub max_consecutive_failures: u32,
    pub last_failed_action: Option<String>,
    pub last_failed_action_fingerprint: Option<String>,
    pub variation_attempts: u32,
    /// Variation selected by the latest diagnosis; consumed by the retry
    /// action so a retry is never substantially identical to the failure.
    pub pending_variation: Option<String>,
    pub completed_action_count: u32,
}

impl AgentLoopRecord {
    pub fn validate(&self) -> Result<(), AgentLoopError> {
        if self.schema_version != AGENT_LOOP_SCHEMA_VERSION {
            return Err(AgentLoopError::UnsupportedSchemaVersion);
        }
        for (field, value) in [
            ("loopId", self.loop_id.as_str()),
            ("sessionId", self.session_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("agentInstanceId", self.agent_instance_id.as_str()),
            ("cancellationScope", self.cancellation_scope.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(AgentLoopError::EmptyField(field));
            }
        }
        if self.iteration_budget == 0 || self.max_consecutive_failures == 0 {
            return Err(AgentLoopError::InvalidBudget);
        }
        if self.iteration > self.iteration_budget {
            return Err(AgentLoopError::InvalidBudget);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentLoopError {
    UnsupportedSchemaVersion,
    EmptyField(&'static str),
    InvalidBudget,
    TerminalState,
    UnexpectedObservation {
        expected_action: String,
        observed_action: String,
    },
    UnknownAction,
}

impl fmt::Display for AgentLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion => {
                f.write_str("M58 agent loop schema version is unsupported")
            }
            Self::EmptyField(field) => write!(f, "M58 agent loop field is empty: {field}"),
            Self::InvalidBudget => f.write_str("M58 agent loop budget is invalid"),
            Self::TerminalState => f.write_str("M58 agent loop is already terminal"),
            Self::UnexpectedObservation {
                expected_action,
                observed_action,
            } => write!(
                f,
                "M58 observation belongs to action {observed_action} while the loop awaits {expected_action}"
            ),
            Self::UnknownAction => f.write_str("M58 agent action is unknown"),
        }
    }
}
impl std::error::Error for AgentLoopError {}

/// An action the embedder must execute through the real authorities.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AgentAction {
    pub action_id: String,
    pub action_type: AgentActionType,
    pub attempt: u32,
    pub variation: String,
    pub action_fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentActionType {
    /// Resolve the construction contract into a synthesis plan (M4).
    SynthesizeProject,
    /// Generate and apply the Android project scaffold (M4b).
    ScaffoldProject,
    /// Run the locked Gradle build (M5/M108).
    BuildArtifact,
    /// Classify a build failure and select a repair strategy.
    DiagnoseFailure,
    /// Verify the produced APK (hash + inspection when aapt is available).
    ValidateArtifact,
}

impl AgentActionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SynthesizeProject => "SYNTHESIZE_PROJECT",
            Self::ScaffoldProject => "SCAFFOLD_PROJECT",
            Self::BuildArtifact => "BUILD_ARTIFACT",
            Self::DiagnoseFailure => "DIAGNOSE_FAILURE",
            Self::ValidateArtifact => "VALIDATE_ARTIFACT",
        }
    }
}

/// What the embedder reports back after executing an [`AgentAction`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LoopObservation {
    pub observation_id: String,
    pub action_id: String,
    pub success: bool,
    pub summary: String,
    pub evidence_id: Option<String>,
    /// Failure classification for diagnosis; `None` on success.
    pub failure_class: Option<FailureClass>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureClass {
    /// Toolchain/gradle could not be located: environment problem.
    EnvironmentUnavailable,
    /// Build exceeded its time budget.
    Timeout,
    /// Gradle exited non-zero (compile/tooling error).
    BuildFailed,
    /// Gradle succeeded but no APK was produced.
    MissingArtifact,
    /// The attempt was cancelled by the user/supervisor.
    Cancelled,
}

impl FailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnvironmentUnavailable => "ENVIRONMENT_UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::BuildFailed => "BUILD_FAILED",
            Self::MissingArtifact => "MISSING_ARTIFACT",
            Self::Cancelled => "CANCELLED",
        }
    }
}

/// Deterministic diagnosis: maps a failure class to the next action and the
/// strategy variation that makes the retry substantively different.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticReport {
    pub diagnosis_id: String,
    pub failure_class: FailureClass,
    pub strategy: String,
    pub variation: String,
    pub retry_recommended: bool,
}

pub fn diagnose_failure(failure_class: FailureClass, variation_attempts: u32) -> DiagnosticReport {
    let (strategy, variation, retry) = match failure_class {
        FailureClass::EnvironmentUnavailable => (
            "environment-repair-required".to_owned(),
            "none".to_owned(),
            false,
        ),
        FailureClass::Timeout => (
            "extend-build-budget".to_owned(),
            format!("timeout-x{}", variation_attempts + 2),
            true,
        ),
        FailureClass::BuildFailed => (
            "clean-rebuild".to_owned(),
            format!("clean+{}", variation_attempts + 1),
            true,
        ),
        FailureClass::MissingArtifact => (
            "task-and-output-reconciliation".to_owned(),
            format!("task-recheck-{}", variation_attempts + 1),
            true,
        ),
        FailureClass::Cancelled => ("none".to_owned(), "none".to_owned(), false),
    };
    DiagnosticReport {
        diagnosis_id: format!("diagnosis-{}", failure_class.as_str()),
        failure_class,
        strategy,
        variation,
        retry_recommended: retry,
    }
}

/// The decision the kernel hands to the embedder after each observation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum KernelDecision {
    /// Execute this action next (through policy + real executors).
    Execute(AgentAction),
    /// Loop finished; carry the terminal state in the record.
    Finished,
}

/// Only the reducer commits lifecycle transitions (spec §58.2).
pub struct AgentLoopReducer;

impl AgentLoopReducer {
    /// Starts a loop: returns the initial record and the first action.
    pub fn start(
        session_id: &str,
        task_id: &str,
        goal_revision: u64,
        iteration_budget: u32,
        now_epoch_seconds: u64,
    ) -> Result<(AgentLoopRecord, AgentAction), AgentLoopError> {
        if session_id.trim().is_empty() || task_id.trim().is_empty() {
            return Err(AgentLoopError::EmptyField("sessionId/taskId"));
        }
        if goal_revision == 0 {
            return Err(AgentLoopError::EmptyField("goalRevision"));
        }
        let budget = if iteration_budget == 0 {
            DEFAULT_ITERATION_BUDGET
        } else {
            iteration_budget
        };
        let loop_id = format!("loop-{task_id}-{goal_revision}");
        let record = AgentLoopRecord {
            schema_version: AGENT_LOOP_SCHEMA_VERSION,
            loop_id: loop_id.clone(),
            session_id: session_id.into(),
            task_id: task_id.into(),
            agent_instance_id: format!("agent-{loop_id}"),
            state: AgentLoopState::Running,
            state_version: 1,
            goal_revision,
            plan_revision: goal_revision,
            project_revision: 0,
            last_observation_id: None,
            last_proposal_id: None,
            progress_status: ProgressStatus::NotStarted,
            retry_strategy: "bounded-retry-with-variation".into(),
            cancellation_scope: format!("task:{task_id}"),
            created_at_epoch_seconds: now_epoch_seconds,
            updated_at_epoch_seconds: now_epoch_seconds,
            phase: AgentLoopPhase::Observe,
            iteration: 0,
            iteration_budget: budget,
            consecutive_failures: 0,
            max_consecutive_failures: DEFAULT_MAX_CONSECUTIVE_FAILURES,
            last_failed_action: None,
            last_failed_action_fingerprint: None,
            variation_attempts: 0,
            pending_variation: None,
            completed_action_count: 0,
        };
        let action = Self::action(&record, AgentActionType::SynthesizeProject, 1, "initial");
        Ok((record, action))
    }

    /// Feeds an observation back and returns the next decision. This is the
    /// whole loop: evaluate progress, pick the continuation, commit the
    /// transition, and propose the next action (or a terminal state).
    pub fn advance(
        record: &AgentLoopRecord,
        observation: &LoopObservation,
        now_epoch_seconds: u64,
    ) -> Result<(AgentLoopRecord, KernelDecision), AgentLoopError> {
        record.validate()?;
        if record.state.is_terminal() {
            return Err(AgentLoopError::TerminalState);
        }
        if record.awaited_action_type().is_none() {
            return Err(AgentLoopError::UnknownAction);
        }
        let expected = record.awaited_action_type().expect("checked above");
        let observed_matches = observation.summary.contains(expected.as_str())
            || observation.action_id.contains(expected.as_str());
        if !observed_matches {
            return Err(AgentLoopError::UnexpectedObservation {
                expected_action: expected.as_str().into(),
                observed_action: observation.action_id.clone(),
            });
        }
        let mut next = record.clone();
        next.state_version = next.state_version.saturating_add(1);
        next.updated_at_epoch_seconds = now_epoch_seconds;
        next.last_observation_id = Some(observation.observation_id.clone());
        next.iteration = next.iteration.saturating_add(1);

        // Cancellation is observed, never overridden.
        if let Some(FailureClass::Cancelled) = observation.failure_class {
            next.state = AgentLoopState::Cancelled;
            next.progress_status = ProgressStatus::Cancelled;
            next.phase = AgentLoopPhase::EvaluateProgress;
            return Ok((next, KernelDecision::Finished));
        }

        let continuation = if observation.success {
            // Only real work resets the failure streak: a successful
            // diagnosis is internal recovery, not progress.
            if expected != AgentActionType::DiagnoseFailure {
                next.consecutive_failures = 0;
            }
            next.completed_action_count = next.completed_action_count.saturating_add(1);
            Self::continuation_after_success(expected)
        } else {
            next.consecutive_failures = next.consecutive_failures.saturating_add(1);
            Self::continuation_after_failure(&mut next, observation)
        };

        // Budget enforcement: the iteration budget is a hard stop.
        if next.iteration >= next.iteration_budget
            && !matches!(
                (continuation, next.state),
                (LoopContinuation::Complete, _) | (_, AgentLoopState::Complete)
            )
        {
            next.state = AgentLoopState::Exhausted;
            next.progress_status = ProgressStatus::Exhausted;
            next.phase = AgentLoopPhase::EvaluateProgress;
            next.retry_strategy = format!("budget-exhausted-after-{}-iterations", next.iteration);
            return Ok((next, KernelDecision::Finished));
        }

        match continuation {
            LoopContinuation::Complete => {
                next.state = AgentLoopState::Complete;
                next.progress_status = ProgressStatus::Complete;
                next.phase = AgentLoopPhase::EvaluateProgress;
                Ok((next, KernelDecision::Finished))
            }
            LoopContinuation::Continue | LoopContinuation::Validate => {
                next.state = AgentLoopState::Running;
                next.progress_status = if next.consecutive_failures > 0 {
                    ProgressStatus::Recovering
                } else {
                    ProgressStatus::OnTrack
                };
                next.phase = AgentLoopPhase::SelectAction;
                let action_type = if continuation == LoopContinuation::Validate {
                    AgentActionType::ValidateArtifact
                } else {
                    Self::next_success_action(expected)
                };
                // A retry that follows a successful diagnosis must carry
                // the diagnosis variation; every other transition starts
                // fresh, which is what makes retries substantively
                // different from the failed attempt.
                let (attempt, variation) =
                    if expected == AgentActionType::DiagnoseFailure && observation.success {
                        let variation = next
                            .pending_variation
                            .clone()
                            .unwrap_or_else(|| "none".to_owned());
                        (next.variation_attempts.max(1), variation)
                    } else if observation.success {
                        (1, "none".to_owned())
                    } else {
                        (
                            next.variation_attempts.saturating_add(1),
                            next.variation_attempts.to_string(),
                        )
                    };
                let action = Self::action(&next, action_type, attempt, &variation);
                next.pending_variation = None;
                next.last_proposal_id = Some(action.action_id.clone());
                Ok((next, KernelDecision::Execute(action)))
            }
            LoopContinuation::Recover => {
                next.state = AgentLoopState::Running;
                next.progress_status = ProgressStatus::Recovering;
                next.phase = AgentLoopPhase::SelectAction;
                let action = Self::action(
                    &next,
                    AgentActionType::DiagnoseFailure,
                    next.variation_attempts + 1,
                    &format!("diagnose-{}", observation.observation_id),
                );
                next.last_proposal_id = Some(action.action_id.clone());
                Ok((next, KernelDecision::Execute(action)))
            }
            LoopContinuation::Replan => {
                // The current plan cannot proceed (identical failed attempt
                // or unrecoverable failure): fail safely.
                next.state = AgentLoopState::Failed;
                next.progress_status = ProgressStatus::Failed;
                next.phase = AgentLoopPhase::EvaluateProgress;
                next.retry_strategy = format!(
                    "replan-required-after-{}-failures",
                    next.consecutive_failures
                );
                Ok((next, KernelDecision::Finished))
            }
            LoopContinuation::Delegate => {
                // Delegation is out of scope for the local single-agent loop;
                // record it as a safe failure with a clear reason.
                next.state = AgentLoopState::Failed;
                next.progress_status = ProgressStatus::Failed;
                next.phase = AgentLoopPhase::EvaluateProgress;
                next.retry_strategy = "delegation-required".into();
                Ok((next, KernelDecision::Finished))
            }
        }
    }

    fn continuation_after_success(expected: AgentActionType) -> LoopContinuation {
        match expected {
            AgentActionType::SynthesizeProject
            | AgentActionType::ScaffoldProject
            | AgentActionType::DiagnoseFailure => LoopContinuation::Continue,
            AgentActionType::BuildArtifact => LoopContinuation::Validate,
            AgentActionType::ValidateArtifact => LoopContinuation::Complete,
        }
    }

    fn next_success_action(expected: AgentActionType) -> AgentActionType {
        match expected {
            AgentActionType::SynthesizeProject => AgentActionType::ScaffoldProject,
            AgentActionType::ScaffoldProject => AgentActionType::BuildArtifact,
            AgentActionType::DiagnoseFailure => AgentActionType::BuildArtifact,
            AgentActionType::BuildArtifact | AgentActionType::ValidateArtifact => {
                AgentActionType::ValidateArtifact
            }
        }
    }

    fn continuation_after_failure(
        next: &mut AgentLoopRecord,
        observation: &LoopObservation,
    ) -> LoopContinuation {
        if next.consecutive_failures >= next.max_consecutive_failures {
            return LoopContinuation::Replan;
        }
        let Some(failure_class) = observation.failure_class else {
            return LoopContinuation::Replan;
        };
        // Environment failures cannot be repaired by retrying the same action.
        if matches!(failure_class, FailureClass::EnvironmentUnavailable) {
            return LoopContinuation::Replan;
        }
        // Identical-attempt rejection (spec: recovery planner must reject a
        // substantially identical retry): the next build retry must carry a
        // new variation; when variations run out, replan.
        let diagnosis = diagnose_failure(failure_class, next.variation_attempts);
        if !diagnosis.retry_recommended {
            return LoopContinuation::Replan;
        }
        next.variation_attempts = next.variation_attempts.saturating_add(1);
        next.last_failed_action = Some(observation.action_id.clone());
        next.last_failed_action_fingerprint = Some(diagnosis.variation.clone());
        next.pending_variation = Some(diagnosis.variation.clone());
        LoopContinuation::Recover
    }

    fn action(
        record: &AgentLoopRecord,
        action_type: AgentActionType,
        attempt: u32,
        variation: &str,
    ) -> AgentAction {
        // The id embeds the monotonically increasing iteration so every
        // transition produces a distinct, replayable identity.
        let action_id = format!(
            "action-{}-{}-{}",
            record.loop_id,
            action_type.as_str(),
            record.iteration.saturating_add(1)
        );
        let fingerprint = format!(
            "{action_type:?}:{variation}:{attempt}:{}",
            record.goal_revision
        );
        AgentAction {
            action_id,
            action_type,
            attempt,
            variation: variation.into(),
            action_fingerprint: fingerprint,
        }
    }
}

impl AgentLoopRecord {
    /// The action type the loop currently awaits, derived from the phase
    /// machine: after `start` it awaits synthesis; observations for terminal
    /// loops are rejected earlier.
    fn awaited_action_type(&self) -> Option<AgentActionType> {
        match self.progress_status {
            ProgressStatus::NotStarted => Some(AgentActionType::SynthesizeProject),
            ProgressStatus::OnTrack => match self.last_proposal_id.as_deref() {
                Some(proposal) => {
                    if proposal.contains(AgentActionType::ScaffoldProject.as_str()) {
                        Some(AgentActionType::ScaffoldProject)
                    } else if proposal.contains(AgentActionType::BuildArtifact.as_str()) {
                        Some(AgentActionType::BuildArtifact)
                    } else if proposal.contains(AgentActionType::ValidateArtifact.as_str()) {
                        Some(AgentActionType::ValidateArtifact)
                    } else {
                        Some(AgentActionType::SynthesizeProject)
                    }
                }
                None => Some(AgentActionType::SynthesizeProject),
            },
            ProgressStatus::Recovering => {
                if self.last_proposal_id.as_deref().is_some_and(|proposal| {
                    proposal.contains(AgentActionType::DiagnoseFailure.as_str())
                }) {
                    Some(AgentActionType::DiagnoseFailure)
                } else {
                    Some(AgentActionType::BuildArtifact)
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(action: &AgentAction, success: bool) -> LoopObservation {
        LoopObservation {
            observation_id: format!("observation-{}", action.action_id),
            action_id: action.action_id.clone(),
            success,
            summary: format!("{} executed", action.action_type.as_str()),
            evidence_id: Some(format!("evidence-{}", action.action_id)),
            failure_class: None,
        }
    }

    fn failed_observation(action: &AgentAction, failure_class: FailureClass) -> LoopObservation {
        LoopObservation {
            observation_id: format!("observation-{}", action.action_id),
            action_id: action.action_id.clone(),
            success: false,
            summary: format!("{} failed", action.action_type.as_str()),
            evidence_id: Some(format!("evidence-{}", action.action_id)),
            failure_class: Some(failure_class),
        }
    }

    #[test]
    fn happy_path_drives_synthesis_scaffold_build_validate_complete() {
        let (mut record, mut action) =
            AgentLoopReducer::start("session-1", "task-1", 3, 8, 1000).expect("start");
        assert_eq!(action.action_type, AgentActionType::SynthesizeProject);

        // SynthesizeProject -> ScaffoldProject
        let (next, decision) =
            AgentLoopReducer::advance(&record, &observation(&action, true), 1001)
                .expect("advance synthesis");
        record = next;
        let KernelDecision::Execute(next_action) = decision else {
            panic!("expected next action");
        };
        assert_eq!(next_action.action_type, AgentActionType::ScaffoldProject);
        action = next_action;

        // ScaffoldProject -> BuildArtifact
        let (next, decision) =
            AgentLoopReducer::advance(&record, &observation(&action, true), 1002)
                .expect("advance scaffold");
        record = next;
        let KernelDecision::Execute(next_action) = decision else {
            panic!("expected next action");
        };
        assert_eq!(next_action.action_type, AgentActionType::BuildArtifact);
        action = next_action;

        // BuildArtifact -> ValidateArtifact
        let (next, decision) =
            AgentLoopReducer::advance(&record, &observation(&action, true), 1003)
                .expect("advance build");
        record = next;
        let KernelDecision::Execute(next_action) = decision else {
            panic!("expected next action");
        };
        assert_eq!(next_action.action_type, AgentActionType::ValidateArtifact);
        action = next_action;

        // ValidateArtifact -> COMPLETE
        let (next, decision) =
            AgentLoopReducer::advance(&record, &observation(&action, true), 1004)
                .expect("advance validate");
        assert_eq!(next.state, AgentLoopState::Complete);
        assert_eq!(next.progress_status, ProgressStatus::Complete);
        assert_eq!(decision, KernelDecision::Finished);
        assert!(next.validate().is_ok());
    }

    #[test]
    fn build_failure_recovers_through_diagnosis_with_variation() {
        let (record, action) =
            AgentLoopReducer::start("session-1", "task-1", 3, 8, 1000).expect("start");
        // Synthesis succeeds.
        let (record, scaffold_action) =
            AgentLoopReducer::advance(&record, &observation(&action, true), 1001)
                .expect("advance synthesis");
        let KernelDecision::Execute(scaffold_action) = scaffold_action else {
            panic!("expected scaffold action");
        };
        assert_eq!(
            scaffold_action.action_type,
            AgentActionType::ScaffoldProject
        );
        // Scaffold succeeds.
        let (record, build_action) =
            AgentLoopReducer::advance(&record, &observation(&scaffold_action, true), 1002)
                .expect("advance scaffold");
        let KernelDecision::Execute(build_action) = build_action else {
            panic!("expected build action");
        };
        assert_eq!(build_action.action_type, AgentActionType::BuildArtifact);

        // Build fails with a compile error: kernel demands diagnosis first.
        let failure = failed_observation(&build_action, FailureClass::BuildFailed);
        let (record, decision) =
            AgentLoopReducer::advance(&record, &failure, 1003).expect("advance failure");
        let KernelDecision::Execute(diagnose) = decision else {
            panic!("expected diagnose action");
        };
        assert_eq!(diagnose.action_type, AgentActionType::DiagnoseFailure);
        assert_eq!(record.progress_status, ProgressStatus::Recovering);
        assert_eq!(record.variation_attempts, 1);
        assert_eq!(record.consecutive_failures, 1);

        // Diagnosis succeeds: retry the build with a different fingerprint.
        let (record, decision) =
            AgentLoopReducer::advance(&record, &observation(&diagnose, true), 1004)
                .expect("advance diagnosis");
        let KernelDecision::Execute(retry) = decision else {
            panic!("expected retry action");
        };
        assert_eq!(retry.action_type, AgentActionType::BuildArtifact);
        assert_ne!(retry.action_fingerprint, build_action.action_fingerprint);
        assert_ne!(retry.action_id, build_action.action_id);
        assert!(!retry.variation.is_empty());

        // Retried build succeeds: the failure streak resets and the loop
        // continues to validation.
        let (record, decision) =
            AgentLoopReducer::advance(&record, &observation(&retry, true), 1005)
                .expect("advance retry");
        match decision {
            KernelDecision::Execute(action) => {
                assert_eq!(action.action_type, AgentActionType::ValidateArtifact);
            }
            KernelDecision::Finished => panic!("unexpected finish"),
        }
        assert_eq!(record.consecutive_failures, 0);
        assert_eq!(record.completed_action_count, 4);
    }

    #[test]
    fn identical_unrecoverable_failures_end_in_replan() {
        // Environment failure is unrecoverable locally: immediate safe fail.
        let (record, action) =
            AgentLoopReducer::start("session-1", "task-1", 3, 8, 1000).expect("start");
        let (record, scaffold_action) =
            AgentLoopReducer::advance(&record, &observation(&action, true), 1001)
                .expect("synthesis");
        let KernelDecision::Execute(scaffold_action) = scaffold_action else {
            panic!("expected scaffold action");
        };
        let (record, build_action) =
            AgentLoopReducer::advance(&record, &observation(&scaffold_action, true), 1002)
                .expect("scaffold");
        let KernelDecision::Execute(build_action) = build_action else {
            panic!("expected build action");
        };
        let failure = failed_observation(&build_action, FailureClass::EnvironmentUnavailable);
        let (next, decision) = AgentLoopReducer::advance(&record, &failure, 1003).expect("advance");
        assert_eq!(next.state, AgentLoopState::Failed);
        assert_eq!(decision, KernelDecision::Finished);

        // Consecutive build failures exhaust the failure budget -> replan.
        let (record, action) =
            AgentLoopReducer::start("session-1", "task-2", 3, 32, 2000).expect("start");
        let (mut current, mut step) = (record.clone(), 2001u64);
        let mut current_action = action;
        loop {
            if current.state.is_terminal() {
                break;
            }
            let (next, decision) = AgentLoopReducer::advance(
                &current,
                &failed_observation(&current_action, FailureClass::BuildFailed),
                step,
            )
            .expect("advance");
            step += 1;
            current = next;
            current_action = match decision {
                KernelDecision::Execute(next_action) => next_action,
                KernelDecision::Finished => break,
            };
        }
        assert_eq!(current.state, AgentLoopState::Failed);
        assert!(current.consecutive_failures >= current.max_consecutive_failures);
    }

    #[test]
    fn iteration_budget_terminates_loop_instead_of_cycling() {
        let (record, action) =
            AgentLoopReducer::start("session-1", "task-1", 3, 2, 1000).expect("start");
        let (record, _) = AgentLoopReducer::advance(&record, &observation(&action, true), 1001)
            .expect("synthesis");
        let mut current = record;
        let mut step = 1002;
        loop {
            if current.state.is_terminal() {
                break;
            }
            let awaited = current
                .awaited_action_type()
                .expect("non-terminal loop awaits an action");
            let action = AgentAction {
                action_id: format!(
                    "action-loop-task-1-3-{}-{}",
                    awaited.as_str(),
                    current.iteration
                ),
                action_type: awaited,
                attempt: current.iteration,
                variation: "v".into(),
                action_fingerprint: format!("{awaited:?}:v"),
            };
            let observation = failed_observation(&action, FailureClass::BuildFailed);
            let (next, decision) =
                AgentLoopReducer::advance(&current, &observation, step).expect("advance");
            step += 1;
            current = next;
            if let KernelDecision::Finished = decision {
                break;
            }
        }
        assert_eq!(current.state, AgentLoopState::Exhausted);
        assert!(current.iteration <= current.iteration_budget);
    }

    #[test]
    fn cancellation_is_observed_immediately() {
        let (record, action) =
            AgentLoopReducer::start("session-1", "task-1", 3, 8, 1000).expect("start");
        let cancellation = failed_observation(&action, FailureClass::Cancelled);
        let (next, decision) =
            AgentLoopReducer::advance(&record, &cancellation, 1001).expect("advance");
        assert_eq!(next.state, AgentLoopState::Cancelled);
        assert_eq!(decision, KernelDecision::Finished);
    }

    #[test]
    fn terminal_loops_reject_further_observations() {
        let (record, action) =
            AgentLoopReducer::start("session-1", "task-1", 3, 2, 1000).expect("start");
        let cancellation = failed_observation(&action, FailureClass::Cancelled);
        let (terminal, _) =
            AgentLoopReducer::advance(&record, &cancellation, 1001).expect("advance");
        let error = AgentLoopReducer::advance(&terminal, &observation(&action, true), 1002)
            .expect_err("terminal");
        assert_eq!(error, AgentLoopError::TerminalState);
    }

    fn action_placeholder() -> AgentAction {
        AgentAction {
            action_id: "placeholder".into(),
            action_type: AgentActionType::ScaffoldProject,
            attempt: 1,
            variation: "none".into(),
            action_fingerprint: "SCAFFOLD:1".into(),
        }
    }

    #[test]
    fn record_round_trips_through_json() {
        let (record, _) =
            AgentLoopReducer::start("session-1", "task-1", 3, 8, 1000).expect("start");
        let json = serde_json::to_string(&record).expect("serialize");
        let reloaded: AgentLoopRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, reloaded);
    }
}
