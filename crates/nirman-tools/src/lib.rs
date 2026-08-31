//! M118: Platform capability resolution and cross-compilation gates.
//!
//! Implements the deterministic core of `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`
//! (ADR-206, Build Spec §79, Technical Architecture §84.3).
//!
//! Design invariants honored here:
//!
//! * The four-state invariant (BS §79.1): host environment, target platform,
//!   validation platform, and certification status are distinct inputs and
//!   outputs. Nothing in this crate collapses them.
//! * Deterministic classification (CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION):
//!   every decision is a pure function of observed records. No model output,
//!   heuristic, or OS call participates.
//! * No runtime inference (CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE): a
//!   `TargetBuild` admission never admits, implies, or records a runtime
//!   validation result.
//! * No substitute targets (CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET): only a
//!   matching `ValidationEnvironment` under a durable lease admits native
//!   validation; containers/VMs/simulation are not accepted inputs.
//!
//! This crate is headless by construction: it consumes *observed*
//! `EnvironmentCapabilityRecord` values (produced by the environment
//! preflight skill on a real host) and emits decisions. It never probes the
//! OS itself, so its fixtures are deterministic and platform-independent.

use nirman_domain::{
    BuildGateRecord, BuildGateResult, EnvironmentCapabilityRecord, EnvironmentCapabilityResult,
    MatrixExpectedResult, PlatformCapabilityEntry, PlatformCapabilityState, PlatformRequirements,
    ValidationEnvironment, ValidationEnvironmentHealth,
};
use std::collections::BTreeMap;

/// Capability ids used by the canonical matrix (BS §79.3 minimum coverage).
pub mod capability {
    pub const SOURCE_COMPILATION: &str = "source_compilation";
    pub const DEPENDENCY_INSTALLATION: &str = "dependency_installation";
    pub const STATIC_ANALYSIS: &str = "static_analysis";
    pub const HOST_NATIVE_TESTS: &str = "host_native_tests";
    pub const CROSS_BUILD_WINDOWS: &str = "cross_build_windows";
    pub const WINDOWS_INSTALLER_GENERATION: &str = "windows_installer_generation";
    pub const ARTIFACT_INSPECTION: &str = "artifact_inspection";
    pub const WINDOWS_NATIVE_EXECUTION: &str = "windows_native_execution";
    pub const WINDOWS_CONPTY: &str = "windows_conpty";
    pub const WINDOWS_JOB_OBJECTS: &str = "windows_job_objects";
    pub const WINDOWS_RESTRICTED_TOKENS: &str = "windows_restricted_tokens";
    pub const WINDOWS_CREDENTIAL_STORAGE: &str = "windows_credential_storage";
    pub const WINDOWS_NATIVE_IPC: &str = "windows_native_ipc";
    pub const WINDOWS_PROCESS_SUPERVISION: &str = "windows_process_supervision_recovery";
    pub const ANDROID_BUILD: &str = "android_build";
    pub const ANDROID_EMULATOR: &str = "android_emulator";
    pub const ANDROID_PHYSICAL_DEVICE: &str = "android_physical_device";
}

/// Declared targets of the product scope (TA §84.3): generated applications
/// are Android; Nirman's own desktop artifact is the Windows host.
pub const DECLARED_TARGET_PLATFORMS: &[&str] = &["android", "windows"];

pub const MATRIX_VERSION_V1: u32 = 1;

// ─────────────────────────── TargetPlatformResolver ────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub enum TargetResolutionError {
    UndeclaredTarget(String),
    EmptyPlatform,
}

impl std::fmt::Display for TargetResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetResolutionError::UndeclaredTarget(t) => {
                write!(
                    f,
                    "declared target platform is not in the product scope: {t}"
                )
            }
            TargetResolutionError::EmptyPlatform => write!(f, "platform identifier is empty"),
        }
    }
}

/// Resolved, explicitly recorded host/target pair (BS §79.2). Downstream
/// consumers must use these recorded values and never re-infer them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub host_platform: String,
    pub host_architecture: String,
    pub target_platform: String,
    pub target_architecture: String,
}

impl ResolvedTarget {
    pub fn is_cross_build(&self) -> bool {
        self.host_platform != self.target_platform
    }
}

/// `TargetPlatformResolver` (TA §58.1 module, §84.3): resolves the declared
/// target for a task and rejects targets outside the product scope.
pub struct TargetPlatformResolver;

impl TargetPlatformResolver {
    pub fn resolve(
        host_platform: &str,
        host_architecture: &str,
        declared_target: &str,
        target_architecture: &str,
    ) -> Result<ResolvedTarget, TargetResolutionError> {
        for value in [
            host_platform,
            host_architecture,
            declared_target,
            target_architecture,
        ] {
            if value.trim().is_empty() {
                return Err(TargetResolutionError::EmptyPlatform);
            }
        }
        if !DECLARED_TARGET_PLATFORMS.contains(&declared_target) {
            return Err(TargetResolutionError::UndeclaredTarget(
                declared_target.to_string(),
            ));
        }
        Ok(ResolvedTarget {
            host_platform: host_platform.to_string(),
            host_architecture: host_architecture.to_string(),
            target_platform: declared_target.to_string(),
            target_architecture: target_architecture.to_string(),
        })
    }
}

// ────────────────────────── PlatformCapabilityRegistry ─────────────────────

/// Canonical platform capability matrix (BS §79.3). The matrix is a prior
/// for preflight, not a truth source: `environment_dependent` cells must be
/// classified from observation, and `unavailable_by_platform` reflects
/// platform impossibility (e.g. ConPTY on a non-Windows host), which no
/// toolchain repair can change.
#[derive(Clone)]
pub struct PlatformCapabilityRegistry {
    pub entries: Vec<PlatformCapabilityEntry>,
}

