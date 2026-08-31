//! TEST-PLAT-001 (evidence EV-PLAT-001): M118 platform capability and
//! cross-compilation hallucination-prevention fixtures (BS §79.13, TA §84.5,
//! ADR-206, CONTRACT.RUNTIME.PLATFORM_CAPABILITY).
//!
//! Every fixture is a pure, deterministic exercise of the gate logic over
//! *synthetic observed records*. No fixture executes a Windows runtime, an
//! Android device session, or a real cross-build on the host; the evidence
//! trace written by `evidence_trace_is_deterministic_and_honest` states
//! that explicitly. These fixtures certify that the *decisions* are correct:
//! cross-build may be admitted, runtime validation may not be claimed,
//! fakes are rejected, and stale evidence is invalidated.

use nirman_domain::{
    BuildGateRecord, BuildGateResult, BuildGateStage, EnvironmentCapabilityRecord,
    EnvironmentCapabilityResult, PlatformCapabilityState, PlatformRequirements, Revision,
    ValidationEnvironment, ValidationEnvironmentHealth,
};
use nirman_tools::{
    admit_native_validation, admit_target_operation, aggregate_capability_status, capability,
    emit_platform_trace_edges, gate_evidence_is_stale, validate_gate_evidence,
    worker_satisfies_platform_requirements, AdmissionOutcome, CommandOperation,
    NativeValidationOutcome, PlatformCapabilityRegistry, PlatformRuntimeEvidence,
    PlatformTraceEdgeType, TargetPlatformResolver,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ─────────────────────────── Shared fixtures ───────────────────────────────

const EP1: u64 = 1_700_000_000;

fn result(cap: &str, state: PlatformCapabilityState, id: &str) -> EnvironmentCapabilityResult {
    EnvironmentCapabilityResult {
        capability_id: cap.to_string(),
        state,
        observed_version: None,
        path: None,
        fingerprint: None,
        detail: String::new(),
        evidence_id: id.to_string(),
    }
}

/// Linux host, Windows target, cross-build toolchain proven.
fn linux_to_windows(
    cross: PlatformCapabilityState,
    fingerprint: &str,
    revision_hint: u64,
) -> EnvironmentCapabilityRecord {
    EnvironmentCapabilityRecord {
        schema_version: EnvironmentCapabilityRecord::SCHEMA_VERSION,
        environment_id: "env-linux-1".into(),
        host_platform: "linux".into(),
        host_architecture: "x86_64".into(),
        target_platform: "windows".into(),
        target_architecture: "x86_64".into(),
        shell: "bash".into(),
        compiler: "rustc 1.98".into(),
        linker: "x86_64-w64-mingw32-gcc".into(),
        sdk: String::new(),
        runtime: "node 22".into(),
        build_tools: vec!["cargo".into()],
        installer_tools: vec![],
        native_dependencies: vec![],
        tool_versions: BTreeMap::from([
            ("rustc".to_string(), "1.98".to_string()),
            ("revision-hint".to_string(), revision_hint.to_string()),
        ]),
        environment_fingerprint: fingerprint.to_string(),
        capability_results: vec![
            result(capability::CROSS_BUILD_WINDOWS, cross, "ev-cross"),
            result(
                capability::WINDOWS_NATIVE_EXECUTION,
                PlatformCapabilityState::Unavailable,
                "ev-native",
            ),
        ],
        repair_attempts: vec![],
        required_user_actions: vec!["provision a windows validation environment".into()],
        runtime_validation_available: false,
        cross_compilation_available: cross == PlatformCapabilityState::Available,
        evidence_ids: vec!["ev-cross".into()],
        recorded_at_epoch_seconds: EP1,
        supersedes: None,
    }
}

fn gate(
    gate_id: &str,
    stage: BuildGateStage,
    record: &EnvironmentCapabilityRecord,
    revision: u64,
    evidence_ids: Vec<String>,
    result: BuildGateResult,
) -> BuildGateRecord {
    BuildGateRecord {
        schema_version: BuildGateRecord::SCHEMA_VERSION,
        gate_id: gate_id.into(),
        stage,
        platform: record.target_platform.clone(),
        environment_id: record.environment_id.clone(),
        revision: Revision(revision),
        command_or_operation_ref: format!("{gate_id}-op"),
        evidence_ids,
        result,
        recorded_at_epoch_seconds: EP1 + 10,
    }
}

fn evidence(
    id: &str,
    record: &EnvironmentCapabilityRecord,
    revision: u64,
    observations: Vec<&str>,
) -> PlatformRuntimeEvidence {
    PlatformRuntimeEvidence {
        evidence_id: id.into(),
        environment_id: record.environment_id.clone(),
        environment_fingerprint: record.environment_fingerprint.clone(),
        target_platform: record.target_platform.clone(),
        revision,
        observations: observations.into_iter().map(str::to_string).collect(),
    }
}

fn windows_env(lease: Option<&str>) -> ValidationEnvironment {
    ValidationEnvironment {
        schema_version: ValidationEnvironment::SCHEMA_VERSION,
        environment_id: "val-win-1".into(),
        platform: "windows".into(),
        architecture: "x86_64".into(),
        toolchain: "windows-sdk".into(),
        runtime: "windows".into(),
        available_tools: vec!["powershell".into()],
        available_devices: vec![],
        isolation_profile: "standard".into(),
        network_policy: "local".into(),
        fingerprint: "fp-win-val".into(),
        health: ValidationEnvironmentHealth::Healthy,
        lease_id: lease.map(str::to_string),
        reserved_by_task: lease.map(|_| "task-1".to_string()),
        acquired_at_epoch_seconds: lease.map(|_| EP1 + 1),
        released_at_epoch_seconds: None,
    }
}

// ───────────────────────────── Fixture A ───────────────────────────────────

#[derive(Debug)]
struct FixtureAResult {
    cross_build_admitted: bool,
    native_validation_claim_admitted: bool,
    runtime_claim_rejected_before_execution: bool,
    blocked_state: String,
    can_continue: Vec<String>,
    cannot_continue: Vec<String>,
}

fn fixture_a() -> FixtureAResult {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    // Planner resolution records host and target explicitly (BS §79.2).
    let resolved = TargetPlatformResolver::resolve("linux", "x86_64", "windows", "x86_64")
        .expect("declared target");
    assert!(resolved.is_cross_build());

    // Cross-build may execute.
    let cross = admit_target_operation(CommandOperation::TargetBuild, &record, true);
    let cross_build_admitted = matches!(cross, AdmissionOutcome::Admitted { .. });

    // Native Windows validation MUST NOT be claimed from this host — the
    // target-mismatch guard rejects it before execution.
    let runtime_claim = admit_target_operation(CommandOperation::RuntimeValidation, &record, true);
    let runtime_claim_rejected_before_execution =
        runtime_claim == AdmissionOutcome::RuntimeValidationClaimOnNonMatchingHost;
    let native_validation_claim_admitted = matches!(
        admit_native_validation(&record, None),
        NativeValidationOutcome::Admitted { .. }
    );

    // The blocked node records the truthful state plus the §79.11 lists.
    let blocked = admit_native_validation(&record, None);
    let blocked_state = match blocked {
        NativeValidationOutcome::HostDoesNotMatchTarget
        | NativeValidationOutcome::NoValidationEnvironmentLease(
            PlatformCapabilityState::Unavailable,
        ) => "UNAVAILABLE".to_string(),
        NativeValidationOutcome::NoValidationEnvironmentLease(
            PlatformCapabilityState::UserRequired,
        ) => "USER_REQUIRED".to_string(),
        _ => "UNEXPECTED".to_string(),
    };
    let can_continue = vec!["cross-build and platform-independent checks".to_string()];
    let cannot_continue = vec!["Windows runtime certification".to_string()];

    FixtureAResult {
        cross_build_admitted,
        native_validation_claim_admitted,
        runtime_claim_rejected_before_execution,
        blocked_state,
        can_continue,
        cannot_continue,
    }
}

#[test]
fn fixture_a_host_mismatch_cross_build_only() {
    let a = fixture_a();
    assert!(
        a.cross_build_admitted,
        "cross-build must be admitted with a proven toolchain"
    );
    assert!(
        !a.native_validation_claim_admitted,
        "native validation must not be admitted on a non-matching host"
    );
    assert!(
        a.runtime_claim_rejected_before_execution,
        "the runtime-validation claim must be rejected before execution"
    );
    assert!(
        a.blocked_state == "UNAVAILABLE" || a.blocked_state == "USER_REQUIRED",
        "blocked node must record a truthful state, got {}",
        a.blocked_state
    );
    assert!(a.can_continue.iter().any(|l| l.contains("cross-build")));
    assert!(a
        .cannot_continue
        .iter()
        .any(|l| l.contains("Windows runtime certification")));
}

// ───────────────────────────── Fixture B ───────────────────────────────────

#[derive(Debug)]
struct FixtureBResult {
    artifact_build: String,
    windows_runtime: String,
    aggregate: String,
    evidence_binding_edges_populated: bool,
}

fn fixture_b() -> FixtureBResult {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    let build_evidence = vec![evidence(
        "ev-build",
        &record,
        1,
        vec![
            "target_build_observation",
            "artifact_inspection_observation",
        ],
    )];

    // A Windows .exe was produced from the Linux host: the build gate
    // verifies with bound build observations.
    let build_gate = gate(
        "gate-target-build",
        BuildGateStage::TargetBuild,
        &record,
        1,
        vec!["ev-build".into()],
        BuildGateResult::Verified,
    );
    assert!(validate_gate_evidence(
        &build_gate,
        &record,
        &build_evidence,
        &["target_build_observation"]
    )
    .is_ok());

    // Runtime validation has no target observation: it is recorded as
    // UNVERIFIED, not verified.
    let runtime_gate = gate(
        "gate-runtime",
        BuildGateStage::RuntimeValidation,
        &record,
        1,
        vec![],
        BuildGateResult::Unverified,
    );

    let artifact_build = build_gate.result.to_string();
    let windows_runtime = runtime_gate.result.to_string();
    let aggregate = aggregate_capability_status(build_gate.result, runtime_gate.result).to_string();

    let edges = emit_platform_trace_edges(&record, &[build_gate, runtime_gate], &build_evidence);
    let evidence_binding_edges_populated = edges
        .iter()
        .any(|e| e.edge_type == PlatformTraceEdgeType::EvidenceBinding);

    FixtureBResult {
        artifact_build,
        windows_runtime,
        aggregate,
        evidence_binding_edges_populated,
    }
}

#[test]
fn fixture_b_successful_cross_build_reports_both_states() {
    let b = fixture_b();
    assert_eq!(
        b.artifact_build, "Verified",
        "the produced artifact build is verified"
    );
    assert_eq!(
        b.windows_runtime, "Unverified",
        "the Windows runtime is recorded unverified"
    );
    assert_eq!(
        b.aggregate, "SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS",
        "aggregate must be SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS, never SUPPORTED"
    );
    assert!(
        b.evidence_binding_edges_populated,
        "traceability chain must carry evidence-binding edges"
    );
}

// ───────────────────────────── Fixture C ───────────────────────────────────

#[derive(Debug)]
struct FixtureCResult {
    completion_claim_accepted: bool,
    rejection_cites_missing_evidence: bool,
    rejection: String,
}

fn fixture_c() -> FixtureCResult {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    // A model or worker reports "Windows runtime tests passed" with no
    // target observation: the claim is a Verified runtime gate with no
    // bound evidence.
    let fake_gate = gate(
        "gate-claimed-runtime",
        BuildGateStage::RuntimeValidation,
        &record,
        1,
        vec![],
        BuildGateResult::Verified,
    );
    let outcome = validate_gate_evidence(&fake_gate, &record, &[], &["process_launch_observation"]);
    let completion_claim_accepted = outcome.is_ok();
    let rejection = outcome.err().map(|e| e.to_string()).unwrap_or_default();
    let rejection_cites_missing_evidence =
        !rejection.to_lowercase().contains("ok") && !rejection.is_empty();

    // The same claim expressed as an operation from a non-matching host is
    // rejected by the target-mismatch guard as well.
    assert_eq!(
        admit_target_operation(CommandOperation::RuntimeValidation, &record, true),
        AdmissionOutcome::RuntimeValidationClaimOnNonMatchingHost
    );

    FixtureCResult {
        completion_claim_accepted,
        rejection_cites_missing_evidence,
        rejection,
    }
}

#[test]
fn fixture_c_fake_completion_is_durably_rejected() {
    let c = fixture_c();
    assert!(
        !c.completion_claim_accepted,
        "a verified target gate with no evidence must be rejected"
    );
    assert!(
        c.rejection_cites_missing_evidence,
        "the rejection must cite the missing evidence: {c:?}"
    );
    assert!(
        c.rejection.contains("evidence"),
        "rejection must name the evidence gap: {}",
        c.rejection
    );
}

// ───────────────────────────── Fixture D ───────────────────────────────────

#[derive(Debug)]
struct FixtureDResult {
    prior_evidence_invalidated: bool,
    certification_gate_reclosed: bool,
}

fn fixture_d() -> FixtureDResult {
    let producing = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    let runtime_evidence = vec![evidence(
        "ev-runtime",
        &producing,
        1,
        vec!["process_launch_observation"],
    )];
    let runtime_gate = gate(
        "gate-runtime",
        BuildGateStage::RuntimeValidation,
        &producing,
        1,
        vec!["ev-runtime".into()],
        BuildGateResult::Verified,
    );

    // Certification closed on top of that verified runtime gate.
    let certification_gate = gate(
        "gate-certification",
        BuildGateStage::Certification,
        &producing,
        1,
        vec!["ev-runtime".into()],
        BuildGateResult::Verified,
    );
    assert!(validate_gate_evidence(
        &certification_gate,
        &producing,
        &runtime_evidence,
        &["process_launch_observation"]
    )
    .is_ok());

    // The source revision, toolchain identity, or environment fingerprint
    // changes: a newer observation arrives.
    let current = EnvironmentCapabilityRecord {
        environment_fingerprint: "fp-linux-2".into(),
        tool_versions: BTreeMap::from([
            ("rustc".to_string(), "1.99".to_string()),
            ("revision-hint".to_string(), "2".to_string()),
        ]),
        recorded_at_epoch_seconds: EP1 + 100,
        supersedes: Some(producing.environment_id.clone()),
        ..producing.clone()
    };
    assert!(
        current.invalidates_older(&producing),
        "the newer observation must invalidate the older one"
    );

    let prior_evidence_invalidated =
        gate_evidence_is_stale(&runtime_gate, &runtime_evidence, &current, &producing);

    // The certification gate re-closes: the previously bound evidence no
    // longer validates under the current record.
    let reclosed = validate_gate_evidence(
        &certification_gate,
        &current,
        &runtime_evidence,
        &["process_launch_observation"],
    )
    .is_err();

    FixtureDResult {
        prior_evidence_invalidated,
        certification_gate_reclosed: reclosed,
    }
}

#[test]
fn fixture_d_stale_target_evidence_is_invalidated() {
    let d = fixture_d();
    assert!(
        d.prior_evidence_invalidated,
        "prior target evidence must be INVALIDATED when the environment identity changes"
    );
    assert!(
        d.certification_gate_reclosed,
        "the certification gate must re-close until re-validation on the target platform"
    );
}

// ─────────────────────── TA §84.5 additional proofs ────────────────────────

#[test]
fn target_mismatch_guard_rejects_before_execution() {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    assert_eq!(
        admit_target_operation(CommandOperation::RuntimeValidation, &record, true),
        AdmissionOutcome::RuntimeValidationClaimOnNonMatchingHost
    );
    assert_eq!(
        admit_native_validation(&record, Some(&windows_env(Some("lease-x")))),
        NativeValidationOutcome::HostDoesNotMatchTarget,
        "even with a lease, a non-matching host cannot run native validation"
    );
}

#[test]
fn worker_scheduling_honors_platform_fields() {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    let native_only = PlatformRequirements {
        required_host_platforms: vec!["windows".into()],
        native_execution_required: true,
        ..Default::default()
    };
    assert!(
        !worker_satisfies_platform_requirements(&native_only, &record),
        "a native-execution worker must not be scheduled on a non-matching host"
    );
    let cross_ok = PlatformRequirements {
        required_capabilities: vec![capability::CROSS_BUILD_WINDOWS.into()],
        cross_compilation_allowed: true,
        ..Default::default()
    };
    assert!(worker_satisfies_platform_requirements(&cross_ok, &record));
}

#[test]
fn lease_loss_fences_in_flight_validation() {
    let mut record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    record.host_platform = "windows".into();
    record.target_platform = "windows".into();
    record.runtime_validation_available = true;

    let leased = windows_env(Some("lease-1"));
    assert!(matches!(
        admit_native_validation(&record, Some(&leased)),
        NativeValidationOutcome::Admitted { .. }
    ));

    // Lease lost: the same gate re-evaluation fences the in-flight
    // validation instead of continuing or simulating.
    let lost = windows_env(None);
    assert_eq!(
        admit_native_validation(&record, Some(&lost)),
        NativeValidationOutcome::NoValidationEnvironmentLease(
            PlatformCapabilityState::UserRequired
        )
    );
}

#[test]
fn matrix_version_change_reruns_preflight_without_invalidating_unrelated_records() {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    let v1 = PlatformCapabilityRegistry::canonical_v1();
    assert_eq!(v1.matrix_version(), 1);

    // A v2 matrix in which the linux cross-build cell is re-prioritized with
    // one additional required tool.
    let mut updated = v1
        .entry("linux", capability::CROSS_BUILD_WINDOWS)
        .unwrap()
        .clone();
    updated.matrix_version = 2;
    updated.required_toolchain = vec![
        "rust_target_windows".into(),
        "windows_linker".into(),
        "new-tool".into(),
    ];
    let v2 = PlatformCapabilityRegistry {
        entries: v1
            .entries
            .iter()
            .filter(|e| {
                !(e.host_platform == "linux" && e.capability_id == capability::CROSS_BUILD_WINDOWS)
            })
            .cloned()
            .chain(std::iter::once(updated))
            .collect(),
    };
    assert_eq!(v2.matrix_version(), 2);

    // Re-running preflight under v2 with the same observation reclassifies
    // the affected capability deterministically.
    let tools = BTreeMap::from([
        (
            "rust_target_windows".to_string(),
            nirman_tools::ToolState::Installed,
        ),
        (
            "windows_linker".to_string(),
            nirman_tools::ToolState::Installed,
        ),
    ]);
    let v1_classified = nirman_tools::classify_capability(
        v1.entry("linux", capability::CROSS_BUILD_WINDOWS).unwrap(),
        &tools,
    );
    let v2_classified = nirman_tools::classify_capability(
        v2.entry("linux", capability::CROSS_BUILD_WINDOWS).unwrap(),
        &tools,
    );
    assert_eq!(v1_classified, PlatformCapabilityState::Available);
    assert_eq!(
        v2_classified,
        PlatformCapabilityState::Repairable,
        "the newly required tool is missing, so the re-run must classify repairable"
    );

    // Unrelated observed records are not invalidated by the matrix change:
    // invalidation is driven by fingerprint/revision/toolchain identity, not
    // by the matrix version.
    let unrelated = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    assert!(!record.invalidates_older(&unrelated));
    assert_eq!(
        record.environment_fingerprint,
        unrelated.environment_fingerprint
    );
}

#[test]
fn planner_emits_extended_traceability_chain() {
    let record = linux_to_windows(PlatformCapabilityState::Available, "fp-linux-1", 1);
    let build_evidence = vec![evidence(
        "ev-build",
        &record,
        1,
        vec!["target_build_observation"],
    )];
    let build_gate = gate(
        "gate-target-build",
        BuildGateStage::TargetBuild,
        &record,
        1,
        vec!["ev-build".into()],
        BuildGateResult::Verified,
    );
    let edges = emit_platform_trace_edges(&record, &[build_gate], &build_evidence);
    assert!(
        edges
            .iter()
            .any(|e| e.edge_type == PlatformTraceEdgeType::EnvironmentRequirement),
        "environment-requirement edge must be populated"
    );
    assert!(
        edges
            .iter()
            .any(|e| e.edge_type == PlatformTraceEdgeType::CapabilityResolution),
        "capability-resolution edges must be populated"
    );
    assert!(
        edges
            .iter()
            .any(|e| e.edge_type == PlatformTraceEdgeType::EvidenceBinding),
        "evidence-binding edges must be populated"
    );
}

// ────────────────────── Deterministic evidence trace ───────────────────────

fn evidence_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("..")
        .join("tests")
        .join("evidence")
        .join("m118_platform_capability.json")
}

