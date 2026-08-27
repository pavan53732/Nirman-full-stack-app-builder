use nirman_control_plane::DurableControlPlane;
use nirman_domain::{
    CommandEnvelope, CommandKind, ProductLifecycleState, ProjectId, Revision, TaskId,
};
use nirman_supervisor::{
    detect_stale_worker, recover_interrupted_run, BackgroundRunRecord, BackgroundRunState,
    RecoveryAction, Supervisor, M7_SCHEMA_VERSION,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn database_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nirman-m7-acceptance-{nonce}.sqlite3"))
}

fn command(revision: u64, id: &str, kind: CommandKind, payload: &str) -> CommandEnvelope {
    CommandEnvelope {
        command_id: id.into(),
        project_id: ProjectId("project-0001".into()),
        task_id: Some(TaskId("task-m7".into())),
        kind,
        payload: payload.into(),
        expected_projection_revision: Revision(revision),
        idempotency_key: Some(format!("idem-{id}")),
    }
}

fn run_record() -> BackgroundRunRecord {
    BackgroundRunRecord {
        schema_version: M7_SCHEMA_VERSION,
        run_id: "run-m7".into(),
        project_id: "project-0001".into(),
        task_id: "task-m7".into(),
        worker_id: "worker-single".into(),
        checkpoint_id: Some("checkpoint-m7".into()),
        state: BackgroundRunState::Running,
        last_heartbeat_epoch_seconds: 10,
        attempt: 1,
        recovery_action: None,
        failure_fingerprint: None,
        notification_kind: None,
    }
}

#[test]
fn m7_ui_disconnect_restart_and_event_replay_preserve_durable_lifecycle() {
    let database = database_path();
    let mut plane =
        DurableControlPlane::open(&database, ProjectId("project-0001".into())).expect("open");

    let accepted = plane
        .dispatch_with_result(
            command(
                0,
                "m7-start",
                CommandKind::TaskStart,
                "Build an Android notes app",
            ),
            "corr-m7-start",
        )
        .expect("task start");
    let start_snapshot = match &accepted {
        nirman_control_plane::DurableDispatchOutcome::Accepted { snapshot, .. }
        | nirman_control_plane::DurableDispatchOutcome::Duplicate { snapshot } => snapshot.clone(),
    };
    assert_eq!(start_snapshot.task_state, ProductLifecycleState::Planning);
    plane.save_background_run(&run_record()).expect("save run");
    drop(plane);

    let mut reopened =
        DurableControlPlane::open(&database, ProjectId("project-0001".into())).expect("reopen");

    let replayed = reopened.replay_after(0).expect("replay");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].event_id, "m7-start");
    assert_eq!(
        reopened.load_background_run("run-m7").expect("load run"),
        Some(run_record())
    );

    let paused = reopened
        .dispatch_with_result(
            command(1, "m7-pause", CommandKind::PauseTask, "pause"),
            "corr-m7-pause",
        )
        .expect("pause");
    let paused_snapshot = match &paused {
        nirman_control_plane::DurableDispatchOutcome::Accepted { snapshot, .. }
        | nirman_control_plane::DurableDispatchOutcome::Duplicate { snapshot } => snapshot.clone(),
    };
    assert_eq!(paused_snapshot.task_state, ProductLifecycleState::Paused);
    let _ = std::fs::remove_file(database);
}

#[test]
fn m7_supervisor_restart_requires_reconciliation_and_new_heartbeat() {
    let mut supervisor = Supervisor::start();
    supervisor.register_lease("lease-m7", "worker-single", 1);
    supervisor.restart_from_checkpoint("checkpoint-m7");
    assert_eq!(
        supervisor.snapshot().state,
        nirman_supervisor::SupervisorState::Reconciling
    );
    assert_eq!(supervisor.snapshot().active_fence, None);
    supervisor.heartbeat();
    assert_eq!(
        supervisor.snapshot().state,
        nirman_supervisor::SupervisorState::Running
    );
}

#[test]
fn m7_stale_worker_checkpoint_recovery_is_deterministic() {
    let mut run = run_record();
    assert!(detect_stale_worker(&run, 20, 5).expect("stale"));
    assert_eq!(
        recover_interrupted_run(&mut run, 20, 5).expect("recover"),
        RecoveryAction::ResumeFromCheckpoint
    );
    assert_eq!(run.state, BackgroundRunState::Recovering);
    assert_eq!(run.attempt, 2);
}