impl PlatformCapabilityRegistry {
    /// Canonical v1 matrix.
    pub fn canonical_v1() -> Self {
        let mut entries = Vec::new();
        let mut add = |host: &str,
                       cap: &str,
                       expected: MatrixExpectedResult,
                       tools: &[&str],
                       evidence: &[&str]| {
            entries.push(PlatformCapabilityEntry {
                capability_id: cap.to_string(),
                host_platform: host.to_string(),
                expected_result: expected,
                required_toolchain: tools.iter().map(|t| t.to_string()).collect(),
                evidence_requirements: evidence.iter().map(|e| e.to_string()).collect(),
                matrix_version: MATRIX_VERSION_V1,
            });
        };

        for host in ["linux", "windows"] {
            let native = host == "windows";
            let unavailable = if native {
                MatrixExpectedResult::Available
            } else {
                MatrixExpectedResult::UnavailableByPlatform
            };
            add(
                host,
                capability::SOURCE_COMPILATION,
                MatrixExpectedResult::Available,
                &["rustc", "node"],
                &["compile_observation"],
            );
            add(
                host,
                capability::DEPENDENCY_INSTALLATION,
                MatrixExpectedResult::EnvironmentDependent,
                &["package_manager"],
                &["install_observation"],
            );
            add(
                host,
                capability::STATIC_ANALYSIS,
                MatrixExpectedResult::Available,
                &["cargo", "tsc"],
                &["static_analysis_observation"],
            );
            add(
                host,
                capability::HOST_NATIVE_TESTS,
                MatrixExpectedResult::Available,
                &["cargo", "node"],
                &["test_run_observation"],
            );
            add(
                host,
                capability::ARTIFACT_INSPECTION,
                MatrixExpectedResult::Available,
                &["file"],
                &["artifact_inspection_observation"],
            );
            if native {
                add(
                    host,
                    capability::CROSS_BUILD_WINDOWS,
                    MatrixExpectedResult::Available,
                    &["rustc", "linker"],
                    &["target_build_observation"],
                );
                add(
                    host,
                    capability::WINDOWS_INSTALLER_GENERATION,
                    MatrixExpectedResult::EnvironmentDependent,
                    &["nsis"],
                    &["installer_observation"],
                );
            } else {
                add(
                    host,
                    capability::CROSS_BUILD_WINDOWS,
                    MatrixExpectedResult::EnvironmentDependent,
                    &["rust_target_windows", "windows_linker"],
                    &["target_build_observation"],
                );
                add(
                    host,
                    capability::WINDOWS_INSTALLER_GENERATION,
                    MatrixExpectedResult::EnvironmentDependent,
                    &["nsis", "wine"],
                    &["installer_observation"],
                );
            }
            add(
                host,
                capability::WINDOWS_NATIVE_EXECUTION,
                unavailable,
                &["windows_os"],
                &[
                    "process_launch_observation",
                    "executable_path",
                    "process_identity",
                    "runtime_output",
                ],
            );
            add(
                host,
                capability::WINDOWS_CONPTY,
                unavailable,
                &["windows_os"],
                &["conpty_observation"],
            );
            add(
                host,
                capability::WINDOWS_JOB_OBJECTS,
                unavailable,
                &["windows_os"],
                &["job_object_observation"],
            );
            add(
                host,
                capability::WINDOWS_RESTRICTED_TOKENS,
                unavailable,
                &["windows_os"],
                &["restricted_token_observation"],
            );
            add(
                host,
                capability::WINDOWS_CREDENTIAL_STORAGE,
                unavailable,
                &["windows_os", "dpapi"],
                &["credential_storage_observation"],
            );
            add(
                host,
                capability::WINDOWS_NATIVE_IPC,
                unavailable,
                &["windows_os"],
                &["ipc_observation"],
            );
            add(
                host,
                capability::WINDOWS_PROCESS_SUPERVISION,
                unavailable,
                &["windows_os"],
                &["supervision_observation", "recovery_observation"],
            );
            add(
                host,
                capability::ANDROID_BUILD,
                MatrixExpectedResult::EnvironmentDependent,
                &["java", "gradle", "android_sdk"],
                &["android_build_observation"],
            );
            add(
                host,
                capability::ANDROID_EMULATOR,
                MatrixExpectedResult::EnvironmentDependent,
                &["emulator"],
                &["emulator_session_observation"],
            );
            add(
                host,
                capability::ANDROID_PHYSICAL_DEVICE,
                MatrixExpectedResult::EnvironmentDependent,
                &["adb", "device"],
                &["device_session_observation"],
            );
        }
        Self { entries }
    }

    /// The registry version is the highest version among its priors, so a
    /// partially re-prioritized matrix reports the newest version applied.
    pub fn matrix_version(&self) -> u32 {
        self.entries
            .iter()
            .map(|e| e.matrix_version)
            .max()
            .unwrap_or(0)
    }

    /// Looks up the matrix prior for (host, capability). Returns `None` when
    /// the pair is not covered; callers must treat that as
    /// environment-dependent, never as unavailable.
    pub fn entry(
        &self,
        host_platform: &str,
        capability_id: &str,
    ) -> Option<&PlatformCapabilityEntry> {
        self.entries
            .iter()
            .find(|e| e.host_platform == host_platform && e.capability_id == capability_id)
    }
}

// ─────────────────────────── Classification core ───────────────────────────

/// Observed tool state per the §9.2 diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    Installed,
    Missing,
    Outdated,
    Misconfigured,
    Inaccessible,
}

/// Deterministic classification of one capability (BS §79.4):
///
/// * `unavailable_by_platform` prior → `Unavailable` (no observation
///   changes a platform impossibility).
/// * any required tool observed `Inaccessible` → `UserRequired`.
/// * every required tool observed `Installed` → `Available`.
/// * otherwise (missing/outdated/misconfigured, or a required tool absent
///   from the observation) → `Repairable`.
///
/// A required tool that the preflight did not observe counts as missing:
/// an honest preflight cannot credit an unobserved tool as installed.
/// `available` and `environment_dependent` priors are decided identically:
/// the observation is what decides.
pub fn classify_capability(
    entry: &PlatformCapabilityEntry,
    observed: &BTreeMap<String, ToolState>,
) -> PlatformCapabilityState {
    if entry.expected_result == MatrixExpectedResult::UnavailableByPlatform {
        return PlatformCapabilityState::Unavailable;
    }
    for tool in &entry.required_toolchain {
        if observed.get(tool) == Some(&ToolState::Inaccessible) {
            return PlatformCapabilityState::UserRequired;
        }
    }
    if entry
        .required_toolchain
        .iter()
        .all(|tool| observed.get(tool) == Some(&ToolState::Installed))
    {
        return PlatformCapabilityState::Available;
    }
    PlatformCapabilityState::Repairable
}

// ───────────────────────── Cross-build admission gate ──────────────────────