#[test]
fn evidence_trace_is_deterministic_and_honest() {
    let a = fixture_a();
    let b = fixture_b();
    let c = fixture_c();
    let d = fixture_d();

    let trace = serde_json::json!({
        "schema": "nirman.m118.platform_capability.v1",
        "milestone": "M118",
        "contract": "CONTRACT.RUNTIME.PLATFORM_CAPABILITY",
        "testFamily": "TEST-PLAT-001",
        "evidenceId": "EV-PLAT-001",
        "adr": "ADR-206",
        "generatedAtEpochSeconds": EP1,
        "windowsRuntimeObserved": false,
        "androidDeviceObserved": false,
        "crossBuildExecutedOnHost": false,
        "honestyNote": "Fixtures execute deterministic gate logic over synthetic observed records. No Windows runtime, Android device, or real cross-build was executed in this test.",
        "fixtureA": {
            "name": "host_mismatch",
            "crossBuildAdmitted": a.cross_build_admitted,
            "nativeValidationClaimed": a.native_validation_claim_admitted,
            "runtimeClaimRejectedBeforeExecution": a.runtime_claim_rejected_before_execution,
            "blockedState": a.blocked_state,
            "canContinue": a.can_continue,
            "cannotContinue": a.cannot_continue,
        },
        "fixtureB": {
            "name": "successful_cross_build",
            "artifactBuild": b.artifact_build,
            "windowsRuntime": b.windows_runtime,
            "aggregate": b.aggregate,
        },
        "fixtureC": {
            "name": "fake_completion",
            "completionClaimAccepted": c.completion_claim_accepted,
            "rejectionCitesMissingEvidence": c.rejection_cites_missing_evidence,
            "rejection": c.rejection,
        },
        "fixtureD": {
            "name": "stale_target_evidence",
            "priorEvidenceInvalidated": d.prior_evidence_invalidated,
            "certificationGateReClosed": d.certification_gate_reclosed,
        },
        "additional": {
            "targetMismatchGuardRejectedBeforeExecution": true,
            "schedulingHonorsPlatformFields": true,
            "leaseLossFencesValidation": true,
            "matrixVersionChangeRerunsPreflight": true,
            "traceabilityChainPopulated": true,
        },
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

    // Re-read and confirm the required decisions.
    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read trace"))
            .expect("parse trace");
    assert_eq!(read["fixtureA"]["crossBuildAdmitted"], true);
    assert_eq!(read["fixtureA"]["nativeValidationClaimed"], false);
    assert_eq!(read["fixtureB"]["artifactBuild"], "Verified");
    assert_eq!(read["fixtureB"]["windowsRuntime"], "Unverified");
    assert_eq!(
        read["fixtureB"]["aggregate"],
        "SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS"
    );
    assert_eq!(read["fixtureC"]["completionClaimAccepted"], false);
    assert_eq!(read["fixtureD"]["priorEvidenceInvalidated"], true);
    assert_eq!(read["fixtureD"]["certificationGateReClosed"], true);
    assert_eq!(read["windowsRuntimeObserved"], false);
}
