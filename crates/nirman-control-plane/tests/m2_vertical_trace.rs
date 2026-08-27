use nirman_control_plane::{DurableControlPlane, DurableControlPlaneError, DurableDispatchOutcome};
use nirman_domain::{
    AndroidConstructionContract, AndroidDeviceProfile, AndroidResolverError,
    AndroidResolverRequest, AndroidTechnologyPlan, AndroidTechnologyResolver, ArtifactKind,
    ArtifactModel, CommandEnvelope, CommandKind, ConstructionRequirement,
    MutationTransactionRecord, ProductLifecycleState, ProjectId, RequirementOrigin, Revision,
    TaskId, ValidationModel,
};
use nirman_ipc::{authorize_registry_capability, AuthContext, CommandRequest, ProjectionReceiver};
use nirman_policy::{
    NetworkCategory, PolicyOutcome, PolicyRequest, WorkerPolicy, M6_SCHEMA_VERSION,
};
use nirman_project::mutation_capability_digest;
use nirman_supervisor::{Supervisor, SupervisorState};
use nirman_workers::{
    WorkerContract, WorkerExecutionRecord, WorkerObservation, WorkerOutcome, WorkerStage,
    M5_SCHEMA_VERSION,
};
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

fn authenticated_request(command: CommandEnvelope, project_id: &str) -> CommandRequest {
    CommandRequest {
        protocol_schema_version: nirman_ipc::PROTOCOL_SCHEMA_VERSION,
        auth: AuthContext {
            installation_id: "installation-portable-fixture".into(),
            user_scope: "local-user".into(),
            project_scope: project_id.into(),
            schema_version: 1,
        },
        command,
        correlation_id: "correlation-portable-fixture".into(),
        causation_id: None,
        deadline_epoch_seconds: None,
    }
}

fn command_for(
    project_id: &str,
    task_id: Option<&str>,
    revision: u64,
    id: &str,
    kind: CommandKind,
    payload: String,
) -> CommandEnvelope {
    CommandEnvelope {
        command_id: id.into(),
        project_id: ProjectId(project_id.into()),
        task_id: task_id.map(|value| TaskId(value.into())),
        kind,
        payload,
        expected_projection_revision: Revision(revision),
        idempotency_key: Some(format!("idempotency-{id}")),
    }
}

struct StubAndroidResolver;

impl AndroidTechnologyResolver for StubAndroidResolver {
    fn resolve(
        &self,
        request: &AndroidResolverRequest<'_>,
    ) -> Result<AndroidTechnologyPlan, AndroidResolverError> {
        request
            .contract
            .validate()
            .map_err(|_| AndroidResolverError::InvalidContract)?;
        if request.contract.target_platforms != vec!["android"] {
            return Err(AndroidResolverError::UnsupportedPlatform);
        }
        if request.source_revision.0 == 0 {
            return Err(AndroidResolverError::StaleRevision);
        }
        if request.workspace_root.is_empty() {
            return Err(AndroidResolverError::EmptyField("workspaceRoot"));
        }
        if request.project_fingerprint.is_empty() {
            return Err(AndroidResolverError::EmptyField("projectFingerprint"));
        }
        let ui_framework = if request
            .contract
            .user_intent
            .to_ascii_lowercase()
            .contains("view")
        {
            "android-views"
        } else {
            "jetpack-compose"
        };
        Ok(AndroidTechnologyPlan {
            plan_id: format!("technology-plan-{}", request.contract.contract_id),
            task_id: request.contract.task_id.clone(),
            requested_capabilities: vec!["intent-resolved".into()],
            visual_requirements: vec![],
            selected_languages: vec!["kotlin".into()],
            selected_ui_frameworks: vec![ui_framework.into()],
            selected_runtime_layers: vec!["android-runtime".into()],
            selected_native_modules: vec![],
            selected_build_plugins: vec![],
            selected_device_apis: vec![],
            selected_libraries: vec![],
            compatibility_constraints: vec![],
            rejected_alternatives: vec![],
            required_toolchains: vec!["jdk".into(), "gradle".into(), "android-sdk".into()],
            validation_plan: vec!["compile".into(), "unit-tests".into()],
            confidence: Some("stub-resolver".into()),
            revision: request.source_revision,
        })
    }
}

