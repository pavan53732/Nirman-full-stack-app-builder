//! M118 slice 4 acceptance (BS §79.7, CLAUSE.SKILL.NO_PERMISSION_GRANT):
//! the v1 platform skill set. Six per-platform SkillPackages (no generic
//! catch-all skill), each declaring requiredTools/requiredCapabilities/
//! triggerConditions/permissionRequests/input+outputSchema, permission-
//! neutral, with required capabilities drawn from the canonical matrix,
//! and gated steps that cannot execute when their required capabilities
//! resolve to UNAVAILABLE or USER_REQUIRED.

use std::collections::BTreeMap;

use nirman_domain::PlatformCapabilityState;
use nirman_skills::{
    evaluate_skill_admission, load_builtin_skill_packages, ScanStatus, SkillAdmission,
    SkillPackage, SkillScope, TrustStatus,
};
use nirman_tools::{
    EnvironmentCapabilityPlanner, PlatformCapabilityRegistry, PlatformProbe, ToolObservation,
};

const EXPECTED_V1_SET: [&str; 6] = [
    "environment-preflight",
    "environment-repair",
    "windows-desktop-build",
    "windows-runtime-validation",
    "cross-platform-build-diagnostics",
    "android-toolchain",
];

fn skills_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("skills")
}

fn load_all() -> Vec<SkillPackage> {
    load_builtin_skill_packages(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).expect("load")
}

fn by_id<'a>(packages: &'a [SkillPackage], skill_id: &str) -> &'a SkillPackage {
    packages
        .iter()
        .find(|package| package.skill_id == skill_id)
        .expect("skill present")
}

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
        ("nsis", "makensis 3.1"),
        ("wine", "wine 10.0"),
    ] {
        tools.insert(name.into(), ToolObservation::installed(version, "scripted"));
    }
    for name in ["gradle", "android_sdk", "emulator", "adb", "device"] {
        tools.insert(name.into(), ToolObservation::missing("scripted missing"));
    }
    FakeProbe {
        host: "linux".into(),
        arch: "x86_64".into(),
        tools,
    }
}