/// Declared operation class of a command (TA §84.3 target-mismatch guard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOperation {
    HostBuild,
    TargetBuild,
    RuntimeValidation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionOutcome {
    Admitted {
        classification: PlatformCapabilityState,
    },
    /// TARGET_BUILD without a proven toolchain: not admitted; the
    /// classification names the truthful state (repair/user-required/
    /// unavailable) for the blocked node.
    ToolchainNotProven(PlatformCapabilityState),
    /// A runtime-validation command or claim issued from a non-matching
    /// host. Hard rejection before execution (BS §79.11 blocked state).
    RuntimeValidationClaimOnNonMatchingHost,
    /// The worker contract does not allow cross-compilation.
    CrossCompilationNotAllowedForContract,
    /// Undeclared capability for admission purposes.
    UnknownCapability(String),
}

/// `CrossCompilationAuthority` decision point (TA §84.3) — a decision point
/// of ToolBroker/PolicyAuthority, not a new authority.
///
/// Rules (BS §79.5):
/// * `RuntimeValidation` is admitted only when host platform equals target
///   platform and the native capability is available.
/// * `TargetBuild` on the same host is admitted (host build).
/// * `TargetBuild` across hosts (cross-build) is admitted only when the
///   worker contract allows cross-compilation and the cross-build
///   capability classifies `Available`. Admission records an
///   artifact-production decision only — never a runtime result.
pub fn admit_target_operation(
    operation: CommandOperation,
    record: &EnvironmentCapabilityRecord,
    cross_compilation_allowed: bool,
) -> AdmissionOutcome {
    let cross = record.host_platform != record.target_platform;
    match operation {
        CommandOperation::HostBuild => AdmissionOutcome::Admitted {
            classification: PlatformCapabilityState::Available,
        },
        CommandOperation::RuntimeValidation => {
            if cross {
                return AdmissionOutcome::RuntimeValidationClaimOnNonMatchingHost;
            }
            match record.capability_state(capability::WINDOWS_NATIVE_EXECUTION) {
                Some(PlatformCapabilityState::Available) => AdmissionOutcome::Admitted {
                    classification: PlatformCapabilityState::Available,
                },
                Some(state) => AdmissionOutcome::ToolchainNotProven(state),
                None => AdmissionOutcome::ToolchainNotProven(PlatformCapabilityState::Unavailable),
            }
        }
        CommandOperation::TargetBuild => {
            if !cross {
                return AdmissionOutcome::Admitted {
                    classification: PlatformCapabilityState::Available,
                };
            }
            if !cross_compilation_allowed {
                return AdmissionOutcome::CrossCompilationNotAllowedForContract;
            }
            // The classified capability state is the single source of truth;
            // `cross_compilation_available` is the planner's derived summary
            // of this same observation and must not re-decide it.
            match record.capability_state(capability::CROSS_BUILD_WINDOWS) {
                Some(PlatformCapabilityState::Available) => AdmissionOutcome::Admitted {
                    classification: PlatformCapabilityState::Available,
                },
                Some(state) => AdmissionOutcome::ToolchainNotProven(state),
                None => {
                    AdmissionOutcome::UnknownCapability(capability::CROSS_BUILD_WINDOWS.to_string())
                }
            }
        }
    }
}

// ───────────────────── Native runtime validation gate ──────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeValidationOutcome {
    Admitted {
        lease_id: String,
        environment_id: String,
    },
    /// No host match: a non-target host can never run native validation
    /// (CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE / NO_SUBSTITUTE_TARGET).
    HostDoesNotMatchTarget,
    /// No durable ValidationEnvironment lease exists (BS §79.8). The
    /// truthful state, never a simulated pass.
    NoValidationEnvironmentLease(PlatformCapabilityState),
    EnvironmentNotMatchingTarget,
    EnvironmentUnhealthy,
}

/// `NativeRuntimeValidationAuthority` gate (TA §84.3) — the
/// EvidenceAuthority/completion-evaluator decision point.
///
/// Admission requires, in order: host platform equals target platform; a
/// `ValidationEnvironment` with a durable lease; environment platform and
/// architecture matching the target; healthy environment. Closure (a
/// `Verified` gate result) additionally requires bound target observations
/// — see `validate_gate_evidence`.
pub fn admit_native_validation(
    record: &EnvironmentCapabilityRecord,
    environment: Option<&ValidationEnvironment>,
) -> NativeValidationOutcome {
    if record.host_platform != record.target_platform {
        return NativeValidationOutcome::HostDoesNotMatchTarget;
    }
    let environment = match environment {
        Some(env) if env.lease_id.is_some() => env,
        _ => {
            let state = if record.runtime_validation_available {
                PlatformCapabilityState::UserRequired
            } else {
                PlatformCapabilityState::Unavailable
            };
            return NativeValidationOutcome::NoValidationEnvironmentLease(state);
        }
    };
    if environment.platform != record.target_platform
        || environment.architecture != record.target_architecture
    {
        return NativeValidationOutcome::EnvironmentNotMatchingTarget;
    }
    if environment.health == ValidationEnvironmentHealth::Unhealthy {
        return NativeValidationOutcome::EnvironmentUnhealthy;
    }
    NativeValidationOutcome::Admitted {
        lease_id: environment.lease_id.clone().expect("lease checked above"),
        environment_id: environment.environment_id.clone(),
    }
}

// ──────────────────────── Evidence binding & staleness ─────────────────────

/// A platform runtime observation claim (the unit of evidence a target-host
/// gate must bind, BS §79.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformRuntimeEvidence {
    pub evidence_id: String,
    pub environment_id: String,
    pub environment_fingerprint: String,
    pub target_platform: String,
    pub revision: u64,
    pub observations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceBindingError {
    /// The gate claims `Verified` on a target-host stage but carries no
    /// evidence ids (fixture C: fake completion).
    VerifiedWithoutEvidence,
    /// A referenced evidence id is absent from the supplied evidence set.
    MissingEvidence(String),
    /// Evidence fingerprint differs from the current record (stale).
    FingerprintMismatch,
    /// Evidence target platform differs from the record target.
    TargetPlatformMismatch,
    /// Evidence revision differs from the gate revision (stale).
    StaleRevision,
    /// A target-host stage declares evidence requirements the observations
    /// do not satisfy.
    MissingRequiredObservation(String),
}

