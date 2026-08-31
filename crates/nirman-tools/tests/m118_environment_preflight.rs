//! M118 environment preflight (TA §58.8, §84.1; ADR-206; TEST-PLAT-001,
//! EV-PLAT-001): the `EnvironmentCapabilityPlanner` consumes injectable
//! probe observations and produces the canonical
//! `EnvironmentCapabilityRecord` that the cross-build admission gate and
//! the native-validation gate consume.
//!
//! Part 1 is a scripted fake probe: fully deterministic, platform-
//! independent proofs. Part 2 runs the real `OsProbe` against the
//! developer host at test time and writes an honest evidence trace: the
//! probe *inspects* the environment only — it executes no build, no
//! runtime, and no device session.

use nirman_domain::{
    BuildGateRecord, BuildGateResult, BuildGateStage, EnvironmentCapabilityRecord,
    PlatformCapabilityState, Revision,
};
use nirman_tools::{
    admit_native_validation, admit_target_operation, capability, environment_fingerprint,
    AdmissionOutcome, CommandOperation, EnvironmentCapabilityPlanner, NativeValidationOutcome,
    OsProbe, PlatformCapabilityRegistry, PlatformProbe, PreflightError, TargetPlatformResolver,
    TargetResolutionError, ToolObservation,
};
use std::collections::BTreeMap;

// ───────────────────────────── Fake probe ──────────────────────────────────

struct FakeProbe {
    host: String,
    arch: String,
    tools: BTreeMap<String, ToolObservation>,
}