fn windows_native() -> FakeProbe {
    let base = linux_cross_capable();
    let arch = base.arch.clone();
    let mut tools = base.tools;
    tools.insert(
        "windows_os".into(),
        ToolObservation::installed("Microsoft Windows 11 Pro", "scripted"),
    );
    tools.insert(
        "dpapi".into(),
        ToolObservation::installed("DPAPI (OS)", "scripted"),
    );
    tools.insert(
        "linker".into(),
        ToolObservation::installed("link.exe 14.40", "scripted"),
    );
    FakeProbe {
        host: "windows".into(),
        arch,
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

fn record_for(probe: FakeProbe) -> nirman_domain::EnvironmentCapabilityRecord {
    let planner = EnvironmentCapabilityPlanner::new(
        Box::new(probe),
        PlatformCapabilityRegistry::canonical_v1(),
    );
    planner.run("windows", "x86_64").expect("record")
}

#[test]
fn all_six_v1_platform_skills_load_from_their_directories() {
    let packages = load_all();
    let ids: Vec<&str> = packages
        .iter()
        .map(|package| package.skill_id.as_str())
        .collect();
    for expected in &EXPECTED_V1_SET {
        assert!(
            ids.contains(expected),
            "missing v1 skill {expected}; loaded: {ids:?}"
        );
    }
    assert_eq!(packages.len(), 6, "the v1 set is exactly six: {ids:?}");
    for package in &packages {
        assert_eq!(package.scope, SkillScope::BuiltIn);
        assert_eq!(package.scan_status, ScanStatus::Clean);
        assert_eq!(package.trust_status, TrustStatus::Trusted);
        assert!(package.enabled);
        assert!(package.is_invocable());
        assert!(
            !package.source_path.starts_with('/') && package.source_path.contains("builtin/"),
            "source_path must be a builtin path: {}",
            package.source_path
        );
        assert!(!package.trigger_conditions.is_empty());
        // The instruction body ships next to the manifest.
        let skill_md = skills_root()
            .join(&package.source_path["builtin/".len()..])
            .join("SKILL.md");
        assert!(
            skill_md.is_file(),
            "SKILL.md missing for {}",
            package.skill_id
        );
    }
}

#[test]
fn no_generic_catch_all_skill_exists() {
    let packages = load_all();
    assert_eq!(packages.len(), 6, "no seventh (generic) skill");
    for package in &packages {
        let haystack = format!(
            "{} {} {}",
            package.skill_id, package.name, package.description
        )
        .to_lowercase()
        .replace('_', " ");
        for token in ["universal", "general", "mega", "catch-all", "all-in-one"] {
            assert!(
                !haystack.contains(token),
                "skill {} looks like a prohibited generic catch-all",
                package.skill_id
            );
        }
    }
    // The loader itself rejects a generic catch-all if one appears.
    let mut generic = by_id(&packages, "environment-preflight").clone();
    generic.skill_id = "universal-coding-skill".into();
    let root = std::env::temp_dir().join(format!("nirman-skill-generic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("skills/universal-coding-skill")).expect("temp dir");
    std::fs::write(
        root.join("skills/universal-coding-skill/skill.json"),
        serde_json::to_string(&generic).expect("serialize"),
    )
    .expect("write");
    let result = load_builtin_skill_packages(&root);
    assert!(
        matches!(
            result,
            Err(nirman_skills::SkillLoadError::SchemaViolation { .. })
        ),
        "a generic catch-all skill must be rejected: {result:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn skill_required_capabilities_are_canonical_matrix_ids() {
    let registry = PlatformCapabilityRegistry::canonical_v1();
    let canonical: Vec<&str> = registry
        .entries
        .iter()
        .map(|entry| entry.capability_id.as_str())
        .collect();
    for package in load_all() {
        for capability in &package.required_capabilities {
            assert!(
                canonical.contains(&capability.as_str()),
                "skill {} requires non-canonical capability {capability}",
                package.skill_id
            );
        }
    }
}

#[test]
fn build_skill_never_requires_runtime_validation_capabilities() {
    let packages = load_all();
    let build = by_id(&packages, "windows-desktop-build");
    let runtime_caps = [
        "windows_native_execution",
        "windows_conpty",
        "windows_job_objects",
        "windows_restricted_tokens",
        "windows_credential_storage",
        "windows_native_ipc",
        "windows_process_supervision_recovery",
    ];
    for capability in runtime_caps {
        assert!(
            !build
                .required_capabilities
                .iter()
                .any(|cap| cap == capability),
            "the build skill must never claim runtime validation (capability {capability})"
        );
    }
    // The output schema pins the no-claim invariant.
    let schema = &build.output_schema;
    assert_eq!(
        schema
            .get("properties")
            .and_then(|properties| properties.get("runtimeValidationClaimed"))
            .and_then(|field| field.get("const"))
            .and_then(|value| value.as_bool()),
        Some(false),
        "the build skill output must pin runtimeValidationClaimed = false"
    );
    let validation = by_id(&packages, "windows-runtime-validation");
    assert_eq!(
        validation
            .output_schema
            .get("properties")
            .and_then(|properties| properties.get("simulated"))
            .and_then(|field| field.get("const"))
            .and_then(|value| value.as_bool()),
        Some(false),
        "the validation skill output must pin simulated = false"
    );
}

#[test]
fn windows_runtime_validation_is_blocked_off_native_host() {
    let packages = load_all();
    let skill = by_id(&packages, "windows-runtime-validation");
    let record = record_for(linux_cross_capable());
    let admission = evaluate_skill_admission(skill, &record);
    match admission {
        SkillAdmission::Blocked {
            blocked_capabilities,
            state,
            reason,
        } => {
            assert_eq!(state, PlatformCapabilityState::Unavailable);
            assert!(
                blocked_capabilities
                    .iter()
                    .any(|(id, _)| id == "windows_native_execution"),
                "the block must cite the native execution capability: {blocked_capabilities:?}"
            );
            assert!(reason.contains("must not execute"));
            assert!(reason.contains(&record.environment_id));
        }
        SkillAdmission::Admitted => {
            panic!(
                "native Windows runtime validation must never be admitted from a non-Windows host"
            )
        }
    }
}

#[test]
fn windows_runtime_validation_is_admitted_on_native_windows_host() {
    let packages = load_all();
    let skill = by_id(&packages, "windows-runtime-validation");
    let record = record_for(windows_native());
    assert!(
        record.capability_state("windows_native_execution")
            == Some(PlatformCapabilityState::Available),
        "sanity: the scripted native host must observe the capability as available"
    );
    assert!(
        matches!(
            evaluate_skill_admission(skill, &record),
            SkillAdmission::Admitted
        ),
        "a native Windows environment admits the runtime validation skill"
    );
}

#[test]
fn host_local_skills_are_admitted_on_a_linux_host() {
    let packages = load_all();
    let record = record_for(linux_cross_capable());
    for skill_id in [
        "environment-preflight",
        "environment-repair",
        "cross-platform-build-diagnostics",
    ] {
        let skill = by_id(&packages, skill_id);
        assert!(
            matches!(
                evaluate_skill_admission(skill, &record),
                SkillAdmission::Admitted
            ),
            "{skill_id} is host-local and must not be blocked by target capabilities"
        );
    }
    // The android toolchain skill blocks only when its own capability is
    // not available — on this scripted host the android toolchain is
    // missing, so it must block with the truthful state, not pretend.
    let android = by_id(&packages, "android-toolchain");
    assert!(
        matches!(
            evaluate_skill_admission(android, &record),
            SkillAdmission::Blocked { .. }
        ),
        "a missing android toolchain must block the android skill truthfully"
    );
}

#[test]
fn skills_request_permissions_but_never_grant_them() {
    let packages = load_all();
    let repair = by_id(&packages, "environment-repair");
    assert_eq!(
        repair.permission_requests,
        vec!["environment.repair".to_string()],
        "environment-repair declares its single policy request"
    );
    for package in &packages {
        if package.skill_id == "environment-repair" {
            continue;
        }
        assert!(
            package.is_permission_neutral(),
            "{} must be permission-neutral",
            package.skill_id
        );
    }
    // Loading a skill never changes trust or grants anything: the loaded
    // package is exactly the shipped declaration.
    let json = serde_json::to_string(repair).expect("serialize");
    let back: SkillPackage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.permission_requests, repair.permission_requests);
    assert_eq!(back.trust_status, TrustStatus::Trusted);
}

#[test]
fn unknown_required_capability_blocks_fail_closed() {
    let packages = load_all();
    let skill = by_id(&packages, "windows-runtime-validation");
    let record = record_for(linux_cross_capable());
    let mut modified = skill.clone();
    modified
        .required_capabilities
        .push("nonexistent_capability".into());
    match evaluate_skill_admission(&modified, &record) {
        SkillAdmission::Blocked {
            blocked_capabilities,
            ..
        } => {
            assert!(
                blocked_capabilities
                    .iter()
                    .any(|(id, _)| id == "nonexistent_capability"),
                "an unproven capability must block fail-closed: {blocked_capabilities:?}"
            );
        }
        SkillAdmission::Admitted => {
            panic!("an unobserved capability must never admit the gated steps")
        }
    }
}