impl std::fmt::Display for EvidenceBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceBindingError::VerifiedWithoutEvidence => {
                write!(f, "target-host gate is Verified without bound evidence")
            }
            EvidenceBindingError::MissingEvidence(id) => {
                write!(f, "evidence {id} is not present in the supplied set")
            }
            EvidenceBindingError::FingerprintMismatch => {
                write!(
                    f,
                    "evidence environment fingerprint does not match the current record"
                )
            }
            EvidenceBindingError::TargetPlatformMismatch => {
                write!(
                    f,
                    "evidence target platform does not match the record target platform"
                )
            }
            EvidenceBindingError::StaleRevision => {
                write!(f, "evidence revision does not match the gate revision")
            }
            EvidenceBindingError::MissingRequiredObservation(req) => {
                write!(
                    f,
                    "required target observation {req} is not present in the evidence"
                )
            }
        }
    }
}

/// CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING, made executable.
///
/// Validates a gate record against the current environment record and the
/// evidence that supposedly produced it. A `Verified` target-host gate
/// passes only when every referenced evidence item exists, binds to the
/// current fingerprint, target platform, and revision, and satisfies the
/// stage's declared observation requirements.
pub fn validate_gate_evidence(
    gate: &BuildGateRecord,
    record: &EnvironmentCapabilityRecord,
    evidence: &[PlatformRuntimeEvidence],
    stage_evidence_requirements: &[&str],
) -> Result<(), EvidenceBindingError> {
    if gate.stage.requires_target_host() && gate.result == BuildGateResult::Verified {
        if gate.evidence_ids.is_empty() {
            return Err(EvidenceBindingError::VerifiedWithoutEvidence);
        }
    }
    for id in &gate.evidence_ids {
        let item = evidence.iter().find(|e| &e.evidence_id == id);
        let item = match item {
            Some(item) => item,
            None => return Err(EvidenceBindingError::MissingEvidence(id.clone())),
        };
        if item.environment_fingerprint != record.environment_fingerprint {
            return Err(EvidenceBindingError::FingerprintMismatch);
        }
        if item.target_platform != record.target_platform {
            return Err(EvidenceBindingError::TargetPlatformMismatch);
        }
        if item.revision != gate.revision.0 {
            return Err(EvidenceBindingError::StaleRevision);
        }
        for requirement in stage_evidence_requirements {
            if !item.observations.iter().any(|o| o == requirement) {
                return Err(EvidenceBindingError::MissingRequiredObservation(
                    requirement.to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// TA §84.2 invalidation: the gate's bound evidence is stale when a newer
/// environment record invalidates the record the evidence was produced
/// under (fingerprint, target platform, or toolchain identity changed).
pub fn gate_evidence_is_stale(
    gate: &BuildGateRecord,
    evidence: &[PlatformRuntimeEvidence],
    current_record: &EnvironmentCapabilityRecord,
    producing_record: &EnvironmentCapabilityRecord,
) -> bool {
    if current_record.invalidates_older(producing_record) {
        return true;
    }
    gate.evidence_ids.iter().any(|id| {
        evidence
            .iter()
            .find(|e| &e.evidence_id == id)
            .map(|e| e.environment_fingerprint != current_record.environment_fingerprint)
            .unwrap_or(true)
    })
}

// ─────────────────────── Aggregation & traceability ────────────────────────

/// Truthful aggregate capability status (BS §79.5, §5.6 vocabulary).
///
/// `SUPPORTED` is only reported when the target runtime gate is verified.
/// A verified cross-build whose runtime is unverified or unavailable
/// reports `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS` — never `SUPPORTED`
/// (fixture B, BS §79.13).
pub fn aggregate_capability_status(
    target_build: BuildGateResult,
    runtime_validation: BuildGateResult,
) -> &'static str {
    match (target_build, runtime_validation) {
        (_, BuildGateResult::Verified) => "SUPPORTED",
        (BuildGateResult::Verified, BuildGateResult::Unverified) => {
            "SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS"
        }
        (BuildGateResult::Verified, BuildGateResult::Unavailable) => {
            "SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS"
        }
        (BuildGateResult::Verified, BuildGateResult::UserRequired) => "USER_REQUIRED",
        _ => "DEGRADED",
    }
}

/// Traceability edge types for the M118 extended trace (TA §84.5): the
/// planner emits environment-requirement, capability-resolution, and
/// evidence-binding edges populated from observed records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformTraceEdgeType {
    EnvironmentRequirement,
    CapabilityResolution,
    EvidenceBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformTraceEdge {
    pub edge_type: PlatformTraceEdgeType,
    pub from_ref: String,
    pub to_ref: String,
    pub detail: String,
}

/// Emits the M118 traceability edges for one environment record plus its
/// gates and evidence. Pure and deterministic.
pub fn emit_platform_trace_edges(
    record: &EnvironmentCapabilityRecord,
    gates: &[BuildGateRecord],
    evidence: &[PlatformRuntimeEvidence],
) -> Vec<PlatformTraceEdge> {
    let mut edges = Vec::new();
    edges.push(PlatformTraceEdge {
        edge_type: PlatformTraceEdgeType::EnvironmentRequirement,
        from_ref: "task".into(),
        to_ref: record.environment_id.clone(),
        detail: format!(
            "host={} target={} validation_environment_required={}",
            record.host_platform,
            record.target_platform,
            record.host_platform != record.target_platform,
        ),
    });
    for result in &record.capability_results {
        edges.push(PlatformTraceEdge {
            edge_type: PlatformTraceEdgeType::CapabilityResolution,
            from_ref: record.environment_id.clone(),
            to_ref: result.capability_id.clone(),
            detail: format!("{:?}", result.state),
        });
    }
    for gate in gates {
        for id in &gate.evidence_ids {
            let known = evidence.iter().any(|e| &e.evidence_id == id);
            edges.push(PlatformTraceEdge {
                edge_type: PlatformTraceEdgeType::EvidenceBinding,
                from_ref: gate.gate_id.clone(),
                to_ref: id.clone(),
                detail: if known { "bound" } else { "missing" }.into(),
            });
        }
    }
    edges
}

// ───────────────────────────── Preflight collection ────────────────────────

/// Observed state of one tool from a probe (the §9.2 diagnostic
/// vocabulary, M43-style: a real observation, never an assumption).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolObservation {
    pub state: ToolState,
    pub version: Option<String>,
    pub detail: String,
}

impl ToolObservation {
    pub fn installed(version: &str, detail: &str) -> Self {
        Self {
            state: ToolState::Installed,
            version: Some(version.to_string()),
            detail: detail.to_string(),
        }
    }
    pub fn missing(detail: &str) -> Self {
        Self {
            state: ToolState::Missing,
            version: None,
            detail: detail.to_string(),
        }
    }
    pub fn inaccessible(detail: &str) -> Self {
        Self {
            state: ToolState::Inaccessible,
            version: None,
            detail: detail.to_string(),
        }
    }
}

/// Injectable environment probe (the M43 `CapabilityProbe` pattern):
/// deterministic tests script a fake; `OsProbe` inspects the real host.
/// The planner never probes the OS itself — it consumes these observations,
/// so classification stays a pure function of the probe (BS §79.4).
pub trait PlatformProbe: Send {
    fn host_platform(&self) -> String;
    fn host_architecture(&self) -> String;
    fn observe_tool(&self, name: &str) -> ToolObservation;
    /// Environment variables that participate in the fingerprint. The OS
    /// probe returns none: the fingerprint is over platform, architecture,
    /// and tool versions so it is stable across shells.
    fn fingerprint_env(&self) -> BTreeMap<String, String>;
}

/// FNV-1a 64-bit fingerprint of the environment identity. Deliberately not
/// cryptographic: it exists for equality-based invalidation (TA §84.2), and
/// it must be cheap, deterministic, and readable in logs.
fn fnv1a64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;
    let mut h = OFFSET;
    for byte in data {
        h ^= u64::from(*byte);
        h = h.wrapping_mul(PRIME);
    }
    h
}

pub fn environment_fingerprint(
    host_platform: &str,
    host_architecture: &str,
    tool_versions: &BTreeMap<String, String>,
    fingerprint_env: &BTreeMap<String, String>,
) -> String {
    let mut canonical = String::from("nirman.envfp.v1|");
    canonical.push_str(host_platform);
    canonical.push('|');
    canonical.push_str(host_architecture);
    canonical.push('|');
    for (tool, version) in tool_versions {
        canonical.push_str(tool);
        canonical.push('=');
        canonical.push_str(version);
        canonical.push(';');
    }
    for (key, value) in fingerprint_env {
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(value);
        canonical.push(';');
    }
    format!("{:016x}", fnv1a64(canonical.as_bytes()))
}

/// Probes the real host (the M43 `HostCapabilityProbe` analogue). Version
/// output follows the M43 convention: first non-empty line of stdout, with
/// stderr as fallback (java prints its version to stderr).
pub struct OsProbe;

impl OsProbe {
    fn probe_spec(
        name: &str,
    ) -> Option<(&'static str, &'static [&'static str], Option<&'static str>)> {
        match name {
            "rustc" => Some(("rustc", &["--version"], None)),
            "cargo" => Some(("cargo", &["--version"], None)),
            "node" => Some(("node", &["--version"], None)),
            "tsc" => Some(("tsc", &["--version"], None)),
            "file" => Some(("file", &["--version"], None)),
            "package_manager" => Some(("pnpm", &["--version"], None)),
            "rust_target_windows" => Some((
                "rustup",
                &["target", "list", "--installed"],
                Some("x86_64-pc-windows-gnu"),
            )),
            "windows_linker" => Some(("x86_64-w64-mingw32-gcc", &["--version"], None)),
            "linker" => Some(("x86_64-w64-mingw32-gcc", &["--version"], None)),
            "nsis" => Some(("makensis", &["-VERSION"], None)),
            "wine" => Some(("wine", &["--version"], None)),
            "java" => Some(("java", &["-version"], None)),
            "gradle" => Some(("gradle", &["--version"], None)),
            "android_sdk" => Some(("sdkmanager", &["--version"], None)),
            "emulator" => Some(("emulator", &["-list-avds"], None)),
            "adb" => Some(("adb", &["version"], None)),
            _ => None,
        }
    }

    fn run_spec(program: &str, args: &[&str], requires_substring: Option<&str>) -> ToolObservation {
        let output = std::process::Command::new(program).args(args).output();
        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let text = if stdout.trim().is_empty() {
                    String::from_utf8_lossy(&output.stderr).to_string()
                } else {
                    stdout.to_string()
                };
                if let Some(required) = requires_substring {
                    if !text.contains(required) {
                        return ToolObservation::missing(&format!(
                            "installed set does not include {required}"
                        ));
                    }
                }
                let version = text.lines().next().unwrap_or("unknown").trim().to_string();
                ToolObservation::installed(&version, "host-observed")
            }
            Ok(_) => ToolObservation::missing("tool command returned a non-success status"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ToolObservation::missing("tool command is not installed or not discoverable")
            }
            Err(_) => ToolObservation::inaccessible("tool command could not be inspected"),
        }
    }
}

impl PlatformProbe for OsProbe {
    fn host_platform(&self) -> String {
        std::env::consts::OS.to_string()
    }

    fn host_architecture(&self) -> String {
        std::env::consts::ARCH.to_string()
    }

    fn observe_tool(&self, name: &str) -> ToolObservation {
        match name {
            "windows_os" | "dpapi" => {
                if self.host_platform() == "windows" {
                    ToolObservation::installed("native", "host platform is windows")
                } else {
                    ToolObservation::missing(
                        "host platform is not windows; no substitute target is accepted",
                    )
                }
            }
            "device" => {
                let adb = Self::run_spec("adb", &["devices"], None);
                if adb.state != ToolState::Installed {
                    return ToolObservation::missing(
                        "adb is not available, so no device can be attached",
                    );
                }
                let attached = std::process::Command::new("adb")
                    .arg("devices")
                    .output()
                    .ok()
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .filter(|l| l.ends_with("device"))
                            .count()
                    })
                    .unwrap_or(0);
                if attached > 0 {
                    ToolObservation::installed(
                        &format!("{attached} attached"),
                        "adb device session available",
                    )
                } else {
                    ToolObservation::missing("adb is present but no device is attached")
                }
            }
            _ => match Self::probe_spec(name) {
                Some((program, args, substring)) => Self::run_spec(program, args, substring),
                None => ToolObservation::missing("no probe spec for this tool in this environment"),
            },
        }
    }

    fn fingerprint_env(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PreflightError {
    UndeclaredTarget(String),
    UnknownHostPlatform(String),
    EmptyPlatform,
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreflightError::UndeclaredTarget(t) => {
                write!(
                    f,
                    "declared target platform is not in the product scope: {t}"
                )
            }
            PreflightError::UnknownHostPlatform(p) => {
                write!(
                    f,
                    "the canonical capability matrix does not cover host platform: {p}"
                )
            }
            PreflightError::EmptyPlatform => write!(f, "platform identifier is empty"),
        }
    }
}

