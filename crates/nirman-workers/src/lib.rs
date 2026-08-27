#![forbid(unsafe_code)]

use nirman_domain::{ProjectId, Revision, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

pub const M8_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerRole {
    Architecture,
    Implementation,
    Debugging,
    Testing,
    Security,
    VisualQa,
    Performance,
    Release,
    Reconciliation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceIsolation {
    GitWorktree,
    CopyOnWrite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerLease {
    pub lease_id: String,
    pub worker_id: String,
    pub fence_token: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationTask {
    pub schema_version: u16,
    pub task_id: String,
    pub parent_task_id: String,
    pub worker_id: String,
    pub role: WorkerRole,
    pub capability_ceiling: Vec<String>,
    pub workspace_root: String,
    pub parent_workspace_root: String,
    pub isolation: WorkspaceIsolation,
    pub dependencies: Vec<String>,
    pub expected_source_revision: Revision,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerHandoffRecord {
    pub message_id: String,
    pub task_id: String,
    pub worker_id: String,
    pub lease_id: String,
    pub fence_token: u64,
    pub source_revision: Revision,
    pub outcome: WorkerOutcome,
    pub changed_paths: Vec<String>,
    pub changed_symbols: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerConflict {
    pub left_task_id: String,
    pub right_task_id: String,
    pub conflicting_paths: Vec<String>,
    pub conflicting_symbols: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoordinationError {
    InvalidTask(&'static str),
    CapabilityCeilingExceeded,
    WorkspaceNotIsolated,
    DependencyIncomplete,
    LeaseRequired,
    LeaseFenced,
    DuplicateHandoff,
    MissingHandoff,
    Conflict(Vec<WorkerConflict>),
    EvidenceRequired,
}

impl fmt::Display for CoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTask(field) => {
                write!(formatter, "M8 coordination task is invalid: {field}")
            }
            Self::CapabilityCeilingExceeded => {
                formatter.write_str("M8 child capability exceeds parent ceiling")
            }
            Self::WorkspaceNotIsolated => {
                formatter.write_str("M8 worker workspace is not isolated from the parent")
            }
            Self::DependencyIncomplete => {
                formatter.write_str("M8 worker dependencies are incomplete")
            }
            Self::LeaseRequired => formatter.write_str("M8 worker lease is required"),
            Self::LeaseFenced => formatter.write_str("M8 worker lease is stale or fenced"),
            Self::DuplicateHandoff => formatter.write_str("M8 worker handoff was already recorded"),
            Self::MissingHandoff => {
                formatter.write_str("M8 reconciliation is missing a worker handoff")
            }
            Self::Conflict(conflicts) => write!(
                formatter,
                "M8 worker conflicts detected: {}",
                conflicts.len()
            ),
            Self::EvidenceRequired => formatter.write_str("M8 worker handoff has no evidence"),
        }
    }
}

impl std::error::Error for CoordinationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiWorkerCoordinator {
    parent: WorkerContract,
    tasks: BTreeMap<String, CoordinationTask>,
    leases: BTreeMap<String, WorkerLease>,
    handoffs: BTreeMap<String, WorkerHandoffRecord>,
    next_fence_token: u64,
}

impl MultiWorkerCoordinator {
    pub fn new(parent: WorkerContract) -> Result<Self, CoordinationError> {
        parent
            .validate()
            .map_err(|_| CoordinationError::InvalidTask("parentContract"))?;
        Ok(Self {
            parent,
            tasks: BTreeMap::new(),
            leases: BTreeMap::new(),
            handoffs: BTreeMap::new(),
            next_fence_token: 1,
        })
    }

    pub fn register_task(&mut self, task: CoordinationTask) -> Result<(), CoordinationError> {
        if task.schema_version != M8_SCHEMA_VERSION
            || task.task_id.trim().is_empty()
            || task.parent_task_id != self.parent.task_id.0
            || task.worker_id.trim().is_empty()
            || task.workspace_root.trim().is_empty()
            || task.parent_workspace_root != self.parent.workspace_root
            || task.required_evidence.is_empty()
        {
            return Err(CoordinationError::InvalidTask("identity/scope/evidence"));
        }
        if task.workspace_root == self.parent.workspace_root {
            return Err(CoordinationError::WorkspaceNotIsolated);
        }
        if task
            .capability_ceiling
            .iter()
            .any(|capability| !self.parent.capability_ceiling.contains(capability))
        {
            return Err(CoordinationError::CapabilityCeilingExceeded);
        }
        if self.tasks.contains_key(&task.task_id) {
            return Err(CoordinationError::InvalidTask("duplicateTask"));
        }
        self.tasks.insert(task.task_id.clone(), task);
        Ok(())
    }

    pub fn claim(
        &mut self,
        task_id: &str,
        now_epoch_seconds: u64,
        lease_duration_seconds: u64,
    ) -> Result<WorkerLease, CoordinationError> {
        let task = self
            .tasks
            .get(task_id)
            .ok_or(CoordinationError::InvalidTask("taskId"))?;
        if task
            .dependencies
            .iter()
            .any(|dependency| !self.handoffs.contains_key(dependency))
        {
            return Err(CoordinationError::DependencyIncomplete);
        }
        if self.leases.contains_key(task_id) {
            return Err(CoordinationError::LeaseRequired);
        }
        let lease = WorkerLease {
            lease_id: format!("lease-{task_id}"),
            worker_id: task.worker_id.clone(),
            fence_token: self.next_fence_token,
            expires_at_epoch_seconds: now_epoch_seconds.saturating_add(lease_duration_seconds),
        };
        self.next_fence_token = self.next_fence_token.saturating_add(1);
        self.leases.insert(task_id.into(), lease.clone());
        Ok(lease)
    }

    pub fn record_handoff(
        &mut self,
        handoff: WorkerHandoffRecord,
    ) -> Result<(), CoordinationError> {
        let task = self
            .tasks
            .get(&handoff.task_id)
            .ok_or(CoordinationError::InvalidTask("taskId"))?;
        let lease = self
            .leases
            .get(&handoff.task_id)
            .ok_or(CoordinationError::LeaseRequired)?;
        if lease.lease_id != handoff.lease_id
            || lease.worker_id != handoff.worker_id
            || lease.fence_token != handoff.fence_token
        {
            return Err(CoordinationError::LeaseFenced);
        }
        if handoff.source_revision < task.expected_source_revision {
            return Err(CoordinationError::InvalidTask("staleSourceRevision"));
        }
        if handoff.evidence_refs.is_empty() {
            return Err(CoordinationError::EvidenceRequired);
        }
        if self.handoffs.contains_key(&handoff.task_id) {
            return Err(CoordinationError::DuplicateHandoff);
        }
        self.handoffs.insert(handoff.task_id.clone(), handoff);
        self.leases.remove(&task.task_id);
        Ok(())
    }

    pub fn detect_conflicts(&self) -> Vec<WorkerConflict> {
        let handoffs: Vec<&WorkerHandoffRecord> = self.handoffs.values().collect();
        let mut conflicts = Vec::new();
        for (left_index, left) in handoffs.iter().enumerate() {
            for right in handoffs.iter().skip(left_index + 1) {
                let paths: Vec<String> = left
                    .changed_paths
                    .iter()
                    .filter(|path| right.changed_paths.contains(path))
                    .cloned()
                    .collect();
                let symbols: Vec<String> = left
                    .changed_symbols
                    .iter()
                    .filter(|symbol| right.changed_symbols.contains(symbol))
                    .cloned()
                    .collect();
                if !paths.is_empty() || !symbols.is_empty() {
                    conflicts.push(WorkerConflict {
                        left_task_id: left.task_id.clone(),
                        right_task_id: right.task_id.clone(),
                        conflicting_paths: paths,
                        conflicting_symbols: symbols,
                    });
                }
            }
        }
        conflicts
    }

    pub fn reconcile(&self) -> Result<String, CoordinationError> {
        if self.handoffs.len() != self.tasks.len() {
            return Err(CoordinationError::MissingHandoff);
        }
        let conflicts = self.detect_conflicts();
        if !conflicts.is_empty() {
            return Err(CoordinationError::Conflict(conflicts));
        }
        if self
            .handoffs
            .values()
            .any(|handoff| handoff.outcome != WorkerOutcome::Success)
        {
            return Err(CoordinationError::InvalidTask("workerOutcome"));
        }
        Ok(format!("m8-integration-checkpoint-{}", self.handoffs.len()))
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

#[cfg(test)]
mod m8_tests {
    use super::*;

    fn parent() -> WorkerContract {
        WorkerContract {
            schema_version: M5_SCHEMA_VERSION,
            worker_id: "coordinator".into(),
            project_id: ProjectId("project-m8".into()),
            task_id: TaskId("root-task".into()),
            capability_ceiling: vec![
                "android.inspect".into(),
                "android.build".into(),
                "android.test".into(),
            ],
            workspace_root: "/workspace/root".into(),
            allowed_paths: vec!["/workspace/root".into()],
            denied_paths: vec!["/workspace/root/.git".into()],
            max_attempts: 3,
            evidence_requirements: vec!["handoff".into()],
        }
    }

    fn child(
        task_id: &str,
        worker_id: &str,
        workspace_root: &str,
        capability: &str,
    ) -> CoordinationTask {
        CoordinationTask {
            schema_version: M8_SCHEMA_VERSION,
            task_id: task_id.into(),
            parent_task_id: "root-task".into(),
            worker_id: worker_id.into(),
            role: WorkerRole::Implementation,
            capability_ceiling: vec![capability.into()],
            workspace_root: workspace_root.into(),
            parent_workspace_root: "/workspace/root".into(),
            isolation: WorkspaceIsolation::GitWorktree,
            dependencies: vec![],
            expected_source_revision: Revision(1),
            required_evidence: vec!["build-evidence".into()],
        }
    }

    fn handoff(
        task_id: &str,
        worker_id: &str,
        lease: &WorkerLease,
        path: &str,
    ) -> WorkerHandoffRecord {
        WorkerHandoffRecord {
            message_id: format!("message-{task_id}"),
            task_id: task_id.into(),
            worker_id: worker_id.into(),
            lease_id: lease.lease_id.clone(),
            fence_token: lease.fence_token,
            source_revision: Revision(1),
            outcome: WorkerOutcome::Success,
            changed_paths: vec![path.into()],
            changed_symbols: vec![format!("symbol-{task_id}")],
            evidence_refs: vec![format!("evidence-{task_id}")],
        }
    }

    #[test]
    fn isolated_workers_reconcile_after_scoped_handoffs() {
        let mut coordinator = MultiWorkerCoordinator::new(parent()).expect("coordinator");
        coordinator
            .register_task(child("task-a", "worker-a", "/workspace/a", "android.build"))
            .expect("task a");
        coordinator
            .register_task(child("task-b", "worker-b", "/workspace/b", "android.test"))
            .expect("task b");
        let lease_a = coordinator.claim("task-a", 10, 30).expect("lease a");
        let lease_b = coordinator.claim("task-b", 10, 30).expect("lease b");
        coordinator
            .record_handoff(handoff("task-a", "worker-a", &lease_a, "app/A.java"))
            .expect("handoff a");
        coordinator
            .record_handoff(handoff("task-b", "worker-b", &lease_b, "app/B.java"))
            .expect("handoff b");
        assert_eq!(
            coordinator.reconcile().expect("reconcile"),
            "m8-integration-checkpoint-2"
        );
    }

    #[test]
    fn capability_and_workspace_boundaries_are_rejected() {
        let mut coordinator = MultiWorkerCoordinator::new(parent()).expect("coordinator");
        assert_eq!(
            coordinator.register_task(child(
                "task-bad",
                "worker-bad",
                "/workspace/bad",
                "android.deploy"
            )),
            Err(CoordinationError::CapabilityCeilingExceeded)
        );
        assert_eq!(
            coordinator.register_task(child(
                "task-root",
                "worker-root",
                "/workspace/root",
                "android.build"
            )),
            Err(CoordinationError::WorkspaceNotIsolated)
        );
    }

    #[test]
    fn stale_lease_and_changed_path_conflict_block_reconciliation() {
        let mut coordinator = MultiWorkerCoordinator::new(parent()).expect("coordinator");
        coordinator
            .register_task(child("task-a", "worker-a", "/workspace/a", "android.build"))
            .expect("task a");
        coordinator
            .register_task(child("task-b", "worker-b", "/workspace/b", "android.build"))
            .expect("task b");
        let lease_a = coordinator.claim("task-a", 10, 30).expect("lease a");
        let lease_b = coordinator.claim("task-b", 10, 30).expect("lease b");
        let mut stale = handoff("task-a", "worker-a", &lease_a, "app/Shared.java");
        stale.fence_token += 1;
        assert_eq!(
            coordinator.record_handoff(stale),
            Err(CoordinationError::LeaseFenced)
        );
        coordinator
            .record_handoff(handoff("task-a", "worker-a", &lease_a, "app/Shared.java"))
            .expect("handoff a");
        coordinator
            .record_handoff(handoff("task-b", "worker-b", &lease_b, "app/Shared.java"))
            .expect("handoff b");
        assert!(matches!(
            coordinator.reconcile(),
            Err(CoordinationError::Conflict(_))
        ));
    }
}