impl FakeProbe {
    fn linux_cross_capable() -> Self {
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

fn state_of(record: &EnvironmentCapabilityRecord, capability_id: &str) -> PlatformCapabilityState {
    record
        .capability_state(capability_id)
        .unwrap_or_else(|| panic!("capability {capability_id} was not classified"))
}

// ─────────────────────── Part 1: deterministic proofs ──────────────────────

#[test]
fn planner_classifies_the_full_matrix_from_observations() {
    let probe = Box::new(FakeProbe::linux_cross_capable());
    let planner =
        EnvironmentCapabilityPlanner::new(probe, PlatformCapabilityRegistry::canonical_v1());
    let record = planner.run("windows", "x86_64").expect("preflight run");

    assert_eq!(record.host_platform, "linux");
    assert_eq!(record.host_architecture, "x86_64");
    assert_eq!(record.target_platform, "windows");
    assert!(record.is_cross_build());

    // Observation-driven: every state must follow from the scripted tools.
    assert_eq!(
        state_of(&record, capability::SOURCE_COMPILATION),
        PlatformCapabilityState::Available
    );
    assert_eq!(
        state_of(&record, capability::CROSS_BUILD_WINDOWS),
        PlatformCapabilityState::Available
    );
    assert_eq!(
        state_of(&record, capability::WINDOWS_INSTALLER_GENERATION),
        PlatformCapabilityState::Repairable,
        "nsis and wine are missing in the script: repairable, not invented away"
    );
    assert_eq!(
        state_of(&record, capability::WINDOWS_NATIVE_EXECUTION),
        PlatformCapabilityState::Unavailable,
        "platform impossibility is unavailable regardless of tools"
    );
    assert_eq!(
        state_of(&record, capability::ANDROID_BUILD),
        PlatformCapabilityState::Repairable,
        "java present, gradle/SDK missing"
    );

    // Derived summaries agree with the classifications.
    assert!(record.cross_compilation_available);
    assert!(!record.runtime_validation_available);
    assert!(record
        .required_user_actions
        .iter()
        .any(|a| a.contains("windows_native_execution")));
    assert_eq!(
        record.environment_id,
        format!("env-linux-{}", record.environment_fingerprint)
    );
}

#[test]
fn planner_run_is_deterministic_and_fingerprint_tracks_toolchain_identity() {
    let registry = PlatformCapabilityRegistry::canonical_v1();
    let a = EnvironmentCapabilityPlanner::new(
        Box::new(FakeProbe::linux_cross_capable()),
        registry.clone(),
    )
    .run("windows", "x86_64")
    .expect("run a");
    let b = EnvironmentCapabilityPlanner::new(
        Box::new(FakeProbe::linux_cross_capable()),
        registry.clone(),
    )
    .run("windows", "x86_64")
    .expect("run b");
    assert_eq!(a, b, "identical environment => identical record");

    // A different toolchain identity changes the fingerprint.
    let mut changed = FakeProbe::linux_cross_capable();
    changed.tools.insert(
        "rustc".into(),
        ToolObservation::installed("rustc 1.99.0", "scripted"),
    );
    let c = EnvironmentCapabilityPlanner::new(Box::new(changed), registry.clone())
        .run("windows", "x86_64")
        .expect("run c");
    assert_ne!(
        c.environment_fingerprint, a.environment_fingerprint,
        "toolchain identity must move the fingerprint (invalidation input)"
    );
    assert!(c.invalidates_older(&a));
}

#[test]
fn planner_rejects_undeclared_targets_before_any_observation() {
    let planner = EnvironmentCapabilityPlanner::new(
        Box::new(FakeProbe::linux_cross_capable()),
        PlatformCapabilityRegistry::canonical_v1(),
    );
    let err = planner.run("macos", "aarch64").unwrap_err();
    assert_eq!(err, PreflightError::UndeclaredTarget("macos".into()));
    assert_eq!(
        TargetPlatformResolver::resolve("linux", "x86_64", "macos", "aarch64").unwrap_err(),
        nirman_tools::TargetResolutionError::UndeclaredTarget("macos".into())
    );
}

#[test]
fn planner_output_feeds_the_admission_gates() {
    let registry = PlatformCapabilityRegistry::canonical_v1();
    let capable = EnvironmentCapabilityPlanner::new(
        Box::new(FakeProbe::linux_cross_capable()),
        registry.clone(),
    )
    .run("windows", "x86_64")
    .expect("capable record");
    assert!(matches!(
        admit_target_operation(CommandOperation::TargetBuild, &capable, true),
        AdmissionOutcome::Admitted { .. }
    ));
    assert_eq!(
        admit_target_operation(CommandOperation::RuntimeValidation, &capable, true),
        AdmissionOutcome::RuntimeValidationClaimOnNonMatchingHost
    );
    assert_eq!(
        admit_native_validation(&capable, None),
        NativeValidationOutcome::HostDoesNotMatchTarget
    );

    // Toolchain not proven: the gate refuses with the truthful state.
    let mut broken = FakeProbe::linux_cross_capable();
    broken.tools.insert(
        "windows_linker".into(),
        ToolObservation::missing("linker missing"),
    );
    let degraded = EnvironmentCapabilityPlanner::new(Box::new(broken), registry.clone())
        .run("windows", "x86_64")
        .expect("degraded record");
    assert!(!degraded.cross_compilation_available);
    assert_eq!(
        admit_target_operation(CommandOperation::TargetBuild, &degraded, true),
        AdmissionOutcome::ToolchainNotProven(PlatformCapabilityState::Repairable)
    );
}

#[test]
fn fingerprint_is_stable_and_deterministic() {
    let mut versions = BTreeMap::new();
    versions.insert("rustc".to_string(), "1.98.0".to_string());
    let fp = environment_fingerprint("linux", "x86_64", &versions, &BTreeMap::new());
    assert_eq!(fp.len(), 16);
    assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(
        fp,
        environment_fingerprint("linux", "x86_64", &versions, &BTreeMap::new())
    );
    let mut drifted = versions.clone();
    drifted.insert("rustc".to_string(), "1.99.0".to_string());
    assert_ne!(
        fp,
        environment_fingerprint("linux", "x86_64", &drifted, &BTreeMap::new())
    );
}

// ─────────────────── Part 2: real host observation ─────────────────────────

fn evidence_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("..")
        .join("tests")
        .join("evidence")
        .join("m118_environment_preflight.json")
}

#[test]
fn real_host_preflight_records_an_honest_trace() {
    let planner = EnvironmentCapabilityPlanner::new(
        Box::new(OsProbe),
        PlatformCapabilityRegistry::canonical_v1(),
    );
    let record = planner
        .run("windows", "x86_64")
        .expect("the matrix covers the developer host");

    // Structural invariants that hold on any covered host.
    assert!(
        record.host_platform == "linux" || record.host_platform == "windows",
        "unexpected host platform: {}",
        record.host_platform
    );
    assert!(!record.environment_fingerprint.is_empty());
    assert!(record
        .environment_fingerprint
        .chars()
        .all(|c| c.is_ascii_hexdigit()));
    assert!(
        !record.capability_results.is_empty(),
        "the planner must classify the host's full capability set"
    );
    assert!(record
        .capability_results
        .iter()
        .any(|r| r.capability_id == capability::SOURCE_COMPILATION));
    assert!(record
        .capability_results
        .iter()
        .any(|r| r.capability_id == capability::WINDOWS_NATIVE_EXECUTION));

    // Platform facts: a non-Windows host can never have native execution.
    if record.host_platform != "windows" {
        assert_eq!(
            state_of(&record, capability::WINDOWS_NATIVE_EXECUTION),
            PlatformCapabilityState::Unavailable
        );
        assert!(
            !record.runtime_validation_available,
            "no non-Windows host may report native Windows runtime validation available"
        );
    }
    // Cross-build is observation-driven: available only when proven.
    if record.cross_compilation_available {
        assert_eq!(
            state_of(&record, capability::CROSS_BUILD_WINDOWS),
            PlatformCapabilityState::Available
        );
    }

    // The record round-trips as canonical JSON.
    let json = serde_json::to_string(&record).expect("canonical record serializes");
    let back: EnvironmentCapabilityRecord = serde_json::from_str(&json).expect("round trip");
    assert_eq!(back, record);

    // Two observations of the same environment agree on the fingerprint.
    let again = planner
        .run("windows", "x86_64")
        .expect("second observation");
    assert_eq!(
        again.environment_fingerprint, record.environment_fingerprint,
        "the environment is stable within the test window"
    );

    // Honest evidence trace: this probe inspects; it does not execute.
    let device_state = state_of(&record, capability::ANDROID_PHYSICAL_DEVICE);
    let trace = serde_json::json!({
        "schema": "nirman.m118.environment_preflight.v1",
        "milestone": "M118",
        "testFamily": "TEST-PLAT-001",
        "evidenceId": "EV-PLAT-001",
        "note": "Real OsProbe observation of the developer host at test time. The probe inspects the environment only: no build, runtime, or device session was executed by this test.",
        "observedByRealProbe": true,
        "windowsRuntimeObserved": false,
        "androidDeviceSessionObserved": device_state == PlatformCapabilityState::Available,
        "crossBuildExecutedOnHost": false,
        "declaredTarget": "windows",
        "declaredTargetArchitecture": "x86_64",
        "record": serde_json::to_value(&record).expect("record to json"),
    });
    let path = evidence_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create evidence directory");
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&trace).expect("serialize trace"),
    )
    .expect("write trace");

    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(read["observedByRealProbe"], true);
    assert_eq!(read["windowsRuntimeObserved"], false);
    assert_eq!(read["crossBuildExecutedOnHost"], false);
    assert_eq!(read["record"]["target_platform"], "windows");
    assert_eq!(
        read["record"]["environment_fingerprint"],
        record.environment_fingerprint
    );
}

// Keep imports honest: these types are referenced only in some branches.
#[allow(dead_code)]
fn _type_anchors() -> (BuildGateRecord, BuildGateStage, BuildGateResult, Revision) {
    unreachable!("type anchor only; never executed")
}
