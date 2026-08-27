//! Supervisor lifecycle and M7 background execution primitives.

#![forbid(unsafe_code)]

use nirman_domain::BackgroundContinuityState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub const M7_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorState {
    Starting,
    Running,
    Reconnecting,
    Reconciling,
    UserRequired,
    SafelyFailed,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseFence {
    pub lease_id: String,
    pub fence_token: u64,
    pub owner_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorSnapshot {
    pub state: SupervisorState,
    pub continuity: BackgroundContinuityState,
    pub heartbeat_count: u64,
    pub latest_checkpoint_id: Option<String>,
    pub active_fence: Option<LeaseFence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundRunState {
    Queued,
    Running,
    Paused,
    Recovering,
    UserRequired,
    Completed,
    Cancelled,
    SafelyFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    ResumeFromCheckpoint,
    RetryFromCheckpoint,
    ForkFromCheckpoint,
    RequestUserApproval,
    SafeFailure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerHeartbeatRecord {
    pub run_id: String,
    pub worker_id: String,
    pub lease_id: String,
    pub fence_token: u64,
    pub process_id: Option<u32>,
    pub observed_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRecord {
    pub run_id: String,
    pub previous_state: BackgroundRunState,
    pub next_state: BackgroundRunState,
    pub action: RecoveryAction,
    pub reason: String,
    pub checkpoint_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundRunRecord {
    pub schema_version: u16,
    pub run_id: String,
    pub project_id: String,
    pub task_id: String,
    pub worker_id: String,
    pub checkpoint_id: Option<String>,
    pub state: BackgroundRunState,
    pub last_heartbeat_epoch_seconds: u64,
    pub attempt: u32,
    pub recovery_action: Option<RecoveryAction>,
    pub failure_fingerprint: Option<String>,
    pub notification_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackgroundRunError {
    InvalidRecord(&'static str),
    InvalidTransition,
    MissingCheckpoint,
    StaleWorker,
}

impl fmt::Display for BackgroundRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRecord(field) => write!(formatter, "M7 run record is invalid: {field}"),
            Self::InvalidTransition => {
                formatter.write_str("M7 background run transition is invalid")
            }
            Self::MissingCheckpoint => {
                formatter.write_str("M7 recovery requires a verified checkpoint")
            }
            Self::StaleWorker => formatter.write_str("M7 worker heartbeat is stale"),
        }
    }
}

impl std::error::Error for BackgroundRunError {}

impl BackgroundRunRecord {
    pub fn validate(&self) -> Result<(), BackgroundRunError> {
        if self.schema_version != M7_SCHEMA_VERSION {
            return Err(BackgroundRunError::InvalidRecord("schemaVersion"));
        }
        for (field, value) in [
            ("runId", self.run_id.as_str()),
            ("projectId", self.project_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("workerId", self.worker_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(BackgroundRunError::InvalidRecord(field));
            }
        }
        if matches!(
            self.recovery_action,
            Some(
                RecoveryAction::ResumeFromCheckpoint
                    | RecoveryAction::RetryFromCheckpoint
                    | RecoveryAction::ForkFromCheckpoint
            )
        ) && self.checkpoint_id.is_none()
        {
            return Err(BackgroundRunError::MissingCheckpoint);
        }
        Ok(())
    }

    pub fn heartbeat(&mut self, now_epoch_seconds: u64) -> Result<(), BackgroundRunError> {
        self.validate()?;
        if matches!(
            self.state,
            BackgroundRunState::Completed
                | BackgroundRunState::Cancelled
                | BackgroundRunState::SafelyFailed
        ) {
            return Err(BackgroundRunError::InvalidTransition);
        }
        self.last_heartbeat_epoch_seconds = now_epoch_seconds;
        self.state = BackgroundRunState::Running;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), BackgroundRunError> {
        if !matches!(
            self.state,
            BackgroundRunState::Running | BackgroundRunState::Recovering
        ) {
            return Err(BackgroundRunError::InvalidTransition);
        }
        self.state = BackgroundRunState::Paused;
        Ok(())
    }

    pub fn resume_from_checkpoint(&mut self) -> Result<(), BackgroundRunError> {
        if self.checkpoint_id.is_none()
            || !matches!(
                self.state,
                BackgroundRunState::Paused
                    | BackgroundRunState::Recovering
                    | BackgroundRunState::UserRequired
            )
        {
            return Err(BackgroundRunError::MissingCheckpoint);
        }
        self.state = BackgroundRunState::Running;
        self.recovery_action = Some(RecoveryAction::ResumeFromCheckpoint);
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), BackgroundRunError> {
        if matches!(
            self.state,
            BackgroundRunState::Completed | BackgroundRunState::Cancelled
        ) {
            return Err(BackgroundRunError::InvalidTransition);
        }
        self.state = BackgroundRunState::Cancelled;
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), BackgroundRunError> {
        if !matches!(
            self.state,
            BackgroundRunState::Running | BackgroundRunState::Recovering
        ) {
            return Err(BackgroundRunError::InvalidTransition);
        }
        self.state = BackgroundRunState::Completed;
        Ok(())
    }
}

pub fn detect_stale_worker(
    record: &BackgroundRunRecord,
    now_epoch_seconds: u64,
    heartbeat_timeout_seconds: u64,
) -> Result<bool, BackgroundRunError> {
    record.validate()?;
    if !matches!(
        record.state,
        BackgroundRunState::Running | BackgroundRunState::Recovering
    ) {
        return Ok(false);
    }
    Ok(
        now_epoch_seconds.saturating_sub(record.last_heartbeat_epoch_seconds)
            > heartbeat_timeout_seconds,
    )
}

pub fn recover_interrupted_run(
    record: &mut BackgroundRunRecord,
    now_epoch_seconds: u64,
    heartbeat_timeout_seconds: u64,
) -> Result<RecoveryAction, BackgroundRunError> {
    if !detect_stale_worker(record, now_epoch_seconds, heartbeat_timeout_seconds)? {
        return Err(BackgroundRunError::StaleWorker);
    }
    if record.checkpoint_id.is_some() {
        record.state = BackgroundRunState::Recovering;
        record.recovery_action = Some(RecoveryAction::ResumeFromCheckpoint);
        record.attempt = record.attempt.saturating_add(1);
        Ok(RecoveryAction::ResumeFromCheckpoint)
    } else {
        record.state = BackgroundRunState::SafelyFailed;
        record.recovery_action = Some(RecoveryAction::SafeFailure);
        Ok(RecoveryAction::SafeFailure)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BackgroundScheduler {
    queued: Vec<BackgroundRunRecord>,
    active: BTreeMap<String, WorkerHeartbeatRecord>,
}

impl BackgroundScheduler {
    pub fn enqueue(&mut self, record: BackgroundRunRecord) -> Result<(), BackgroundRunError> {
        record.validate()?;
        if self
            .queued
            .iter()
            .any(|queued| queued.run_id == record.run_id)
            || self.active.contains_key(&record.run_id)
        {
            return Err(BackgroundRunError::InvalidTransition);
        }
        self.queued.push(record);
        Ok(())
    }

    pub fn claim_next(
        &mut self,
        worker_id: &str,
        lease_id: &str,
        fence_token: u64,
        process_id: Option<u32>,
        now_epoch_seconds: u64,
    ) -> Result<Option<BackgroundRunRecord>, BackgroundRunError> {
        let Some(mut record) = self.queued.first().cloned() else {
            return Ok(None);
        };
        self.queued.remove(0);
        if worker_id.trim().is_empty() || lease_id.trim().is_empty() {
            return Err(BackgroundRunError::InvalidRecord("worker/lease"));
        }
        record.worker_id = worker_id.into();
        record.state = BackgroundRunState::Running;
        record.last_heartbeat_epoch_seconds = now_epoch_seconds;
        self.active.insert(
            record.run_id.clone(),
            WorkerHeartbeatRecord {
                run_id: record.run_id.clone(),
                worker_id: worker_id.into(),
                lease_id: lease_id.into(),
                fence_token,
                process_id,
                observed_at_epoch_seconds: now_epoch_seconds,
            },
        );
        Ok(Some(record))
    }

    pub fn heartbeat(
        &mut self,
        run_id: &str,
        worker_id: &str,
        lease_id: &str,
        fence_token: u64,
        now_epoch_seconds: u64,
    ) -> Result<WorkerHeartbeatRecord, BackgroundRunError> {
        let heartbeat = self
            .active
            .get_mut(run_id)
            .ok_or(BackgroundRunError::InvalidRecord("runId"))?;
        if heartbeat.worker_id != worker_id
            || heartbeat.lease_id != lease_id
            || heartbeat.fence_token != fence_token
        {
            return Err(BackgroundRunError::InvalidTransition);
        }
        heartbeat.observed_at_epoch_seconds = now_epoch_seconds;
        Ok(heartbeat.clone())
    }

    pub fn release(&mut self, run_id: &str) -> Option<WorkerHeartbeatRecord> {
        self.active.remove(run_id)
    }

    pub fn queued(&self) -> &[BackgroundRunRecord] {
        &self.queued
    }

    pub fn active_heartbeats(&self) -> &BTreeMap<String, WorkerHeartbeatRecord> {
        &self.active
    }
}

pub fn retry_from_checkpoint(
    record: &mut BackgroundRunRecord,
) -> Result<RecoveryRecord, BackgroundRunError> {
    let previous_state = record.state;
    if record.checkpoint_id.is_none() {
        return Err(BackgroundRunError::MissingCheckpoint);
    }
    if !matches!(
        record.state,
        BackgroundRunState::Recovering | BackgroundRunState::Paused
    ) {
        return Err(BackgroundRunError::InvalidTransition);
    }
    record.state = BackgroundRunState::Running;
    record.attempt = record.attempt.saturating_add(1);
    record.recovery_action = Some(RecoveryAction::RetryFromCheckpoint);
    Ok(RecoveryRecord {
        run_id: record.run_id.clone(),
        previous_state,
        next_state: record.state,
        action: RecoveryAction::RetryFromCheckpoint,
        reason: "retry authorized from the verified checkpoint".into(),
        checkpoint_id: record.checkpoint_id.clone(),
    })
}

pub fn fork_from_checkpoint(
    record: &BackgroundRunRecord,
    fork_run_id: &str,
) -> Result<BackgroundRunRecord, BackgroundRunError> {
    if record.checkpoint_id.is_none() {
        return Err(BackgroundRunError::MissingCheckpoint);
    }
    if fork_run_id.trim().is_empty() {
        return Err(BackgroundRunError::InvalidRecord("forkRunId"));
    }
    let mut fork = record.clone();
    fork.run_id = fork_run_id.into();
    fork.state = BackgroundRunState::Queued;
    fork.attempt = 1;
    fork.recovery_action = Some(RecoveryAction::ForkFromCheckpoint);
    Ok(fork)
}

pub fn reconcile_unknown_outcome(
    record: &mut BackgroundRunRecord,
    external_effect_fingerprint: &str,
    resolved: bool,
) -> Result<RecoveryRecord, BackgroundRunError> {
    let previous_state = record.state;
    if external_effect_fingerprint.trim().is_empty() {
        return Err(BackgroundRunError::InvalidRecord(
            "externalEffectFingerprint",
        ));
    }
    if !matches!(
        record.state,
        BackgroundRunState::Recovering | BackgroundRunState::UserRequired
    ) {
        return Err(BackgroundRunError::InvalidTransition);
    }
    record.failure_fingerprint = Some(external_effect_fingerprint.into());
    record.state = if resolved {
        BackgroundRunState::Running
    } else {
        BackgroundRunState::UserRequired
    };
    Ok(RecoveryRecord {
        run_id: record.run_id.clone(),
        previous_state,
        next_state: record.state,
        action: if resolved {
            RecoveryAction::ResumeFromCheckpoint
        } else {
            RecoveryAction::RequestUserApproval
        },
        reason: if resolved {
            "unknown external outcome reconciled by authoritative read-back".into()
        } else {
            "unknown external outcome remains unresolved and requires user decision".into()
        },
        checkpoint_id: record.checkpoint_id.clone(),
    })
}

#[derive(Debug)]
pub struct Supervisor {
    snapshot: SupervisorSnapshot,
}

impl Supervisor {
    pub fn start() -> Self {
        Self {
            snapshot: SupervisorSnapshot {
                state: SupervisorState::Starting,
                continuity: BackgroundContinuityState::ActiveBackground,
                heartbeat_count: 0,
                latest_checkpoint_id: None,
                active_fence: None,
            },
        }
    }

    pub fn heartbeat(&mut self) {
        self.snapshot.heartbeat_count = self.snapshot.heartbeat_count.saturating_add(1);
        self.snapshot.state = SupervisorState::Running;
    }

    pub fn register_lease(
        &mut self,
        lease_id: impl Into<String>,
        owner_id: impl Into<String>,
        fence_token: u64,
    ) {
        self.snapshot.active_fence = Some(LeaseFence {
            lease_id: lease_id.into(),
            owner_id: owner_id.into(),
            fence_token,
        });
    }

    pub fn restart_from_checkpoint(&mut self, checkpoint_id: impl Into<String>) {
        self.snapshot.state = SupervisorState::Reconnecting;
        self.snapshot.continuity = BackgroundContinuityState::UiDisconnected;
        self.snapshot.active_fence = None;
        self.reconcile_after_restart(checkpoint_id);
    }

    pub fn reconcile_after_restart(&mut self, checkpoint_id: impl Into<String>) {
        self.snapshot.state = SupervisorState::Reconciling;
        self.snapshot.continuity = BackgroundContinuityState::Reconciling;
        self.snapshot.latest_checkpoint_id = Some(checkpoint_id.into());
    }

    pub fn mark_user_required(&mut self) {
        self.snapshot.state = SupervisorState::UserRequired;
        self.snapshot.continuity = BackgroundContinuityState::UserRequired;
    }

    pub fn safe_fail(&mut self) {
        self.snapshot.state = SupervisorState::SafelyFailed;
        self.snapshot.continuity = BackgroundContinuityState::SafelyFailed;
    }

    pub fn snapshot(&self) -> SupervisorSnapshot {
        self.snapshot.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(checkpoint_id: Option<&str>) -> BackgroundRunRecord {
        BackgroundRunRecord {
            schema_version: M7_SCHEMA_VERSION,
            run_id: "run-1".into(),
            project_id: "project-1".into(),
            task_id: "task-1".into(),
            worker_id: "worker-1".into(),
            checkpoint_id: checkpoint_id.map(str::to_owned),
            state: BackgroundRunState::Running,
            last_heartbeat_epoch_seconds: 10,
            attempt: 1,
            recovery_action: None,
            failure_fingerprint: None,
            notification_kind: None,
        }
    }

    #[test]
    fn restart_requires_reconciliation_before_running() {
        let mut supervisor = Supervisor::start();
        supervisor.register_lease("lease-1", "worker-1", 1);
        supervisor.reconcile_after_restart("checkpoint-1");
        assert_eq!(supervisor.snapshot().state, SupervisorState::Reconciling);
        assert_eq!(
            supervisor.snapshot().continuity,
            BackgroundContinuityState::Reconciling
        );
        supervisor.heartbeat();
        assert_eq!(supervisor.snapshot().state, SupervisorState::Running);
    }

    #[test]
    fn stale_worker_recovers_from_checkpoint_and_increments_attempt() {
        let mut run = record(Some("checkpoint-1"));
        assert!(detect_stale_worker(&run, 30, 5).expect("stale check"));
        assert_eq!(
            recover_interrupted_run(&mut run, 30, 5).expect("recovery"),
            RecoveryAction::ResumeFromCheckpoint
        );
        assert_eq!(run.state, BackgroundRunState::Recovering);
        assert_eq!(run.attempt, 2);
        run.resume_from_checkpoint().expect("resume");
        assert_eq!(run.state, BackgroundRunState::Running);
    }

    #[test]
    fn scheduler_claims_runs_and_rejects_wrong_fence_heartbeat() {
        let mut scheduler = BackgroundScheduler::default();
        scheduler
            .enqueue(record(Some("checkpoint-1")))
            .expect("enqueue");
        let claimed = scheduler
            .claim_next("worker-1", "lease-1", 7, Some(42), 20)
            .expect("claim")
            .expect("queued run");
        assert_eq!(claimed.state, BackgroundRunState::Running);
        assert!(scheduler
            .heartbeat("run-1", "worker-1", "lease-1", 7, 21)
            .is_ok());
        assert_eq!(
            scheduler.heartbeat("run-1", "worker-1", "lease-1", 8, 22),
            Err(BackgroundRunError::InvalidTransition)
        );
    }

    #[test]
    fn retry_and_fork_keep_checkpoint_lineage() {
        let mut run = record(Some("checkpoint-1"));
        run.state = BackgroundRunState::Recovering;
        let retry = retry_from_checkpoint(&mut run).expect("retry");
        assert_eq!(retry.action, RecoveryAction::RetryFromCheckpoint);
        assert_eq!(run.attempt, 2);
        let fork = fork_from_checkpoint(&run, "run-fork-1").expect("fork");
        assert_eq!(fork.run_id, "run-fork-1");
        assert_eq!(fork.state, BackgroundRunState::Queued);
        assert_eq!(fork.checkpoint_id, Some("checkpoint-1".into()));
    }

    #[test]
    fn unknown_outcome_requires_reconciliation_or_user_decision() {
        let mut run = record(Some("checkpoint-1"));
        run.state = BackgroundRunState::Recovering;
        let unresolved = reconcile_unknown_outcome(&mut run, "effect-1", false).expect("unknown");
        assert_eq!(unresolved.action, RecoveryAction::RequestUserApproval);
        assert_eq!(run.state, BackgroundRunState::UserRequired);
        run.state = BackgroundRunState::Recovering;
        let resolved = reconcile_unknown_outcome(&mut run, "effect-1", true).expect("resolved");
        assert_eq!(resolved.action, RecoveryAction::ResumeFromCheckpoint);
        assert_eq!(run.state, BackgroundRunState::Running);
    }

    #[test]
    fn interrupted_run_without_checkpoint_fails_safely() {
        let mut run = record(None);
        assert_eq!(
            recover_interrupted_run(&mut run, 30, 5).expect("safe failure"),
            RecoveryAction::SafeFailure
        );
        assert_eq!(run.state, BackgroundRunState::SafelyFailed);
    }
}