/// `EnvironmentCapabilityPlanner` (TA §58.8, §84.1): resolves host and
/// target, consults the matrix as a prior, and classifies every capability
/// for the host from probe observations. It never derives native runtime
/// capability from a build or cross-build result, and a classification is
/// never raised by model assertion.
pub struct EnvironmentCapabilityPlanner {
    probe: Box<dyn PlatformProbe>,
    registry: PlatformCapabilityRegistry,
}

impl EnvironmentCapabilityPlanner {
    pub fn new(probe: Box<dyn PlatformProbe>, registry: PlatformCapabilityRegistry) -> Self {
        Self { probe, registry }
    }

    pub fn registry(&self) -> &PlatformCapabilityRegistry {
        &self.registry
    }

    pub fn run(
        &self,
        declared_target: &str,
        target_architecture: &str,
    ) -> Result<EnvironmentCapabilityRecord, PreflightError> {
        let host_platform = self.probe.host_platform();
        let host_architecture = self.probe.host_architecture();
        if host_platform.trim().is_empty() || host_architecture.trim().is_empty() {
            return Err(PreflightError::EmptyPlatform);
        }
        if !self
            .registry
            .entries
            .iter()
            .any(|e| e.host_platform == host_platform)
        {
            return Err(PreflightError::UnknownHostPlatform(host_platform));
        }
        let resolved = TargetPlatformResolver::resolve(
            &host_platform,
            &host_architecture,
            declared_target,
            target_architecture,
        )
        .map_err(|e| match e {
            TargetResolutionError::UndeclaredTarget(t) => PreflightError::UndeclaredTarget(t),
            TargetResolutionError::EmptyPlatform => PreflightError::EmptyPlatform,
        })?;

        let mut tool_cache: BTreeMap<String, ToolObservation> = BTreeMap::new();
        let mut capability_results = Vec::new();
        let mut user_actions = Vec::new();

        for entry in self
            .registry
            .entries
            .iter()
            .filter(|e| e.host_platform == host_platform)
        {
            let tool_states: BTreeMap<String, ToolState> = entry
                .required_toolchain
                .iter()
                .map(|tool| {
                    let observation = tool_cache
                        .entry(tool.clone())
                        .or_insert_with(|| self.probe.observe_tool(tool));
                    (tool.clone(), observation.state)
                })
                .collect();
            let state = classify_capability(entry, &tool_states);
            if matches!(
                state,
                PlatformCapabilityState::UserRequired | PlatformCapabilityState::Unavailable
            ) {
                user_actions.push(format!(
                    "capability {}: {} (prior: {:?}; required tools: {})",
                    entry.capability_id,
                    state,
                    entry.expected_result,
                    entry.required_toolchain.join(", ")
                ));
            }
            capability_results.push(EnvironmentCapabilityResult {
                capability_id: entry.capability_id.clone(),
                state,
                observed_version: None,
                path: None,
                fingerprint: None,
                detail: format!(
                    "matrix v{} prior {:?}",
                    entry.matrix_version, entry.expected_result
                ),
                evidence_id: format!("pf-{}", entry.capability_id),
            });
        }

        let tool_versions: BTreeMap<String, String> = tool_cache
            .iter()
            .filter(|(_, obs)| obs.state == ToolState::Installed)
            .filter_map(|(tool, obs)| obs.version.clone().map(|v| (tool.clone(), v)))
            .collect();
        let installed_tools: Vec<String> = tool_cache
            .iter()
            .filter(|(_, obs)| obs.state == ToolState::Installed)
            .map(|(tool, _)| tool.clone())
            .collect();

        let cross = resolved.is_cross_build();
        let cross_state = capability_results
            .iter()
            .find(|r| r.capability_id == capability::CROSS_BUILD_WINDOWS)
            .map(|r| r.state)
            .unwrap_or(PlatformCapabilityState::Unavailable);
        let native_state = capability_results
            .iter()
            .find(|r| r.capability_id == capability::WINDOWS_NATIVE_EXECUTION)
            .map(|r| r.state)
            .unwrap_or(PlatformCapabilityState::Unavailable);
        let fingerprint = environment_fingerprint(
            &host_platform,
            &host_architecture,
            &tool_versions,
            &self.probe.fingerprint_env(),
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(EnvironmentCapabilityRecord {
            schema_version: EnvironmentCapabilityRecord::SCHEMA_VERSION,
            environment_id: format!("env-{host_platform}-{fingerprint}"),
            host_platform: host_platform.clone(),
            host_architecture: host_architecture.clone(),
            target_platform: resolved.target_platform.clone(),
            target_architecture: resolved.target_architecture.clone(),
            shell: if host_platform == "windows" {
                "powershell".into()
            } else {
                "sh".into()
            },
            compiler: tool_versions.get("rustc").cloned().unwrap_or_default(),
            linker: tool_versions
                .get("windows_linker")
                .cloned()
                .or_else(|| tool_versions.get("linker").cloned())
                .unwrap_or_default(),
            sdk: tool_versions
                .get("android_sdk")
                .cloned()
                .unwrap_or_default(),
            runtime: tool_versions.get("node").cloned().unwrap_or_default(),
            build_tools: installed_tools
                .iter()
                .filter(|t| matches!(t.as_str(), "rustc" | "cargo" | "node" | "gradle" | "java"))
                .cloned()
                .collect(),
            installer_tools: installed_tools
                .iter()
                .filter(|t| matches!(t.as_str(), "nsis" | "wine"))
                .cloned()
                .collect(),
            native_dependencies: vec![],
            tool_versions,
            environment_fingerprint: fingerprint,
            capability_results,
            repair_attempts: vec![],
            required_user_actions: user_actions,
            runtime_validation_available: !cross
                && native_state == PlatformCapabilityState::Available,
            cross_compilation_available: cross && cross_state == PlatformCapabilityState::Available,
            evidence_ids: vec![],
            recorded_at_epoch_seconds: now,
            supersedes: None,
        })
    }
}

// ─────────────────────────── Worker scheduling ─────────────────────────────

/// BS §79.12 / TA §84.5: the scheduler, not the worker, refuses a worker
/// whose platform requirements the current environment record does not
/// satisfy.
pub fn worker_satisfies_platform_requirements(
    requirements: &PlatformRequirements,
    record: &EnvironmentCapabilityRecord,
) -> bool {
    if !requirements.required_host_platforms.is_empty()
        && !requirements
            .required_host_platforms
            .contains(&record.host_platform)
    {
        return false;
    }
    if !requirements.required_target_platforms.is_empty()
        && !requirements
            .required_target_platforms
            .contains(&record.target_platform)
    {
        return false;
    }
    if !requirements.required_architectures.is_empty()
        && !requirements
            .required_architectures
            .contains(&record.host_architecture)
    {
        return false;
    }
    for capability_id in &requirements.required_capabilities {
        match record.capability_state(capability_id) {
            Some(PlatformCapabilityState::Available) => {}
            _ => return false,
        }
    }
    for tool in &requirements.required_toolchain {
        if !record.build_tools.iter().any(|t| t == tool) {
            return false;
        }
    }
    if requirements.native_execution_required {
        if record.host_platform != record.target_platform || !record.runtime_validation_available {
            return false;
        }
    }
    true
}

// ─────────────────────────────────── Tests ─────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_domain::{BuildGateStage, EnvironmentCapabilityResult, Revision};

