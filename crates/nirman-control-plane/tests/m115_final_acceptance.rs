use nirman_control_plane::{DurableControlPlane, DurableTimeoutOutcome};
use nirman_domain::{CommandEnvelope, CommandKind, ProductLifecycleState, ProjectId, Revision};
use nirman_ipc::{EventBatch, SubscriptionStatus};
use nirman_supervisor::{Supervisor, SupervisorState};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn database_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nirman-m115-final-{nonce}.sqlite3"))
}

fn instruction(revision: u64, id: &str) -> CommandEnvelope {
    CommandEnvelope {
        command_id: id.into(),
        project_id: ProjectId("m115-project".into()),
        task_id: None,
        kind: CommandKind::SubmitInstruction,
        payload: "Build an Android notes app".into(),
        expected_projection_revision: Revision(revision),
        idempotency_key: Some(id.into()),
    }
}

#[test]
fn m115_final_acceptance_observes_restart_gap_recovery_and_timeout(
) -> Result<(), Box<dyn std::error::Error>> {
    let database = database_path();
    let project = ProjectId("m115-project".into());
    let mut supervisor = Supervisor::start();
    supervisor.heartbeat();
    supervisor.register_lease("lease-old", "worker-old", 1);

    let mut first = DurableControlPlane::open(&database, project.clone())?;
    let accepted = first
        .dispatch(instruction(0, "command-1"))
        .map_err(|error| format!("{error:?}"))?;
    first.checkpoint("checkpoint-1")?;
    first.set_retention_floor(2)?;
    let timeout = first
        .record_timeout("timeout-1", "command-late", "corr", "deadline elapsed")
        .map_err(|error| format!("{error:?}"))?;
    assert!(matches!(timeout, DurableTimeoutOutcome::Recorded { .. }));
    drop(first);

    let reopened = DurableControlPlane::open(&database, project.clone())?;
    assert_eq!(reopened.checkpoint_id(), Some("checkpoint-1"));
    assert_eq!(
        reopened.snapshot().task_state,
        ProductLifecycleState::SafelyFailed
    );
    supervisor.restart_from_checkpoint("checkpoint-1");
    let old_fence_cleared = supervisor.snapshot().active_fence.is_none();
    let reconciliation_observed = supervisor.snapshot().state == SupervisorState::Reconciling;
    supervisor.register_lease("lease-new", "worker-new", 2);
    supervisor.heartbeat();
    let restart_recovered = supervisor.snapshot().state == SupervisorState::Running
        && supervisor
            .snapshot()
            .active_fence
            .as_ref()
            .map(|f| f.fence_token)
            == Some(2);

    let (_retained_events, replay_gap) = reopened.replay_after_with_gap(0)?;
    let gap_batch = EventBatch {
        subscription_id: "sub-gap".into(),
        projection_revision: reopened.snapshot().projection_revision,
        from_event_sequence: 0,
        next_event_sequence: 0,
        events: Vec::new(),
        has_gap: replay_gap,
        status: SubscriptionStatus::Gap,
    };
    let retention_gap_observed = gap_batch.has_gap && gap_batch.status == SubscriptionStatus::Gap;
    let recovery_cursor = reopened.snapshot().last_event_sequence;
    let recovered_replay = reopened.replay_after(recovery_cursor)?;
    let snapshot_cursor_recovered = recovered_replay.is_empty() && recovery_cursor > 0;

    let timeout_restarted = reopened
        .replay_after(0)?
        .iter()
        .any(|event| event.kind == "CommandTimedOut");

    let evidence = json!({
        "schema": "nirman.m115.final_acceptance.v1",
        "checkpointReloaded": reopened.checkpoint_id() == Some("checkpoint-1"),
        "oldLeaseFenced": old_fence_cleared,
        "reconciliationObserved": reconciliation_observed,
        "newLeaseHeartbeatRecovered": restart_recovered,
        "retentionGapObserved": retention_gap_observed,
        "snapshotCursorRecoveryObserved": snapshot_cursor_recovered,
        "durableTimeoutAfterRestart": timeout_restarted,
        "acceptedProjectionRevision": accepted.projection_revision.0,
        "tauriCommandRuntime": false,
        "reactDomRuntime": false,
        "androidRuntime": false,
        "apkExport": false,
        "evidenceStatus": "M115_HEADLESS_DURABLE_BOUNDARY_TRACE_ONLY"
    });
    let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m115_final_acceptance.json");
    fs::create_dir_all(evidence_path.parent().expect("evidence directory"))?;
    fs::write(evidence_path, serde_json::to_vec_pretty(&evidence)?)?;

    assert!(old_fence_cleared && reconciliation_observed && restart_recovered);
    assert!(retention_gap_observed && snapshot_cursor_recovered && timeout_restarted);
    let _ = fs::remove_file(database);
    Ok(())
}
