use nirman_control_plane::{DurableControlPlane, DurableControlPlaneError};
use nirman_domain::{
    CommandEnvelope, CommandKind, ProductLifecycleState, ProjectId, Revision, TaskId,
};
use nirman_ipc::ProjectionReceiver;
use nirman_supervisor::{Supervisor, SupervisorState};
use std::fs;
use std::path::PathBuf;

fn command(revision: u64, id: &str, kind: CommandKind, payload: &str) -> CommandEnvelope {
    CommandEnvelope {
        command_id: id.into(),
        project_id: ProjectId("m2-project-0001".into()),
        task_id: Some(TaskId("m2-task-0001".into())),
        kind,
        payload: payload.into(),
        expected_projection_revision: Revision(revision),
        idempotency_key: Some(id.into()),
    }
}

fn evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/evidence/m2_vertical_trace.json")
}

#[test]
fn m2_vertical_trace_persists_replays_recovers_and_rejects_stale_updates(
) -> Result<(), DurableControlPlaneError> {
    let db_path = std::env::temp_dir().join(format!(
        "nirman-m2-vertical-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let project = ProjectId("m2-project-0001".into());

    let mut supervisor = Supervisor::start();
    supervisor.heartbeat();
    supervisor.register_lease("lease-old", "worker-old", 1);

    let mut first = DurableControlPlane::open(&db_path, project.clone())?;
    let created = first.dispatch(command(
        0,
        "m2-event-1",
        CommandKind::SubmitInstruction,
        "Build an Android notes app",
    ))?;
    assert_eq!(created.task_state, ProductLifecycleState::Planning);
    first.checkpoint("checkpoint-before-pause")?;

    let paused = first.dispatch(command(1, "m2-event-2", CommandKind::PauseTask, ""))?;
    assert_eq!(paused.task_state, ProductLifecycleState::Paused);
    first.checkpoint("checkpoint-paused")?;
    drop(first);

    let mut after_restart = DurableControlPlane::open(&db_path, project.clone())?;
    assert_eq!(
        after_restart.snapshot().task_state,
        ProductLifecycleState::Paused
    );
    assert_eq!(after_restart.snapshot().project_id, project);
    assert_eq!(after_restart.checkpoint_id(), Some("checkpoint-paused"));

    supervisor.restart_from_checkpoint("checkpoint-paused");
    assert!(supervisor.snapshot().active_fence.is_none());
    assert_eq!(supervisor.snapshot().state, SupervisorState::Reconciling);
    supervisor.register_lease("lease-new", "worker-new", 2);
    supervisor.heartbeat();
    assert_eq!(supervisor.snapshot().state, SupervisorState::Running);
    assert_eq!(
        supervisor
            .snapshot()
            .active_fence
            .as_ref()
            .map(|f| f.fence_token),
        Some(2)
    );

    let resumed = after_restart.dispatch(command(2, "m2-event-3", CommandKind::ResumeTask, ""))?;
    assert_eq!(resumed.task_state, ProductLifecycleState::Planning);
    after_restart.request_worker_cancel();
    assert!(after_restart.worker_cancel_requested());
    let cancelled =
        after_restart.dispatch(command(3, "m2-event-4", CommandKind::CancelTask, ""))?;
    assert_eq!(cancelled.task_state, ProductLifecycleState::Cancelled);

    let replayed = after_restart.replay_after(0)?;
    assert_eq!(replayed.len(), 4);
    assert!(replayed
        .windows(2)
        .all(|events| events[0].sequence < events[1].sequence));
    assert!(replayed
        .iter()
        .all(|event| event.task_id == Some(TaskId("m2-task-0001".into()))));

    let mut ui_projection = ProjectionReceiver::default();
    assert!(ui_projection.observe_snapshot(created.clone()));
    assert!(!ui_projection.observe_snapshot(created.clone()));
    assert!(ui_projection.observe_event(&replayed[1]));
    assert!(!ui_projection.observe_event(&replayed[1]));
    assert!(!ui_projection.observe_event(&replayed[3].clone()));
    assert_eq!(ui_projection.rejected_events(), 2);

    let path = evidence_path();
    fs::create_dir_all(path.parent().expect("evidence directory")).expect("evidence directory");
    fs::write(
        path,
        format!(
            "{{\n  \"fixtureId\": \"M2-VERTICAL-TRACE-001\",\n  \"taskIdentity\": \"m2-task-0001\",\n  \"projectIdentity\": \"m2-project-0001\",\n  \"eventSequences\": [1, 2, 3, 4],\n  \"replayedEventCount\": {},\n  \"pausePersisted\": true,\n  \"resumePersisted\": true,\n  \"cancellationReachedControlPlane\": true,\n  \"workerCancellationRequested\": true,\n  \"checkpointReloaded\": \"checkpoint-paused\",\n  \"staleLeaseFenced\": true,\n  \"restartRecoveryState\": \"RECONCILING\",\n  \"recoveredFinalState\": \"CANCELLED\",\n  \"duplicateAndOutOfOrderEventsRejected\": true,\n  \"typedProjectionBoundaryObserved\": true,\n  \"productionReactUiRuntime\": false,\n  \"androidRuntime\": false,\n  \"evidenceStatus\": \"M2_FOUNDATION_TRACE_ONLY\"\n}}\n",
            replayed.len()
        ),
    )
    .expect("machine-readable evidence");

    let _ = fs::remove_file(db_path);
    Ok(())
}
