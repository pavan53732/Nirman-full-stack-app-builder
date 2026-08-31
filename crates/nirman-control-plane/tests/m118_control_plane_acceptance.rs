//! M118 control-plane acceptance (TA §84.3, BS §79.11/§79.12, ADR-206):
//! the dispatch-time platform gate is durable. The environment capability
//! record, gate records, and blocked decisions persist across restart,
//! replay idempotently under an unchanged environment, and blocked work is
//! never scheduled — with the truthful state and both §79.11 lists on the
//! durable node.

use nirman_control_plane::DurableControlPlane;
use nirman_domain::{BuildGateStage, PlatformCapabilityState, PlatformRequirements, ProjectId};
use nirman_tools::{
    evaluate_platform_admission, gate_evidence_is_stale, operation_for_stage,
    EnvironmentCapabilityPlanner, PlatformAdmissionDecision, PlatformCapabilityRegistry,
    PlatformProbe, ToolObservation,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone)]
struct FakeProbe {
    host: String,
    arch: String,
    tools: BTreeMap<String, ToolObservation>,
}

fn linux_cross_capable() -> FakeProbe {
    let mut tools = BTreeMap::new();
    for (name, version) in [
        ("rustc", "rustc 1.98.0"),
        ("cargo", "cargo 1.98.0"),
        ("node", "v22.23.2"),
        ("tsc", "Version 5.9.0"),
        ("file", "file-5.46"),
        ("package_manager", "9.15.9"),
        ("rust_target_windows", "x86_64-pc-windows-gnu"),
        ("windows_linker", "x86_64-w64-mingw32-gcc (GCC) 14.2.0"),
        ("java", "openjdk version 21"),
    ] {
        tools.insert(name.into(), ToolObservation::installed(version, "scripted"));
    }
    for name in [
        "nsis",
        "wine",
        "gradle",
        "android_sdk",
        "emulator",
        "adb",
        "device",
    ] {
        tools.insert(name.into(), ToolObservation::missing("scripted missing"));
    }
    FakeProbe {
        host: "linux".into(),
        arch: "x86_64".into(),
        tools,
    }
}

