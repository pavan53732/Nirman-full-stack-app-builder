//! M119 control-plane acceptance (TA §19.1, BS §79.7): the skill registry
//! and invocation records are durable; selection resolves required skill
//! ids against the registry and the environment record with the truthful
//! outcome — admitted, not found, not invocable, or blocked fail-closed.

use nirman_control_plane::DurableControlPlane;
use nirman_domain::ProjectId;
use nirman_skills::{
    load_builtin_skill_packages, select_required_skills, ScanStatus, SkillInvocationRecord,
    SkillInvocationStatus, SkillPackage, SkillScope, SkillSelectionOutcome, TrustStatus,
};
use std::path::PathBuf;

/// The skills crate directory: the loader appends `skills/` itself.
fn skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../nirman-skills")
}

fn temp_path(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("nirman-m119-cp-{}-{:?}", name, std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join(format!("{name}.sqlite"))
}

fn builtin() -> Vec<SkillPackage> {
    load_builtin_skill_packages(&skills_dir()).expect("built-in skill set loads")
}

fn test_package(skill_id: &str, version: &str) -> SkillPackage {
    SkillPackage {
        skill_id: skill_id.into(),
        name: skill_id.replace('-', " ").into(),
        description: "test skill".into(),
        version: version.into(),
        scope: SkillScope::BuiltIn,
        compatible_worker_roles: vec!["test".into()],
        trigger_conditions: vec!["test".into()],
        required_tools: vec![],
        required_capabilities: vec![],
        permission_requests: vec![],
        input_schema: serde_json::json!({"type": "object"}),
        output_schema: serde_json::json!({"type": "object"}),
        source_path: format!("builtin/test/{skill_id}"),
        scan_status: ScanStatus::Clean,
        trust_status: TrustStatus::Trusted,
        enabled: true,
        installed_at: 0,
        last_used_at: None,
    }
}

#[test]
fn skill_packages_and_invocations_are_durable_across_restart() {
    let path = temp_path("restart");
    let project_id = ProjectId("project-m119-durable".into());
    let plane = DurableControlPlane::open(&path, project_id.clone()).expect("open");
    for package in builtin() {
        plane.save_skill_package(&package).expect("save package");
    }
    let invocation = SkillInvocationRecord {
        invocation_id: "skillinv-task-durable-rev1-environment-preflight".into(),
        skill_id: "environment-preflight".into(),
        skill_version: "1.0.0".into(),
        session_id: "task-task-durable".into(),
        requested_by: "worker-durable".into(),
        trigger_reason: "worker_contract".into(),
        granted_permissions: vec![],
        tool_calls: vec![],
        started_at: 1_700_000_000,
        completed_at: None,
        status: SkillInvocationStatus::Active,
    };
    plane
        .save_skill_invocation_record("task-durable", &invocation)
        .expect("save invocation");
    let stored = plane.load_skill_packages().expect("load packages");
    assert_eq!(stored.len(), 6, "the v1 set is stored: {:?}", stored.len());
    drop(plane);

    let plane = DurableControlPlane::open(&path, project_id).expect("reopen");
    let reloaded = plane
        .load_skill_packages()
        .expect("load packages after restart");
    assert_eq!(reloaded.len(), 6);
    let invocations = plane
        .load_skill_invocation_records("task-durable")
        .expect("load invocations after restart");
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].invocation_id, invocation.invocation_id);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn skill_package_sync_is_idempotent_and_versioned() {
    let path = temp_path("idempotent");
    let project_id = ProjectId("project-m119-idempotent".into());
    let plane = DurableControlPlane::open(&path, project_id).expect("open");
    let package = test_package("test-skill", "1.0.0");
    plane.save_skill_package(&package).expect("save");
    plane.save_skill_package(&package).expect("re-save");
    assert_eq!(
        plane.load_skill_packages().expect("load").len(),
        1,
        "re-syncing the same package upserts, it does not accumulate"
    );
    let mut newer = package.clone();
    newer.version = "1.1.0".into();
    plane.save_skill_package(&newer).expect("save newer");
    let packages = plane.load_skill_packages().expect("load");
    assert_eq!(packages.len(), 2, "distinct versions coexist");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn selection_admits_invocable_capability_free_skill_without_record() {
    let registry = builtin();
    let selection = select_required_skills(&["environment-preflight".into()], &registry, None);
    assert_eq!(selection.len(), 1);
    assert!(
        matches!(selection[0].1, SkillSelectionOutcome::Admitted { .. }),
        "environment-preflight requires no capabilities and no record: {:?}",
        selection[0].1
    );
}

#[test]
fn selection_blocks_capability_skill_without_record_fail_closed() {
    let registry = builtin();
    let selection = select_required_skills(&["windows-runtime-validation".into()], &registry, None);
    match &selection[0].1 {
        SkillSelectionOutcome::Blocked { package, admission } => {
            assert_eq!(package.skill_id, "windows-runtime-validation");
            assert!(
                matches!(
                    admission,
                    nirman_skills::SkillAdmission::Blocked {
                        state: nirman_domain::PlatformCapabilityState::Unavailable,
                        ..
                    }
                ),
                "an unproven capability-bearing skill blocks fail-closed: {admission:?}"
            );
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
}

#[test]
fn selection_reports_not_found_with_the_available_set() {
    let registry = builtin();
    let selection = select_required_skills(&["no-such-skill".into()], &registry, None);
    match &selection[0].1 {
        SkillSelectionOutcome::NotFound {
            requested,
            available,
        } => {
            assert_eq!(requested, "no-such-skill");
            assert_eq!(
                available.len(),
                6,
                "the available set is named: {available:?}"
            );
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn selection_rejects_unscanned_and_revoked_packages() {
    let registry = builtin();
    let mut unscanned = registry
        .iter()
        .find(|package| package.skill_id == "environment-preflight")
        .expect("package")
        .clone();
    unscanned.scan_status = ScanStatus::Pending;
    let mut revoked = registry
        .iter()
        .find(|package| package.skill_id == "environment-repair")
        .expect("package")
        .clone();
    revoked.trust_status = TrustStatus::Revoked;
    let registry = vec![unscanned, revoked];
    let selection = select_required_skills(
        &["environment-preflight".into(), "environment-repair".into()],
        &registry,
        None,
    );
    assert!(
        matches!(selection[0].1, SkillSelectionOutcome::NotInvocable { .. }),
        "an unscanned package is not invocable: {:?}",
        selection[0].1
    );
    assert!(
        matches!(selection[1].1, SkillSelectionOutcome::NotInvocable { .. }),
        "a revoked package is not invocable: {:?}",
        selection[1].1
    );
}