    fn cross_result(available: bool) -> EnvironmentCapabilityResult {
        EnvironmentCapabilityResult {
            capability_id: capability::CROSS_BUILD_WINDOWS.into(),
            state: if available {
                PlatformCapabilityState::Available
            } else {
                PlatformCapabilityState::Repairable
            },
            observed_version: None,
            path: None,
            fingerprint: None,
            detail: String::new(),
            evidence_id: "ev-cross".into(),
        }
    }

    fn native_unavailable_result() -> EnvironmentCapabilityResult {
        EnvironmentCapabilityResult {
            capability_id: capability::WINDOWS_NATIVE_EXECUTION.into(),
            state: PlatformCapabilityState::Unavailable,
            observed_version: None,
            path: None,
            fingerprint: None,
            detail: "no windows host".into(),
            evidence_id: "ev-native".into(),
        }
    }

    fn linux_to_windows_record(cross_available: bool) -> EnvironmentCapabilityRecord {
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
            runtime: "node 20".into(),
            build_tools: vec!["cargo".into()],
            installer_tools: vec![],
            native_dependencies: vec![],
            tool_versions: BTreeMap::from([("rustc".to_string(), "1.98".to_string())]),
            environment_fingerprint: "fp-linux-1".into(),
            capability_results: vec![cross_result(cross_available), native_unavailable_result()],
            repair_attempts: vec![],
            required_user_actions: vec![],
            runtime_validation_available: false,
            cross_compilation_available: cross_available,
            evidence_ids: vec![],
            recorded_at_epoch_seconds: 1_700_000_000,
            supersedes: None,
        }
    }

    #[test]
    fn resolver_rejects_undeclared_targets() {
        let err =
            TargetPlatformResolver::resolve("linux", "x86_64", "macos", "aarch64").unwrap_err();
        assert_eq!(err, TargetResolutionError::UndeclaredTarget("macos".into()));
        let ok = TargetPlatformResolver::resolve("linux", "x86_64", "windows", "x86_64").unwrap();
        assert!(ok.is_cross_build());
        assert_eq!(ok.target_platform, "windows");
    }

    #[test]
    fn matrix_encodes_platform_impossibility_not_tool_unavailability() {
        let matrix = PlatformCapabilityRegistry::canonical_v1();
        assert_eq!(matrix.matrix_version(), MATRIX_VERSION_V1);
        assert_eq!(
            matrix
                .entry("linux", capability::WINDOWS_CONPTY)
                .unwrap()
                .expected_result,
            MatrixExpectedResult::UnavailableByPlatform
        );
        assert_eq!(
            matrix
                .entry("windows", capability::WINDOWS_CONPTY)
                .unwrap()
                .expected_result,
            MatrixExpectedResult::Available
        );
        assert_eq!(
            matrix
                .entry("linux", capability::CROSS_BUILD_WINDOWS)
                .unwrap()
                .expected_result,
            MatrixExpectedResult::EnvironmentDependent
        );
        assert!(matrix.entry("linux", "unknown_capability").is_none());
    }

    #[test]
    fn classification_is_deterministic_and_observation_driven() {
        let matrix = PlatformCapabilityRegistry::canonical_v1();
        let cross = matrix
            .entry("linux", capability::CROSS_BUILD_WINDOWS)
            .unwrap();
        let installed = BTreeMap::from([
            ("rust_target_windows".to_string(), ToolState::Installed),
            ("windows_linker".to_string(), ToolState::Installed),
        ]);
        assert_eq!(
            classify_capability(cross, &installed),
            PlatformCapabilityState::Available
        );
        let missing = BTreeMap::from([
            ("rust_target_windows".to_string(), ToolState::Missing),
            ("windows_linker".to_string(), ToolState::Installed),
        ]);
        assert_eq!(
            classify_capability(cross, &missing),
            PlatformCapabilityState::Repairable
        );
        // A required tool absent from the observation is not credited.
        assert_eq!(
            classify_capability(cross, &BTreeMap::new()),
            PlatformCapabilityState::Repairable
        );
        let conpty = matrix.entry("linux", capability::WINDOWS_CONPTY).unwrap();
        let locked = BTreeMap::from([("windows_os".to_string(), ToolState::Inaccessible)]);
        assert_eq!(
            classify_capability(conpty, &locked),
            PlatformCapabilityState::Unavailable,
            "a platform impossibility is unavailable regardless of observation"
        );
        let deps = matrix
            .entry("linux", capability::DEPENDENCY_INSTALLATION)
            .unwrap();
        let deps_locked =
            BTreeMap::from([("package_manager".to_string(), ToolState::Inaccessible)]);
        assert_eq!(
            classify_capability(deps, &deps_locked),
            PlatformCapabilityState::UserRequired
        );
    }

    #[test]
    fn cross_build_admitted_only_with_proven_toolchain_and_never_admits_runtime() {
        let record = linux_to_windows_record(true);
        assert!(matches!(
            admit_target_operation(CommandOperation::TargetBuild, &record, true),
            AdmissionOutcome::Admitted { .. }
        ));
        let repairable = linux_to_windows_record(false);
        assert_eq!(
            admit_target_operation(CommandOperation::TargetBuild, &repairable, true),
            AdmissionOutcome::ToolchainNotProven(PlatformCapabilityState::Repairable)
        );
        assert_eq!(
            admit_target_operation(CommandOperation::TargetBuild, &record, false),
            AdmissionOutcome::CrossCompilationNotAllowedForContract
        );
        // The target-mismatch guard: a runtime-validation claim from a
        // non-matching host is rejected before execution, no matter what
        // the model says.
        assert_eq!(
            admit_target_operation(CommandOperation::RuntimeValidation, &record, true),
            AdmissionOutcome::RuntimeValidationClaimOnNonMatchingHost
        );
    }

    #[test]
    fn native_validation_requires_matching_lease_and_never_simulates() {
        let record = linux_to_windows_record(true);
        assert_eq!(
            admit_native_validation(&record, None),
            NativeValidationOutcome::HostDoesNotMatchTarget
        );
        let same_host = EnvironmentCapabilityRecord {
            environment_id: "env-win-1".into(),
            host_platform: "windows".into(),
            target_platform: "windows".into(),
            runtime_validation_available: false,
            capability_results: vec![],
            ..record.clone()
        };
        let no_lease = ValidationEnvironment {
            environment_id: "val-1".into(),
            lease_id: None,
            ..validation_env("windows", "x86_64")
        };
        assert_eq!(
            admit_native_validation(&same_host, Some(&no_lease)),
            NativeValidationOutcome::NoValidationEnvironmentLease(
                PlatformCapabilityState::Unavailable
            )
        );
        let leased = ValidationEnvironment {
            lease_id: Some("lease-1".into()),
            reserved_by_task: Some("task-1".into()),
            acquired_at_epoch_seconds: Some(1_700_000_001),
            ..validation_env("windows", "x86_64")
        };
        assert!(matches!(
            admit_native_validation(&same_host, Some(&leased)),
            NativeValidationOutcome::Admitted { .. }
        ));
        let mismatched = ValidationEnvironment {
            platform: "linux".into(),
            lease_id: Some("lease-2".into()),
            ..validation_env("windows", "x86_64")
        };
        assert_eq!(
            admit_native_validation(&same_host, Some(&mismatched)),
            NativeValidationOutcome::EnvironmentNotMatchingTarget
        );
    }

    fn validation_env(platform: &str, arch: &str) -> ValidationEnvironment {
        ValidationEnvironment {
            schema_version: ValidationEnvironment::SCHEMA_VERSION,
            environment_id: "val-env".into(),
            platform: platform.into(),
            architecture: arch.into(),
            toolchain: "windows-sdk".into(),
            runtime: "windows".into(),
            available_tools: vec!["powershell".into()],
            available_devices: vec![],
            isolation_profile: "standard".into(),
            network_policy: "local".into(),
            fingerprint: "fp-win-val".into(),
            health: ValidationEnvironmentHealth::Healthy,
            lease_id: None,
            reserved_by_task: None,
            acquired_at_epoch_seconds: None,
            released_at_epoch_seconds: None,
        }
    }

    #[test]
    fn verified_target_gate_without_evidence_is_rejected() {
        let record = linux_to_windows_record(true);
        let gate = BuildGateRecord {
            schema_version: BuildGateRecord::SCHEMA_VERSION,
            gate_id: "gate-runtime".into(),
            stage: BuildGateStage::RuntimeValidation,
            platform: "windows".into(),
            environment_id: record.environment_id.clone(),
            revision: Revision(1),
            command_or_operation_ref: "model-claim".into(),
            evidence_ids: vec![],
            result: BuildGateResult::Verified,
            recorded_at_epoch_seconds: 1_700_000_002,
        };
        let err = validate_gate_evidence(&gate, &record, &[], &["process_launch_observation"])
            .unwrap_err();
        assert_eq!(err, EvidenceBindingError::VerifiedWithoutEvidence);
    }

    #[test]
    fn stale_evidence_is_invalidated_by_fingerprint_or_revision_change() {
        let producing = linux_to_windows_record(true);
        let gate = BuildGateRecord {
            schema_version: BuildGateRecord::SCHEMA_VERSION,
            gate_id: "gate-runtime".into(),
            stage: BuildGateStage::RuntimeValidation,
            platform: "windows".into(),
            environment_id: producing.environment_id.clone(),
            revision: Revision(1),
            command_or_operation_ref: "op-1".into(),
            evidence_ids: vec!["ev-1".into()],
            result: BuildGateResult::Verified,
            recorded_at_epoch_seconds: 1_700_000_002,
        };
        let evidence = vec![PlatformRuntimeEvidence {
            evidence_id: "ev-1".into(),
            environment_id: producing.environment_id.clone(),
            environment_fingerprint: producing.environment_fingerprint.clone(),
            target_platform: "windows".into(),
            revision: 1,
            observations: vec!["process_launch_observation".into()],
        }];
        // Current record identical to the producing one: valid.
        let current = producing.clone();
        assert!(validate_gate_evidence(
            &gate,
            &current,
            &evidence,
            &["process_launch_observation"]
        )
        .is_ok());
        assert!(!gate_evidence_is_stale(
            &gate, &evidence, &current, &producing
        ));

        // Newer observation with a changed fingerprint: stale.
        let drifted = EnvironmentCapabilityRecord {
            environment_fingerprint: "fp-linux-2".into(),
            ..producing.clone()
        };
        assert!(gate_evidence_is_stale(
            &gate, &evidence, &drifted, &producing
        ));
        let err =
            validate_gate_evidence(&gate, &drifted, &evidence, &["process_launch_observation"])
                .unwrap_err();
        assert_eq!(err, EvidenceBindingError::FingerprintMismatch);

        // Revision moved: stale.
        let gate_rev2 = BuildGateRecord {
            revision: Revision(2),
            ..gate.clone()
        };
        assert_eq!(
            validate_gate_evidence(
                &gate_rev2,
                &current,
                &evidence,
                &["process_launch_observation"]
            )
            .unwrap_err(),
            EvidenceBindingError::StaleRevision
        );
    }

    #[test]
    fn scheduler_refuses_worker_on_non_matching_host() {
        let record = linux_to_windows_record(true);
        let native = PlatformRequirements {
            required_host_platforms: vec!["windows".into()],
            required_target_platforms: vec!["windows".into()],
            native_execution_required: true,
            ..Default::default()
        };
        assert!(!worker_satisfies_platform_requirements(&native, &record));
        let cross = PlatformRequirements {
            required_capabilities: vec![capability::CROSS_BUILD_WINDOWS.into()],
            cross_compilation_allowed: true,
            ..Default::default()
        };
        assert!(worker_satisfies_platform_requirements(&cross, &record));
        let cross_broken = PlatformRequirements {
            required_capabilities: vec![capability::WINDOWS_CONPTY.into()],
            ..Default::default()
        };
        assert!(!worker_satisfies_platform_requirements(
            &cross_broken,
            &record
        ));
    }
}