impl PlatformProbe for FakeProbe {
    fn host_platform(&self) -> String {
        self.host.clone()
    }
    fn host_architecture(&self) -> String {
        self.arch.clone()
    }
    fn observe_tool(&self, name: &str) -> ToolObservation {
        self.tools
            .get(name)
            .cloned()
            .unwrap_or_else(|| ToolObservation::missing("not scripted"))
    }
    fn fingerprint_env(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

fn cross_requirements() -> PlatformRequirements {
    PlatformRequirements {
        required_target_platforms: vec!["windows".into()],
        required_capabilities: vec!["cross_build_windows".into()],
        cross_compilation_allowed: true,
        ..Default::default()
    }
}

fn native_requirements() -> PlatformRequirements {
    PlatformRequirements {
        required_host_platforms: vec!["windows".into()],
        required_target_platforms: vec!["windows".into()],
        native_execution_required: true,
        ..Default::default()
    }
}

fn temp_path(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("nirman-m118-cp-{}-{:?}", name, std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join(format!("{name}.sqlite"))
}

#[test]
fn preflight_and_admission_are_durable_across_restart() {
    let path = temp_path("restart");
    let project_id = ProjectId("project-m118-durable".into());
    let plane = DurableControlPlane::open(&path, project_id.clone()).expect("open");
    let outcome = plane
        .run_platform_preflight_and_admit(
            "task-durable",
            "windows",
            "x86_64",
            &cross_requirements(),
            BuildGateStage::TargetBuild,
            Box::new(linux_cross_capable()),
            1_700_000_100,
        )
        .expect("admission run");
    assert!(
        matches!(outcome, PlatformAdmissionDecision::Admitted),
        "cross-capable environment admits the target build"
    );
    let record = plane
        .load_platform_preflight("task-durable")
        .expect("load")
        .expect("record stored");
    let gate_records = plane.load_platform_gate_records().expect("load gates");
    assert_eq!(gate_records.len(), 1);
    assert_eq!(gate_records[0].stage, BuildGateStage::TargetBuild);
    assert_eq!(
        gate_records[0].result,
        nirman_domain::BuildGateResult::Unverified
    );
    drop(plane);

    // Restart: the durable plane reloads the same environment identity.
    let plane = DurableControlPlane::open(&path, project_id).expect("reopen");
    let reloaded = plane
        .load_platform_preflight("task-durable")
        .expect("load")
        .expect("record survives restart");
    assert_eq!(
        reloaded.environment_fingerprint,
        record.environment_fingerprint
    );
    assert_eq!(reloaded.environment_id, record.environment_id);
    assert_eq!(plane.load_platform_gate_records().expect("gates").len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn replay_under_unchanged_environment_is_idempotent() {
    let path = temp_path("idempotent");
    let project_id = ProjectId("project-m118-idempotent".into());
    let plane = DurableControlPlane::open(&path, project_id).expect("open");
    let first = plane
        .run_platform_preflight_and_admit(
            "task-idem",
            "windows",
            "x86_64",
            &cross_requirements(),
            BuildGateStage::TargetBuild,
            Box::new(linux_cross_capable()),
            1_700_000_200,
        )
        .expect("first run");
    let second = plane
        .run_platform_preflight_and_admit(
            "task-idem",
            "windows",
            "x86_64",
            &cross_requirements(),
            BuildGateStage::TargetBuild,
            Box::new(linux_cross_capable()),
            1_700_000_300,
        )
        .expect("second run");
    assert_eq!(
        first, second,
        "unchanged environment => identical admission"
    );
    let record = plane
        .load_platform_preflight("task-idem")
        .expect("load")
        .expect("record");
    assert!(
        record.supersedes.is_none(),
        "unchanged environment must not create a supersedes lineage"
    );
    assert_eq!(
        plane.load_platform_gate_records().expect("gates").len(),
        1,
        "gate records upsert, they do not accumulate"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn blocked_node_records_truthful_state_and_both_lists() {
    let path = temp_path("blocked");
    let project_id = ProjectId("project-m118-blocked".into());
    let plane = DurableControlPlane::open(&path, project_id.clone()).expect("open");
    let outcome = plane
        .run_platform_preflight_and_admit(
            "task-blocked",
            "windows",
            "x86_64",
            &native_requirements(),
            BuildGateStage::RuntimeValidation,
            Box::new(linux_cross_capable()),
            1_700_000_400,
        )
        .expect("admission run");
    let decision = match outcome {
        PlatformAdmissionDecision::Blocked { decisions } => decisions,
        PlatformAdmissionDecision::Admitted => {
            panic!("native validation must not be admitted from a non-matching host")
        }
    };
    assert_eq!(decision.len(), 1);
    assert_eq!(decision[0].state, PlatformCapabilityState::Unavailable);
    assert!(
        decision[0]
            .can_continue
            .iter()
            .any(|l| l.contains("cross-build")),
        "the can-continue list must name the independent work: {:?}",
        decision[0].can_continue
    );
    assert!(
        decision[0]
            .cannot_continue
            .iter()
            .any(|l| l.contains("WINDOWS")),
        "the cannot-continue list must name the blocked certification: {:?}",
        decision[0].cannot_continue
    );
    assert!(
        decision[0].reason.contains("host linux") && decision[0].reason.contains("target windows"),
        "the reason must name the host/target mismatch: {}",
        decision[0].reason
    );

    // Durable: the blocked node and its gate record survive restart.
    drop(plane);
    let plane = DurableControlPlane::open(&path, project_id).expect("reopen");
    let reloaded = plane
        .load_platform_blocked_decisions("task-blocked")
        .expect("decisions")
        .pop()
        .expect("blocked decision survives restart");
    assert_eq!(reloaded.state, PlatformCapabilityState::Unavailable);
    assert_eq!(reloaded.task_id, "task-blocked");
    let gates = plane.load_platform_gate_records().expect("gates");
    assert_eq!(gates[0].result, nirman_domain::BuildGateResult::Unavailable);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn worker_scheduling_refuses_blocked_contract() {
    let path = temp_path("scheduling");
    let project_id = ProjectId("project-m118-scheduling".into());
    let plane = DurableControlPlane::open(&path, project_id).expect("open");
    let outcome = plane
        .run_platform_preflight_and_admit(
            "task-sched",
            "windows",
            "x86_64",
            &native_requirements(),
            BuildGateStage::RuntimeValidation,
            Box::new(linux_cross_capable()),
            1_700_000_500,
        )
        .expect("admission run");
    assert!(matches!(outcome, PlatformAdmissionDecision::Blocked { .. }));
    let record = plane
        .load_platform_preflight("task-sched")
        .expect("load")
        .expect("record");
    assert!(
        !nirman_tools::worker_satisfies_platform_requirements(&native_requirements(), &record),
        "a native-execution worker must not be scheduled for the gated steps"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn environment_change_supersedes_and_invalidates_stale_gates() {
    let path = temp_path("supersede");
    let project_id = ProjectId("project-m118-supersede".into());
    let plane = DurableControlPlane::open(&path, project_id).expect("open");
    plane
        .run_platform_preflight_and_admit(
            "task-super",
            "windows",
            "x86_64",
            &cross_requirements(),
            BuildGateStage::TargetBuild,
            Box::new(linux_cross_capable()),
            1_700_000_600,
        )
        .expect("first environment");
    let old = plane
        .load_platform_preflight("task-super")
        .expect("load")
        .expect("old record");

    // The toolchain identity changes (rustc 1.99): a newer observation
    // arrives and must supersede the previous environment record.
    let mut drifted = linux_cross_capable();
    drifted.tools.insert(
        "rustc".into(),
        ToolObservation::installed("rustc 1.99.0", "scripted"),
    );
    let outcome = plane
        .run_platform_preflight_and_admit(
            "task-super",
            "windows",
            "x86_64",
            &cross_requirements(),
            BuildGateStage::TargetBuild,
            Box::new(drifted),
            1_700_000_700,
        )
        .expect("second environment");
    assert!(matches!(outcome, PlatformAdmissionDecision::Admitted));
    let current = plane
        .load_platform_preflight("task-super")
        .expect("load")
        .expect("new record");
    assert_eq!(
        current.supersedes.as_deref(),
        Some(old.environment_id.as_str())
    );
    assert_ne!(current.environment_fingerprint, old.environment_fingerprint);

    // A gate produced under the old environment is stale against the new
    // one (TA §84.2 invalidation).
    let gate = plane
        .load_platform_gate_records()
        .expect("gates")
        .pop()
        .expect("gate");
    let evidence = nirman_tools::PlatformRuntimeEvidence {
        evidence_id: "ev-old".into(),
        environment_id: old.environment_id.clone(),
        environment_fingerprint: old.environment_fingerprint.clone(),
        target_platform: "windows".into(),
        revision: gate.revision.0,
        observations: vec!["target_build_observation".into()],
    };
    assert!(gate_evidence_is_stale(&gate, &[evidence], &current, &old));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn undeclared_target_is_rejected_before_any_persistence() {
    let path = temp_path("undeclared");
    let project_id = ProjectId("project-m118-undeclared".into());
    let plane = DurableControlPlane::open(&path, project_id).expect("open");
    let result = plane.run_platform_preflight_and_admit(
        "task-undeclared",
        "macos",
        "aarch64",
        &cross_requirements(),
        BuildGateStage::TargetBuild,
        Box::new(linux_cross_capable()),
        1_700_000_800,
    );
    assert!(
        matches!(
            result,
            Err(
                nirman_control_plane::PlatformPreflightAdmissionError::Preflight(
                    nirman_tools::PreflightError::UndeclaredTarget(_)
                )
            )
        ),
        "undeclared targets are rejected before any observation is recorded"
    );
    assert!(plane
        .load_platform_preflight("task-undeclared")
        .expect("load")
        .is_none());
    assert!(plane
        .load_platform_gate_records()
        .expect("gates")
        .is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn stage_to_operation_mapping_keeps_gates_distinct() {
    let record_probe = linux_cross_capable();
    let planner = EnvironmentCapabilityPlanner::new(
        Box::new(record_probe.clone()),
        PlatformCapabilityRegistry::canonical_v1(),
    );
    let record = planner.run("windows", "x86_64").expect("record");
    assert_eq!(
        operation_for_stage(BuildGateStage::TargetBuild, &record),
        nirman_tools::CommandOperation::TargetBuild
    );
    assert_eq!(
        operation_for_stage(BuildGateStage::RuntimeValidation, &record),
        nirman_tools::CommandOperation::RuntimeValidation
    );
    // Host-local stages never touch the target host.
    let mut host_probe = linux_cross_capable();
    host_probe.host = "windows".into();
    let host_planner = EnvironmentCapabilityPlanner::new(
        Box::new(host_probe),
        PlatformCapabilityRegistry::canonical_v1(),
    );
    let host_record = host_planner.run("windows", "x86_64").expect("host record");
    assert_eq!(
        operation_for_stage(BuildGateStage::Compile, &host_record),
        nirman_tools::CommandOperation::HostBuild
    );
    let admitted = evaluate_platform_admission(
        &host_record,
        &PlatformRequirements {
            required_target_platforms: vec!["windows".into()],
            ..Default::default()
        },
        BuildGateStage::Compile,
        1_700_000_900,
    );
    assert!(matches!(admitted, PlatformAdmissionDecision::Admitted));
}