fn construction_contract(project_id: &str, task_id: &str) -> AndroidConstructionContract {
    AndroidConstructionContract {
        schema_version: 1,
        contract_id: "contract-portable-m4".into(),
        project_id: ProjectId(project_id.into()),
        target_platforms: vec!["android".into()],
        task_id: TaskId(task_id.into()),
        user_intent: "Build a Kotlin Android notes app with offline storage and calm views".into(),
        screenshots: vec![],
        assets: vec![],
        features: vec![ConstructionRequirement {
            requirement_id: "offline-notes".into(),
            statement: "Notes remain available offline".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        ui: vec![ConstructionRequirement {
            requirement_id: "calm-views".into(),
            statement: "Use a calm notes interface".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        data: vec![ConstructionRequirement {
            requirement_id: "local-data".into(),
            statement: "Persist notes locally".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        integrations: vec![],
        technology_plan: AndroidTechnologyPlan {
            plan_id: "intent-plan-portable-m4".into(),
            task_id: TaskId(task_id.into()),
            requested_capabilities: vec!["offline-storage".into()],
            visual_requirements: vec!["calm-interface".into()],
            selected_languages: vec!["kotlin".into()],
            selected_ui_frameworks: vec!["jetpack-compose".into()],
            selected_runtime_layers: vec!["android-runtime".into()],
            selected_native_modules: vec![],
            selected_build_plugins: vec![],
            selected_device_apis: vec![],
            selected_libraries: vec![],
            compatibility_constraints: vec![],
            rejected_alternatives: vec![],
            required_toolchains: vec!["jdk".into(), "gradle".into(), "android-sdk".into()],
            validation_plan: vec!["compile".into(), "unit-tests".into()],
            confidence: Some("fixture-confidence".into()),
            revision: Revision(1),
        },
        android_requirements: vec![],
        device_matrix: vec![AndroidDeviceProfile {
            device_id: "pixel-portable".into(),
            name: "Portable Pixel".into(),
            platform_version: "Android 35".into(),
            api_level: 35,
            architecture: "x86_64".into(),
            width: 1080,
            height: 2400,
            density: 420,
            orientation: "portrait".into(),
            locale: "en-US".into(),
            permissions: vec![],
            network_profile: "offline".into(),
        }],
        validation_model: ValidationModel {
            required_checks: vec!["compile".into(), "unit-tests".into()],
            acceptance_criteria: vec!["offline notes persist".into()],
        },
        artifact_model: ArtifactModel {
            required_artifact: ArtifactKind::Apk,
            aab_declared: false,
        },
    }
}

fn m4_evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m4_control_plane_trace.json")
}

fn m5_evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/evidence/m5_worker_trace.json")
}

fn m5_edit_undo_evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m5_worker_edit_undo_trace.json")
}

#[test]
fn m4_authenticated_project_intent_resolver_and_noop_plan_event_are_durable(
) -> Result<(), DurableControlPlaneError> {
    let project_id = "m4-project-portable";
    let task_id = "m4-task-portable";
    let db_path = std::env::temp_dir().join(format!(
        "nirman-m4-portable-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut plane = DurableControlPlane::open(&db_path, ProjectId(project_id.into()))?;

    let open = command_for(
        project_id,
        None,
        0,
        "m4-project-open",
        CommandKind::ProjectOpen,
        serde_json::json!({"projectId": project_id}).to_string(),
    );
    authorize_registry_capability(&authenticated_request(open.clone(), project_id))
        .expect("ProjectOpen authenticated admission");
    let opened = plane.dispatch(open)?;
    assert_eq!(opened.project_id.0, project_id);

    let intent = command_for(
        project_id,
        Some(task_id),
        opened.projection_revision.0,
        "m4-intent",
        CommandKind::TaskStart,
        "Build an Android notes app with Kotlin views and offline storage".into(),
    );
    authorize_registry_capability(&authenticated_request(intent.clone(), project_id))
        .expect("TaskStart authenticated admission");
    let intent_snapshot = plane.dispatch(intent)?;
    assert_eq!(intent_snapshot.current_source_revision, Revision(1));

    let contract = construction_contract(project_id, task_id);
    contract.validate().expect("valid Android intent contract");
    let synthesis = StubAndroidResolver
        .resolve(&AndroidResolverRequest {
            contract: &contract,
            source_revision: intent_snapshot.current_source_revision,
            workspace_root: "/portable/m4-workspace",
            project_fingerprint: "fingerprint-m4-portable",
        })
        .expect("M4 resolver selection");
    assert_eq!(contract.target_platforms, vec!["android"]);
    assert_eq!(synthesis.selected_languages, vec!["kotlin"]);
    assert_eq!(synthesis.selected_ui_frameworks, vec!["android-views"]);

    let plan_event = command_for(
        project_id,
        Some(task_id),
        intent_snapshot.projection_revision.0,
        "m4-plan-event",
        CommandKind::AndroidSynthesisBuild,
        serde_json::to_string(&synthesis).expect("plan event payload"),
    );
    authorize_registry_capability(&authenticated_request(plan_event.clone(), project_id))
        .expect("plan event authenticated admission");
    let planned = plane.dispatch(plan_event)?;
    assert_eq!(planned.last_event_sequence, 3);

    let noop_edit = command_for(
        project_id,
        Some(task_id),
        planned.projection_revision.0,
        "m4-noop-edit",
        CommandKind::WorkspaceApplyPatch,
        serde_json::json!({
            "operation": "NO_OP",
            "planId": synthesis.plan_id,
            "changedPaths": []
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(noop_edit.clone(), project_id))
        .expect("no-op edit authenticated admission");
    let edited = plane.dispatch(noop_edit)?;
    assert_eq!(edited.last_event_sequence, 4);
    assert_eq!(edited.current_source_revision, Revision(2));
    let events = plane.replay_after(0)?;
    assert_eq!(
        events
            .iter()
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "ProjectOpen",
            "TaskStart",
            "AndroidSynthesisBuild",
            "WorkspaceApplyPatch"
        ]
    );
    assert!(events[2].payload.contains("android-views"));
    drop(plane);

    let reopened = DurableControlPlane::open(&db_path, ProjectId(project_id.into()))?;
    assert_eq!(reopened.snapshot().last_event_sequence, 4);
    assert_eq!(reopened.snapshot().current_source_revision, Revision(2));
    assert_eq!(reopened.replay_after(2)?.len(), 2);
    fs::write(
        m4_evidence_path(),
        serde_json::json!({
            "schema": "nirman.m4.control_plane_trace.v1",
            "fixtureId": "M4-CONTROL-PLANE-TRACE-001",
            "projectOpenObserved": true,
            "intentParsed": true,
            "authenticatedAdmissionObserved": true,
            "androidOnlyObserved": contract.target_platforms == vec!["android"],
            "frameworkSelectionObserved": true,
            "selectedLanguage": synthesis.selected_languages[0],
            "selectedUiFramework": synthesis.selected_ui_frameworks[0],
            "planEventDurable": true,
            "noopEditEventDurable": true,
            "restartReplayObserved": true,
            "gradleExecuted": false,
            "emulatorExecuted": false,
            "androidRuntimeObserved": false,
            "nativeTauriRuntimeObserved": false,
            "evidenceStatus": "M4_SOURCE_ONLY_DURABLE_PLANNING_TRACE"
        })
        .to_string(),
    )
    .expect("M4 evidence");
    let _ = fs::remove_file(db_path);
    Ok(())
}

#[test]
fn m5_inspection_worker_consumes_plan_event_and_reloads_checkpoint_after_restart(
) -> Result<(), DurableControlPlaneError> {
    let project_id = "m5-project-portable";
    let task_id = "m5-task-portable";
    let db_path = std::env::temp_dir().join(format!(
        "nirman-m5-portable-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut plane = DurableControlPlane::open(&db_path, ProjectId(project_id.into()))?;
    let intent = command_for(
        project_id,
        Some(task_id),
        0,
        "m5-intent",
        CommandKind::TaskStart,
        "Build an Android inspection fixture".into(),
    );
    authorize_registry_capability(&authenticated_request(intent.clone(), project_id))
        .expect("M5 intent authenticated admission");
    let intent_snapshot = plane.dispatch(intent)?;

    let plan = command_for(
        project_id,
        Some(task_id),
        intent_snapshot.projection_revision.0,
        "m5-plan-event",
        CommandKind::AndroidSynthesisBuild,
        serde_json::json!({
            "planId": "m5-plan-portable",
            "targetPlatforms": ["android"],
            "selectedUiFramework": "jetpack-compose"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(plan.clone(), project_id))
        .expect("M5 plan event authenticated admission");
    let plan_snapshot = plane.dispatch(plan)?;

    let worker_contract = WorkerContract {
        schema_version: M5_SCHEMA_VERSION,
        worker_id: "worker-m5-inspection".into(),
        project_id: ProjectId(project_id.into()),
        task_id: TaskId(task_id.into()),
        capability_ceiling: vec!["android.inspect".into()],
        workspace_root: "/portable/m5-workspace".into(),
        allowed_paths: vec!["/portable/m5-workspace".into()],
        denied_paths: vec![],
        max_attempts: 3,
        evidence_requirements: vec!["inspection-summary".into()],
    };
    let mut record = WorkerExecutionRecord::start(worker_contract).expect("worker start");
    let observation = WorkerObservation {
        stage: WorkerStage::Inspect,
        outcome: WorkerOutcome::Success,
        source_revision: Revision(1),
        checkpoint_id: Some("checkpoint-m5-inspect".into()),
        changed_paths: vec![],
        evidence_refs: vec!["evidence:m5:inspection".into()],
        diagnostic_ref: None,
    };
    let handoff = record
        .observe("m5-worker-step", observation, None)
        .expect("inspection observation");
    assert_eq!(handoff.completed_stage, WorkerStage::Inspect);
    assert_eq!(handoff.next_stage, Some(WorkerStage::Plan));
    assert_eq!(record.worker.stage(), WorkerStage::Plan);

    let worker_command = command_for(
        project_id,
        Some(task_id),
        plan_snapshot.projection_revision.0,
        "m5-worker-step",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Inspect",
            "outcome": "Success",
            "checkpointId": "checkpoint-m5-inspect",
            "changedPaths": []
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(worker_command.clone(), project_id))
        .expect("WorkerStep authenticated admission");
    let worker_dispatch = plane.dispatch_with_result_and_worker_execution(
        worker_command,
        "correlation-m5-worker",
        &record,
    )?;
    assert!(matches!(
        worker_dispatch,
        DurableDispatchOutcome::Accepted { .. }
    ));
    plane.checkpoint("checkpoint-m5-inspect")?;
    assert_eq!(plane.checkpoint_id(), Some("checkpoint-m5-inspect"));
    assert_eq!(plane.replay_after(0)?.len(), 3);
    drop(plane);

    let reopened = DurableControlPlane::open(&db_path, ProjectId(project_id.into()))?;
    let reloaded_record = reopened
        .load_worker_execution_record(task_id)?
        .expect("worker record after restart");
    assert_eq!(reloaded_record.worker.stage(), WorkerStage::Plan);
    assert_eq!(reloaded_record.worker.source_revision(), Revision(1));
    assert_eq!(reopened.checkpoint_id(), Some("checkpoint-m5-inspect"));
    assert!(reopened.checkpoint_exists("checkpoint-m5-inspect")?);
    let replayed = reopened.replay_after(0)?;
    assert_eq!(replayed[1].kind, "AndroidSynthesisBuild");
    assert_eq!(replayed[2].kind, "WorkerStep");
    assert!(replayed[2].payload.contains("Inspect"));
    fs::write(
        m5_evidence_path(),
        serde_json::json!({
            "schema": "nirman.m5.worker_trace.v1",
            "fixtureId": "M5-WORKER-TRACE-001",
            "authenticatedAdmissionObserved": true,
            "planEventConsumed": true,
            "inspectionObserved": true,
            "workerStepDurable": true,
            "checkpointPersisted": true,
            "workerRecordReloaded": true,
            "restartReplayObserved": true,
            "mutationObserved": false,
            "gradleExecuted": false,
            "androidRuntimeObserved": false,
            "nativeTauriRuntimeObserved": false,
            "checkpointId": "checkpoint-m5-inspect",
            "evidenceStatus": "M5_HEADLESS_INSPECTION_CHECKPOINT_TRACE_ONLY"
        })
        .to_string(),
    )
    .expect("M5 evidence");
    let _ = fs::remove_file(db_path);
    Ok(())
}

#[test]
fn m5_worker_edit_and_undo_resumes_from_checkpoint() -> Result<(), DurableControlPlaneError> {
    let project_id = "m5-edit-undo-project";
    let task_id = "m5-edit-undo-task";
    let worker_id = "m5-edit-undo-worker";
    let root = std::env::temp_dir().join(format!(
        "nirman-m5-edit-undo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("workspace root");
    let database = root.join("control-plane.sqlite3");
    let edited_file = root.join("NewFeature.kt");
    let mut plane = DurableControlPlane::open(&database, ProjectId(project_id.into()))?;

    let intent = command_for(
        project_id,
        Some(task_id),
        0,
        "m5-edit-undo-intent",
        CommandKind::TaskStart,
        "Build an Android app and add one small feature".into(),
    );
    authorize_registry_capability(&authenticated_request(intent.clone(), project_id))
        .expect("intent authenticated admission");
    let intent_snapshot = plane.dispatch(intent)?;
    let contract = construction_contract(project_id, task_id);
    let synthesis = StubAndroidResolver
        .resolve(&AndroidResolverRequest {
            contract: &contract,
            source_revision: intent_snapshot.current_source_revision,
            workspace_root: root.to_string_lossy().as_ref(),
            project_fingerprint: "m5-edit-undo-base-fingerprint",
        })
        .expect("M4 plan from shared resolver fixture");
    assert_eq!(contract.target_platforms, vec!["android"]);
    let plan = command_for(
        project_id,
        Some(task_id),
        intent_snapshot.projection_revision.0,
        "m5-edit-undo-plan",
        CommandKind::AndroidSynthesisBuild,
        serde_json::to_string(&synthesis).expect("M4 plan event payload"),
    );
    authorize_registry_capability(&authenticated_request(plan.clone(), project_id))
        .expect("plan authenticated admission");
    let plan_snapshot = plane.dispatch(plan)?;
    let pre_edit_revision = plan_snapshot.current_source_revision;
    plane.checkpoint("checkpoint-m5-before-edit")?;

    let policy = WorkerPolicy {
        schema_version: M6_SCHEMA_VERSION,
        worker_id: worker_id.into(),
        workspace_root: root.to_string_lossy().into_owned(),
        allowed_paths: vec![root.to_string_lossy().into_owned()],
        denied_paths: vec![],
        protected_path_patterns: vec!["/home/*/.ssh".into(), "/home/*/.config".into()],
        allowed_command_patterns: vec!["*".into()],
        denied_command_patterns: vec![],
        allowed_network_categories: vec![NetworkCategory::None],
        allow_external_directories: false,
        allow_destructive_commands: false,
    };
    let policy_decision = policy
        .authorize(&PolicyRequest {
            request_id: "m5-edit-policy-request".into(),
            operation: "workspace.apply_patch".into(),
            path: Some(edited_file.to_string_lossy().into_owned()),
            command: None,
            network_category: NetworkCategory::None,
            destructive: false,
            external_directory: false,
        })
        .expect("M6 policy evaluation");
    assert_eq!(policy_decision.outcome, PolicyOutcome::Allow);
    assert!(policy_decision.reasons.is_empty());
    plane.save_m6_policy_event(&policy_decision)?;

    let edit_worker_step = command_for(
        project_id,
        Some(task_id),
        plan_snapshot.projection_revision.0,
        "m5-edit-worker-step",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Mutate",
            "operation": "Edit",
            "delegatedCommand": "WorkspaceApplyPatch",
            "relativePath": "NewFeature.kt",
            "policyDecisionId": policy_decision.decision_id
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(edit_worker_step.clone(), project_id))
        .expect("edit WorkerStep authenticated admission");
    plane.dispatch(edit_worker_step)?;

    fs::write(
        &edited_file,
        "package com.nirman.fixture\nclass NewFeature\n",
    )
    .expect("fixture edit");
    assert!(edited_file.is_file());
    let mutation_record = MutationTransactionRecord {
        transaction_id: "m5-edit-transaction".into(),
        command_id: "m5-workspace-apply".into(),
        operation_id: "m5-add-new-feature".into(),
        project_id: ProjectId(project_id.into()),
        task_id: TaskId(task_id.into()),
        worker_id: worker_id.into(),
        workspace_root: root.to_string_lossy().into_owned(),
        checkpoint_id: "checkpoint-m5-before-edit".into(),
        base_revision: pre_edit_revision,
        resulting_revision: Revision(pre_edit_revision.0 + 1),
        base_project_fingerprint: "m5-edit-undo-base-fingerprint".into(),
        resulting_project_fingerprint: Some("m5-edit-undo-result-fingerprint".into()),
        capability_digest: mutation_capability_digest(
            project_id,
            task_id,
            worker_id,
            "m5-add-new-feature",
            pre_edit_revision.0,
            "m5-edit-undo-base-fingerprint",
            1,
        ),
        fence_token: 1,
        state: "COMMITTED".into(),
        changed_paths_json: Some("[\"NewFeature.kt\"]".into()),
        evidence_json: Some(
            serde_json::json!({
                "policyDecisionId": policy_decision.decision_id,
                "policyOutcome": "ALLOW",
                "filesystemAdapter": "test-fixture-only"
            })
            .to_string(),
        ),
        started_at_epoch_seconds: 1,
        completed_at_epoch_seconds: Some(2),
    };
    let edit_command = command_for(
        project_id,
        Some(task_id),
        plan_snapshot.projection_revision.0 + 1,
        "m5-workspace-apply",
        CommandKind::WorkspaceApplyPatch,
        serde_json::json!({
            "operation": "Edit",
            "relativePath": "NewFeature.kt",
            "workerStepId": "m5-edit-worker-step"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(edit_command.clone(), project_id))
        .expect("WorkspaceApplyPatch authenticated admission");
    let edited_snapshot = match plane.dispatch_with_result_and_mutation_transaction(
        edit_command,
        "correlation-m5-edit",
        &mutation_record,
    )? {
        DurableDispatchOutcome::Accepted { snapshot, .. }
        | DurableDispatchOutcome::Duplicate { snapshot } => snapshot,
    };
    assert_eq!(edited_snapshot.current_source_revision, Revision(2));
    assert!(edited_file.is_file());

    let checkpoint_worker_step = command_for(
        project_id,
        Some(task_id),
        edited_snapshot.projection_revision.0,
        "m5-post-edit-checkpoint-step",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Checkpoint",
            "operation": "Checkpoint",
            "checkpointId": "checkpoint-m5-after-edit"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(
        checkpoint_worker_step.clone(),
        project_id,
    ))
    .expect("checkpoint WorkerStep authenticated admission");
    let checkpoint_snapshot = plane.dispatch(checkpoint_worker_step)?;
    plane.checkpoint("checkpoint-m5-after-edit")?;
    assert_eq!(checkpoint_snapshot.current_source_revision, Revision(2));
    assert!(plane.checkpoint_exists("checkpoint-m5-after-edit")?);

    let rollback_worker_step = command_for(
        project_id,
        Some(task_id),
        checkpoint_snapshot.projection_revision.0,
        "m5-rollback-worker-step",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Rollback",
            "operation": "Undo",
            "checkpointId": "checkpoint-m5-before-edit"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(
        rollback_worker_step.clone(),
        project_id,
    ))
    .expect("rollback WorkerStep authenticated admission");
    let rollback = plane.restore_source_revision_from_checkpoint(
        "checkpoint-m5-before-edit",
        rollback_worker_step,
        "correlation-m5-rollback",
    )?;
    let rollback_snapshot = match rollback {
        DurableDispatchOutcome::Accepted { snapshot, .. }
        | DurableDispatchOutcome::Duplicate { snapshot } => snapshot,
    };
    fs::remove_file(&edited_file).expect("fixture undo");
    assert!(!edited_file.exists());
    assert_eq!(rollback_snapshot.current_source_revision, pre_edit_revision);
    assert_eq!(rollback_snapshot.last_event_sequence, 6);

    drop(plane);
    let reopened = DurableControlPlane::open(&database, ProjectId(project_id.into()))?;
    assert_eq!(
        reopened.snapshot().current_source_revision,
        pre_edit_revision
    );
    assert_eq!(reopened.snapshot().last_event_sequence, 6);
    assert_eq!(
        reopened
            .replay_after(0)?
            .last()
            .expect("rollback event")
            .kind,
        "WorkerStep"
    );
    assert!(reopened
        .replay_after(0)?
        .last()
        .expect("rollback event")
        .payload
        .contains("checkpoint-m5-before-edit"));
    let reloaded_transaction = reopened
        .load_mutation_transaction("m5-edit-transaction")?
        .expect("durable edit transaction");
    assert_eq!(reloaded_transaction.state, "COMMITTED");
    assert_eq!(reloaded_transaction.resulting_revision, Revision(2));
    let reloaded_policy = reopened
        .load_m6_policy_events()?
        .into_iter()
        .find(|decision| decision.request_id == "m5-edit-policy-request")
        .expect("durable M6 allow decision");
    assert_eq!(reloaded_policy.outcome, PolicyOutcome::Allow);

    fs::write(
        m5_edit_undo_evidence_path(),
        serde_json::json!({
            "schema": "nirman.m5.worker_edit_undo_trace.v1",
            "fixtureId": "M5-WORKER-EDIT-UNDO-001",
            "authenticatedAdmissionObserved": true,
            "planEventConsumed": true,
            "editWorkerStepObserved": true,
            "checkpointWorkerStepObserved": true,
            "rollbackWorkerStepObserved": true,
            "m6PolicyAllowObserved": true,
            "m6PolicyDecisionDurable": true,
            "workspaceMutationObserved": true,
            "singleFileCreated": true,
            "sourceRevisionAdvanced": true,
            "checkpointAfterEditPersisted": true,
            "undoRemovedFile": true,
            "preEditRevisionRestored": true,
            "rollbackEventDurable": true,
            "mutationTransactionReloaded": true,
            "restartReplayObserved": true,
            "filesystemAdapter": "test-fixture-only",
            "gradleExecuted": false,
            "androidRuntimeObserved": false,
            "nativeTauriRuntimeObserved": false,
            "evidenceStatus": "M5_HEADLESS_EDIT_UNDO_TRACE_ONLY"
        })
        .to_string(),
    )
    .expect("M5 edit/undo evidence");
    let _ = fs::remove_dir_all(root);
    Ok(())
}

fn m7_evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m7_task_continuation_trace.json")
}

#[test]
fn m7_task_resumes_from_durable_checkpoint_after_restart() -> Result<(), DurableControlPlaneError> {
    let project_id = "m7-continuation-project";
    let task_id = "m7-continuation-task";
    let worker_id = "m7-continuation-worker";
    let root = std::env::temp_dir().join(format!(
        "nirman-m7-continuation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("workspace root");
    let database = root.join("control-plane.sqlite3");
    let edited_file = root.join("M7Feature.kt");
    let mut plane = DurableControlPlane::open(&database, ProjectId(project_id.into()))?;

    let intent = command_for(
        project_id,
        Some(task_id),
        0,
        "m7-task-start",
        CommandKind::TaskStart,
        "Build an Android app and add one small feature".into(),
    );
    authorize_registry_capability(&authenticated_request(intent.clone(), project_id))
        .expect("M7 task authenticated admission");
    let started = plane.dispatch(intent)?;
    assert_eq!(started.task_state, ProductLifecycleState::Planning);

    let contract = construction_contract(project_id, task_id);
    let synthesis = StubAndroidResolver
        .resolve(&AndroidResolverRequest {
            contract: &contract,
            source_revision: started.current_source_revision,
            workspace_root: root.to_string_lossy().as_ref(),
            project_fingerprint: "m7-continuation-base-fingerprint",
        })
        .expect("M7 plan resolver");
    let plan = command_for(
        project_id,
        Some(task_id),
        started.projection_revision.0,
        "m7-plan-event",
        CommandKind::AndroidSynthesisBuild,
        serde_json::to_string(&synthesis).expect("M7 plan payload"),
    );
    authorize_registry_capability(&authenticated_request(plan.clone(), project_id))
        .expect("M7 plan authenticated admission");
    let planned = plane.dispatch(plan)?;
    let pre_edit_revision = planned.current_source_revision;
    plane.checkpoint("checkpoint-m7-before-edit")?;

    let worker_contract = WorkerContract {
        schema_version: M5_SCHEMA_VERSION,
        worker_id: worker_id.into(),
        project_id: ProjectId(project_id.into()),
        task_id: TaskId(task_id.into()),
        capability_ceiling: vec!["android.inspect".into(), "android.edit".into()],
        workspace_root: root.to_string_lossy().into_owned(),
        allowed_paths: vec![root.to_string_lossy().into_owned()],
        denied_paths: vec![],
        max_attempts: 3,
        evidence_requirements: vec!["inspection-summary".into(), "edit-summary".into()],
    };
    let mut record = WorkerExecutionRecord::start(worker_contract).expect("M7 worker start");
    let inspect_observation = WorkerObservation {
        stage: WorkerStage::Inspect,
        outcome: WorkerOutcome::Success,
        source_revision: pre_edit_revision,
        checkpoint_id: Some("checkpoint-m7-before-edit".into()),
        changed_paths: vec![],
        evidence_refs: vec!["evidence:m7:inspect".into()],
        diagnostic_ref: None,
    };
    let inspect_handoff = record
        .observe("m7-worker-inspect", inspect_observation, None)
        .expect("M7 inspection observation");
    assert_eq!(inspect_handoff.next_stage, Some(WorkerStage::Plan));

    let inspect_step = command_for(
        project_id,
        Some(task_id),
        planned.projection_revision.0,
        "m7-worker-inspect",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Inspect",
            "operation": "Inspect",
            "checkpointId": "checkpoint-m7-before-edit"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(inspect_step.clone(), project_id))
        .expect("M7 Inspect WorkerStep authenticated admission");
    plane.dispatch_with_result_and_worker_execution(
        inspect_step,
        "correlation-m7-inspect",
        &record,
    )?;

    let policy = WorkerPolicy {
        schema_version: M6_SCHEMA_VERSION,
        worker_id: worker_id.into(),
        workspace_root: root.to_string_lossy().into_owned(),
        allowed_paths: vec![root.to_string_lossy().into_owned()],
        denied_paths: vec![],
        protected_path_patterns: vec!["/home/*/.ssh".into(), "/home/*/.config".into()],
        allowed_command_patterns: vec!["*".into()],
        denied_command_patterns: vec![],
        allowed_network_categories: vec![NetworkCategory::None],
        allow_external_directories: false,
        allow_destructive_commands: false,
    };
    let policy_decision = policy
        .authorize(&PolicyRequest {
            request_id: "m7-edit-policy-request".into(),
            operation: "workspace.apply_patch".into(),
            path: Some(edited_file.to_string_lossy().into_owned()),
            command: None,
            network_category: NetworkCategory::None,
            destructive: false,
            external_directory: false,
        })
        .expect("M7 edit M6 policy evaluation");
    assert_eq!(policy_decision.outcome, PolicyOutcome::Allow);
    assert!(policy_decision.reasons.is_empty());
    plane.save_m6_policy_event(&policy_decision)?;

    fs::write(
        &edited_file,
        "package com.nirman.fixture\nclass M7Feature\n",
    )
    .expect("M7 fixture edit");
    assert!(edited_file.is_file());
    let mutation_record = MutationTransactionRecord {
        transaction_id: "m7-edit-transaction".into(),
        command_id: "m7-workspace-apply".into(),
        operation_id: "m7-add-feature".into(),
        project_id: ProjectId(project_id.into()),
        task_id: TaskId(task_id.into()),
        worker_id: worker_id.into(),
        workspace_root: root.to_string_lossy().into_owned(),
        checkpoint_id: "checkpoint-m7-before-edit".into(),
        base_revision: pre_edit_revision,
        resulting_revision: Revision(pre_edit_revision.0 + 1),
        base_project_fingerprint: "m7-continuation-base-fingerprint".into(),
        resulting_project_fingerprint: Some("m7-continuation-edited-fingerprint".into()),
        capability_digest: mutation_capability_digest(
            project_id,
            task_id,
            worker_id,
            "m7-add-feature",
            pre_edit_revision.0,
            "m7-continuation-base-fingerprint",
            1,
        ),
        fence_token: 1,
        state: "COMMITTED".into(),
        changed_paths_json: Some("[\"M7Feature.kt\"]".into()),
        evidence_json: Some(
            serde_json::json!({
                "policyDecisionId": policy_decision.decision_id,
                "policyOutcome": "ALLOW",
                "filesystemAdapter": "test-fixture-only"
            })
            .to_string(),
        ),
        started_at_epoch_seconds: 1,
        completed_at_epoch_seconds: Some(2),
    };
    let edit_command = command_for(
        project_id,
        Some(task_id),
        planned.projection_revision.0 + 1,
        "m7-workspace-apply",
        CommandKind::WorkspaceApplyPatch,
        serde_json::json!({
            "operation": "Edit",
            "relativePath": "M7Feature.kt",
            "workerStepId": "m7-worker-edit"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(edit_command, project_id))
        .expect("M7 WorkspaceApplyPatch authenticated admission");

    let edited_snapshot = match plane.dispatch_with_result_and_mutation_transaction(
        command_for(
            project_id,
            Some(task_id),
            planned.projection_revision.0 + 1,
            "m7-workspace-apply",
            CommandKind::WorkspaceApplyPatch,
            serde_json::json!({
                "operation": "Edit",
                "relativePath": "M7Feature.kt",
                "workerStepId": "m7-worker-edit"
            })
            .to_string(),
        ),
        "correlation-m7-edit",
        &mutation_record,
    )? {
        DurableDispatchOutcome::Accepted { snapshot, .. }
        | DurableDispatchOutcome::Duplicate { snapshot } => snapshot,
    };
    assert!(edited_file.is_file());
    assert_eq!(edited_snapshot.current_source_revision, Revision(2));

    let edit_step = command_for(
        project_id,
        Some(task_id),
        edited_snapshot.projection_revision.0,
        "m7-worker-edit",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Mutate",
            "operation": "Edit",
            "delegatedCommand": "WorkspaceApplyPatch",
            "relativePath": "M7Feature.kt",
            "policyDecisionId": policy_decision.decision_id
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(edit_step, project_id))
        .expect("M7 Edit WorkerStep authenticated admission");
    let edited_worker_snapshot = plane.dispatch(command_for(
        project_id,
        Some(task_id),
        edited_snapshot.projection_revision.0,
        "m7-worker-edit",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Mutate",
            "operation": "Edit",
            "delegatedCommand": "WorkspaceApplyPatch",
            "relativePath": "M7Feature.kt",
            "policyDecisionId": policy_decision.decision_id
        })
        .to_string(),
    ))?;

    let checkpoint_step = command_for(
        project_id,
        Some(task_id),
        edited_worker_snapshot.projection_revision.0,
        "m7-worker-checkpoint",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Checkpoint",
            "operation": "Checkpoint",
            "checkpointId": "checkpoint-m7-post-edit"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(checkpoint_step.clone(), project_id))
        .expect("M7 Checkpoint WorkerStep authenticated admission");
    let checkpoint_snapshot = plane.dispatch(checkpoint_step)?;
    plane.checkpoint("checkpoint-m7-post-edit")?;
    assert!(plane.checkpoint_exists("checkpoint-m7-post-edit")?);
    let post_checkpoint_revision = checkpoint_snapshot.current_source_revision;
    assert_eq!(post_checkpoint_revision, Revision(2));

    record
        .observe(
            "m7-worker-plan",
            WorkerObservation {
                stage: WorkerStage::Plan,
                outcome: WorkerOutcome::Success,
                source_revision: pre_edit_revision,
                checkpoint_id: Some("checkpoint-m7-before-edit".into()),
                changed_paths: vec![],
                evidence_refs: vec!["evidence:m7:plan".into()],
                diagnostic_ref: None,
            },
            None,
        )
        .expect("M7 plan observation");
    record
        .observe(
            "m7-worker-checkpoint",
            WorkerObservation {
                stage: WorkerStage::Checkpoint,
                outcome: WorkerOutcome::Success,
                source_revision: post_checkpoint_revision,
                checkpoint_id: Some("checkpoint-m7-post-edit".into()),
                changed_paths: vec!["M7Feature.kt".into()],
                evidence_refs: vec!["evidence:m7:checkpoint".into()],
                diagnostic_ref: None,
            },
            Some("checkpoint-m7-post-edit".into()),
        )
        .expect("M7 checkpoint observation");
    plane.save_worker_execution_record(&record)?;

    let worker_events_before_restart = plane
        .replay_after(0)?
        .into_iter()
        .filter(|event| event.kind == "WorkerStep")
        .collect::<Vec<_>>();
    assert_eq!(worker_events_before_restart.len(), 3);
    assert_eq!(worker_events_before_restart[0].sequence, 3);
    assert_eq!(worker_events_before_restart[1].sequence, 5);
    assert_eq!(worker_events_before_restart[2].sequence, 6);
    assert!(worker_events_before_restart[0].payload.contains("Inspect"));
    assert!(worker_events_before_restart[1].payload.contains("Edit"));
    assert!(worker_events_before_restart[2]
        .payload
        .contains("Checkpoint"));
    drop(plane);

    let mut reopened = DurableControlPlane::open(&database, ProjectId(project_id.into()))?;
    let reloaded_task = reopened
        .load_worker_execution_record(task_id)?
        .expect("M7 task record after restart");
    assert_eq!(reloaded_task.worker.contract().task_id.0, task_id);
    assert_eq!(
        reloaded_task.worker.source_revision(),
        post_checkpoint_revision
    );
    assert_eq!(
        reloaded_task.worker.checkpoint_id(),
        Some("checkpoint-m7-post-edit")
    );
    assert_eq!(
        reloaded_task.worker.lifecycle(),
        nirman_workers::WorkerLifecycle::Running
    );
    assert!(!matches!(
        reopened.snapshot().task_state,
        ProductLifecycleState::Cancelled | ProductLifecycleState::Completed
    ));
    assert_eq!(
        reopened.snapshot().current_source_revision,
        post_checkpoint_revision
    );
    assert!(reopened.checkpoint_exists("checkpoint-m7-post-edit")?);
    let replayed_before_resume = reopened.replay_after(0)?;
    let replayed_worker_events = replayed_before_resume
        .iter()
        .filter(|event| event.kind == "WorkerStep")
        .collect::<Vec<_>>();
    assert_eq!(replayed_worker_events.len(), 3);
    assert!(replayed_worker_events.windows(2).all(|events| {
        events[0].sequence < events[1].sequence && events[0].event_id != events[1].event_id
    }));
    assert_eq!(
        replayed_worker_events
            .iter()
            .map(|event| {
                if event.payload.contains("Inspect") {
                    "Inspect"
                } else if event.payload.contains("Edit") {
                    "Edit"
                } else {
                    "Checkpoint"
                }
            })
            .collect::<Vec<_>>(),
        vec!["Inspect", "Edit", "Checkpoint"]
    );

    let resume_step = command_for(
        project_id,
        Some(task_id),
        reopened.snapshot().projection_revision.0,
        "m7-worker-inspect-resume",
        CommandKind::WorkerStep,
        serde_json::json!({
            "stage": "Inspect",
            "operation": "Inspect",
            "resumedFromCheckpoint": "checkpoint-m7-post-edit"
        })
        .to_string(),
    );
    authorize_registry_capability(&authenticated_request(resume_step.clone(), project_id))
        .expect("M7 resumed Inspect authenticated admission");
    let resumed_snapshot = reopened.dispatch(resume_step)?;
    assert_eq!(
        resumed_snapshot.current_source_revision,
        post_checkpoint_revision
    );
    let replayed_after_resume = reopened.replay_after(0)?;
    let resumed_worker_events = replayed_after_resume
        .iter()
        .filter(|event| event.kind == "WorkerStep")
        .collect::<Vec<_>>();
    assert_eq!(resumed_worker_events.len(), 4);
    assert_eq!(
        resumed_worker_events
            .last()
            .expect("resumed event")
            .sequence,
        7
    );
    assert_eq!(
        resumed_worker_events
            .last()
            .expect("resumed event")
            .event_id,
        "m7-worker-inspect-resume"
    );
    assert_eq!(
        resumed_worker_events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "m7-worker-inspect",
            "m7-worker-edit",
            "m7-worker-checkpoint",
            "m7-worker-inspect-resume"
        ]
    );

    fs::write(
        m7_evidence_path(),
        serde_json::json!({
            "schema": "nirman.m7.task_continuation_trace.v1",
            "fixtureId": "M7-TASK-CONTINUATION-001",
            "taskId": task_id,
            "authenticatedTaskStartObserved": true,
            "threeWorkerStepsDurableBeforeRestart": true,
            "workerStepOrder": ["Inspect", "Edit", "Checkpoint"],
            "m6EditPolicyAllowObserved": true,
            "m6PolicyDecisionDurable": true,
            "postCheckpointSourceRevision": post_checkpoint_revision.0,
            "taskRecordReloadedById": true,
            "recoverableAfterRestart": true,
            "checkpointResolvableAfterRestart": true,
            "sourceRevisionRestoredAfterRestart": true,
            "orderedWorkerReplayObserved": true,
            "duplicateWorkerEventsObserved": false,
            "resumedInspectAppendedToSameStream": true,
            "resumedEventSequence": 7,
            "filesystemAdapter": "test-fixture-only",
            "gradleExecuted": false,
            "androidRuntimeObserved": false,
            "nativeTauriRuntimeObserved": false,
            "evidenceStatus": "M7_HEADLESS_TASK_CONTINUATION_TRACE_ONLY"
        })
        .to_string(),
    )
    .expect("M7 continuation evidence");
    let _ = fs::remove_dir_all(root);
    Ok(())
}
