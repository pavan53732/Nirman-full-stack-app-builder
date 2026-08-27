#![forbid(unsafe_code)]

use nirman_domain::{ProjectId, Revision, TaskId};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const M5_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStage {
    Inspect,
    Plan,
    Checkpoint,
    Mutate,
    Build,
    InstallLaunch,
    Observe,
    Validate,
    Repair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerOutcome {
    Success,
    Failure,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerLifecycle {
    Ready,
    Running,
    Recovering,
    Completed,
    SafelyFailed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerContract {
    pub schema_version: u16,
    pub worker_id: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub capability_ceiling: Vec<String>,
    pub workspace_root: String,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub max_attempts: u8,
    pub evidence_requirements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerObservation {
    pub stage: WorkerStage,
    pub outcome: WorkerOutcome,
    pub source_revision: Revision,
    pub checkpoint_id: Option<String>,
    pub changed_paths: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostic_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerHandoff {
    pub worker_id: String,
    pub task_id: TaskId,
    pub completed_stage: WorkerStage,
    pub next_stage: Option<WorkerStage>,
    pub lifecycle: WorkerLifecycle,
    pub source_revision: Revision,
    pub checkpoint_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostic_ref: Option<String>,
    pub attempt: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerLoopError {
    InvalidContract(&'static str),
    WrongStage {
        expected: WorkerStage,
        received: WorkerStage,
    },
    MissingEvidence,
    MissingCheckpoint,
    MissingDiagnostic,
    AttemptsExhausted,
    InvalidRepair,
}

impl fmt::Display for WorkerLoopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract(field) => {
                write!(formatter, "M5 worker contract is invalid: {field}")
            }
            Self::WrongStage { expected, received } => {
                write!(
                    formatter,
                    "M5 worker stage mismatch: expected {expected:?}, received {received:?}"
                )
            }
            Self::MissingEvidence => formatter.write_str("M5 worker observation has no evidence"),
            Self::MissingCheckpoint => {
                formatter.write_str("M5 worker observation has no checkpoint")
            }
            Self::MissingDiagnostic => {
                formatter.write_str("M5 failed worker observation has no diagnostic")
            }
            Self::AttemptsExhausted => {
                formatter.write_str("M5 worker repair attempts are exhausted")
            }
            Self::InvalidRepair => {
                formatter.write_str("M5 worker repair must follow a failed observation")
            }
        }
    }
}

impl std::error::Error for WorkerLoopError {}

impl WorkerContract {
    pub fn validate(&self) -> Result<(), WorkerLoopError> {
        if self.schema_version != M5_SCHEMA_VERSION {
            return Err(WorkerLoopError::InvalidContract("schemaVersion"));
        }
        if self.worker_id.trim().is_empty() {
            return Err(WorkerLoopError::InvalidContract("workerId"));
        }
        if self.project_id.0.trim().is_empty() {
            return Err(WorkerLoopError::InvalidContract("projectId"));
        }
        if self.task_id.0.trim().is_empty() {
            return Err(WorkerLoopError::InvalidContract("taskId"));
        }
        if self.workspace_root.trim().is_empty() || self.allowed_paths.is_empty() {
            return Err(WorkerLoopError::InvalidContract("workspace"));
        }
        if self.max_attempts == 0 || self.evidence_requirements.is_empty() {
            return Err(WorkerLoopError::InvalidContract("limits/evidence"));
        }
        if self
            .allowed_paths
            .iter()
            .any(|path| self.denied_paths.iter().any(|denied| denied == path))
        {
            return Err(WorkerLoopError::InvalidContract("path-overlap"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleWorkerLoop {
    contract: WorkerContract,
    lifecycle: WorkerLifecycle,
    stage: WorkerStage,
    attempt: u8,
    source_revision: Revision,
    checkpoint_id: Option<String>,
    history: Vec<WorkerObservation>,
    repair_pending: bool,
}

impl SingleWorkerLoop {
    pub fn start(contract: WorkerContract) -> Result<Self, WorkerLoopError> {
        contract.validate()?;
        Ok(Self {
            contract,
            lifecycle: WorkerLifecycle::Ready,
            stage: WorkerStage::Inspect,
            attempt: 1,
            source_revision: Revision(0),
            checkpoint_id: None,
            history: Vec::new(),
            repair_pending: false,
        })
    }

    pub fn lifecycle(&self) -> WorkerLifecycle {
        self.lifecycle
    }

    pub fn stage(&self) -> WorkerStage {
        self.stage
    }

    pub fn history(&self) -> &[WorkerObservation] {
        &self.history
    }

    pub fn observe(
        &mut self,
        observation: WorkerObservation,
    ) -> Result<WorkerHandoff, WorkerLoopError> {
        if observation.stage != self.stage {
            return Err(WorkerLoopError::WrongStage {
                expected: self.stage,
                received: observation.stage,
            });
        }
        if observation.outcome == WorkerOutcome::Unknown && observation.diagnostic_ref.is_none() {
            return Err(WorkerLoopError::MissingDiagnostic);
        }
        if observation.outcome == WorkerOutcome::Success && observation.evidence_refs.is_empty() {
            return Err(WorkerLoopError::MissingEvidence);
        }
        if observation.source_revision < self.source_revision {
            return Err(WorkerLoopError::InvalidContract("staleSourceRevision"));
        }
        self.lifecycle = WorkerLifecycle::Running;
        self.source_revision = observation.source_revision;
        if observation.checkpoint_id.is_some() {
            self.checkpoint_id = observation.checkpoint_id.clone();
        }
        self.history.push(observation.clone());
        if observation.outcome != WorkerOutcome::Success {
            self.lifecycle = WorkerLifecycle::Recovering;
            self.repair_pending = true;
            self.stage = WorkerStage::Repair;
            if observation.diagnostic_ref.is_none() {
                return Err(WorkerLoopError::MissingDiagnostic);
            }
            return Ok(self.handoff(observation, Some(WorkerStage::Repair)));
        }
        if observation.stage == WorkerStage::Repair {
            if !self.repair_pending {
                return Err(WorkerLoopError::InvalidRepair);
            }
            self.repair_pending = false;
            self.stage = WorkerStage::Checkpoint;
            return Ok(self.handoff(observation, Some(self.stage)));
        }
        match observation.stage {
            WorkerStage::Inspect => self.stage = WorkerStage::Plan,
            WorkerStage::Plan => self.stage = WorkerStage::Checkpoint,
            WorkerStage::Checkpoint => self.stage = WorkerStage::Mutate,
            WorkerStage::Mutate => self.stage = WorkerStage::Build,
            WorkerStage::Build => self.stage = WorkerStage::InstallLaunch,
            WorkerStage::InstallLaunch => self.stage = WorkerStage::Observe,
            WorkerStage::Observe => self.stage = WorkerStage::Validate,
            WorkerStage::Validate => {
                self.lifecycle = WorkerLifecycle::Completed;
                self.stage = WorkerStage::Checkpoint;
                return Ok(self.handoff(observation, None));
            }
            WorkerStage::Repair => return Err(WorkerLoopError::InvalidRepair),
        }
        Ok(self.handoff(observation, Some(self.stage)))
    }

    pub fn cancel(&mut self) -> WorkerHandoff {
        self.lifecycle = WorkerLifecycle::Cancelled;
        WorkerHandoff {
            worker_id: self.contract.worker_id.clone(),
            task_id: self.contract.task_id.clone(),
            completed_stage: self.stage,
            next_stage: None,
            lifecycle: self.lifecycle,
            source_revision: self.source_revision,
            checkpoint_id: self.checkpoint_id.clone(),
            evidence_refs: vec![],
            diagnostic_ref: None,
            attempt: self.attempt,
        }
    }

    fn handoff(
        &self,
        observation: WorkerObservation,
        next_stage: Option<WorkerStage>,
    ) -> WorkerHandoff {
        WorkerHandoff {
            worker_id: self.contract.worker_id.clone(),
            task_id: self.contract.task_id.clone(),
            completed_stage: observation.stage,
            next_stage,
            lifecycle: self.lifecycle,
            source_revision: self.source_revision,
            checkpoint_id: self.checkpoint_id.clone(),
            evidence_refs: observation.evidence_refs,
            diagnostic_ref: observation.diagnostic_ref,
            attempt: self.attempt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> WorkerContract {
        WorkerContract {
            schema_version: M5_SCHEMA_VERSION,
            worker_id: "worker-1".into(),
            project_id: ProjectId("project-1".into()),
            task_id: TaskId("task-1".into()),
            capability_ceiling: vec!["android.inspect".into(), "android.build".into()],
            workspace_root: "/tmp/nirman-project".into(),
            allowed_paths: vec!["/tmp/nirman-project".into()],
            denied_paths: vec!["/home/ubuntu/.ssh".into()],
            max_attempts: 3,
            evidence_requirements: vec!["build".into(), "runtime".into()],
        }
    }

    fn success(stage: WorkerStage, revision: u64, checkpoint: Option<&str>) -> WorkerObservation {
        WorkerObservation {
            stage,
            outcome: WorkerOutcome::Success,
            source_revision: Revision(revision),
            checkpoint_id: checkpoint.map(str::to_owned),
            changed_paths: vec![],
            evidence_refs: vec![format!("evidence-{revision}")],
            diagnostic_ref: None,
        }
    }

    #[test]
    fn single_worker_follows_documented_stage_order_and_completes_with_evidence() {
        let mut worker = SingleWorkerLoop::start(contract()).expect("worker");
        let stages = [
            WorkerStage::Inspect,
            WorkerStage::Plan,
            WorkerStage::Checkpoint,
            WorkerStage::Mutate,
            WorkerStage::Build,
            WorkerStage::InstallLaunch,
            WorkerStage::Observe,
            WorkerStage::Validate,
        ];
        for (revision, stage) in stages.into_iter().enumerate() {
            let handoff = worker
                .observe(success(stage, revision as u64, Some("checkpoint-1")))
                .expect("handoff");
            assert_eq!(handoff.completed_stage, stage);
        }
        assert_eq!(worker.lifecycle(), WorkerLifecycle::Completed);
        assert_eq!(worker.history().len(), 8);
    }

    #[test]
    fn failed_build_requires_diagnostic_then_repair_before_checkpoint() {
        let mut worker = SingleWorkerLoop::start(contract()).expect("worker");
        for stage in [
            WorkerStage::Inspect,
            WorkerStage::Plan,
            WorkerStage::Checkpoint,
            WorkerStage::Mutate,
        ] {
            worker
                .observe(success(stage, 1, Some("checkpoint-1")))
                .expect("stage");
        }
        let mut failed = success(WorkerStage::Build, 1, Some("checkpoint-1"));
        failed.outcome = WorkerOutcome::Failure;
        failed.diagnostic_ref = Some("diagnostic-build-1".into());
        assert_eq!(
            worker.observe(failed).expect("repair handoff").next_stage,
            Some(WorkerStage::Repair)
        );
        let repair = success(WorkerStage::Repair, 1, Some("checkpoint-2"));
        assert_eq!(
            worker
                .observe(repair)
                .expect("checkpoint handoff")
                .next_stage,
            Some(WorkerStage::Checkpoint)
        );
        assert_eq!(worker.stage(), WorkerStage::Checkpoint);
    }

    #[test]
    fn successful_stage_without_evidence_is_rejected() {
        let mut worker = SingleWorkerLoop::start(contract()).expect("worker");
        let mut observation = success(WorkerStage::Inspect, 1, None);
        observation.evidence_refs.clear();
        assert_eq!(
            worker.observe(observation),
            Err(WorkerLoopError::MissingEvidence)
        );
    }
}
