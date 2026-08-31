#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub const SKILL_PACKAGE_SCHEMA: &str = "nirman.skill_package.v1";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackage {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub scope: SkillScope,
    pub compatible_worker_roles: Vec<String>,
    pub trigger_conditions: Vec<String>,
    pub required_tools: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub permission_requests: Vec<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub source_path: String,
    pub scan_status: ScanStatus,
    pub trust_status: TrustStatus,
    pub enabled: bool,
    pub installed_at: u64,
    pub last_used_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillScope {
    BuiltIn,
    User,
    Project,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanStatus {
    Pending,
    Scanning,
    Clean,
    Suspicious,
    Failed,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustStatus {
    Untrusted,
    PendingReview,
    Trusted,
    Revoked,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillInvocationRecord {
    pub invocation_id: String,
    pub skill_id: String,
    pub skill_version: String,
    pub session_id: String,
    pub requested_by: String,
    pub trigger_reason: String,
    pub granted_permissions: Vec<String>,
    pub tool_calls: Vec<SkillToolCall>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub status: SkillInvocationStatus,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillInvocationStatus {
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillToolCall {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub policy_outcome: String,
}

impl SkillPackage {
    pub fn is_permission_neutral(&self) -> bool {
        self.permission_requests.is_empty()
    }

    pub fn is_scanned(&self) -> bool {
        matches!(
            self.scan_status,
            ScanStatus::Clean | ScanStatus::Failed | ScanStatus::Unknown
        )
    }

    pub fn is_invocable(&self) -> bool {
        self.enabled && self.is_scanned() && !matches!(self.trust_status, TrustStatus::Revoked)
    }

    pub fn declared_tools(&self) -> Vec<String> {
        self.required_tools.clone()
    }
}

/// Errors loading or validating built-in skill packages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillLoadError {
    Io(String),
    Manifest { skill_dir: String, message: String },
    SchemaViolation { skill_id: String, message: String },
}

impl std::fmt::Display for SkillLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillLoadError::Io(message) => write!(f, "skill load io error: {message}"),
            SkillLoadError::Manifest { skill_dir, message } => {
                write!(f, "manifest error in {skill_dir}: {message}")
            }
            SkillLoadError::SchemaViolation { skill_id, message } => {
                write!(f, "schema violation for {skill_id}: {message}")
            }
        }
    }
}

/// Loads the built-in platform skill packages from `<root>/skills`
/// (BS §79.7 v1 set). Each package directory holds a `skill.json`
/// manifest plus its `SKILL.md` instruction body. The v1 set is
/// per-platform by design: a generic catch-all coding skill is
/// prohibited and is rejected here if it appears.
pub fn load_builtin_skill_packages(
    root: &std::path::Path,
) -> Result<Vec<SkillPackage>, SkillLoadError> {
    let skills_root = root.join("skills");
    let mut packages = Vec::new();
    let mut stack = vec![skills_root.clone()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|error| SkillLoadError::Io(error.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|error| SkillLoadError::Io(error.to_string()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some("skill.json") {
                let skill_dir = path
                    .parent()
                    .expect("skill.json always has a parent directory");
                let dir_name = skill_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("directory name is valid unicode");
                let raw = std::fs::read_to_string(&path)
                    .map_err(|error| SkillLoadError::Io(error.to_string()))?;
                let package: SkillPackage =
                    serde_json::from_str(&raw).map_err(|error| SkillLoadError::Manifest {
                        skill_dir: dir_name.to_string(),
                        message: error.to_string(),
                    })?;
                validate_builtin_package(&package, dir_name)?;
                packages.push(package);
            }
        }
    }
    packages.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    Ok(packages)
}

fn validate_builtin_package(package: &SkillPackage, dir_name: &str) -> Result<(), SkillLoadError> {
    let violation = |message: String| SkillLoadError::SchemaViolation {
        skill_id: package.skill_id.clone(),
        message,
    };
    if package.skill_id != dir_name {
        return Err(violation(format!(
            "skill_id {} does not match its directory name {dir_name}",
            package.skill_id
        )));
    }
    if package.scope != SkillScope::BuiltIn {
        return Err(violation(format!(
            "built-in skills must have scope BUILT_IN, found {:?}",
            package.scope
        )));
    }
    if package.version.is_empty() {
        return Err(violation("version must not be empty".into()));
    }
    if !package.source_path.starts_with("builtin/")
        || !package.source_path.ends_with(&package.skill_id)
    {
        return Err(violation(format!(
            "source_path {} must be of the form builtin/.../{}",
            package.source_path, package.skill_id
        )));
    }
    let lower = package.skill_id.to_lowercase().replace('_', " ");
    let name_lower = package.name.to_lowercase();
    let generic = [
        "universal",
        "general",
        "mega",
        "catch-all",
        "all-in-one",
        "general-purpose",
    ]
    .iter()
    .any(|token| lower.contains(token) || name_lower.contains(token));
    if package
        .skill_id
        .eq_ignore_ascii_case("universal-coding-skill")
        || generic
    {
        return Err(violation(
            "a generic catch-all coding skill is prohibited (BS §79.7): platform behavior must be              carried by dedicated per-platform skill packages"
                .into(),
        ));
    }
    Ok(())
}

/// The admission result for invoking a skill's gated steps under the
/// current environment record (BS §79.7): a skill whose
/// `requiredCapabilities` resolve to UNAVAILABLE or USER_REQUIRED MUST
/// NOT execute the gated steps and MUST report the blocked state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillAdmission {
    Admitted,
    Blocked {
        blocked_capabilities: Vec<(String, nirman_domain::PlatformCapabilityState)>,
        state: nirman_domain::PlatformCapabilityState,
        reason: String,
    },
}

/// Deterministic skill admission against an observed environment record.
/// Capability ids that the record does not cover are treated as
/// unproven and block fail-closed: a skill never assumes a capability it
/// did not observe, and a model never decides the outcome.
pub fn evaluate_skill_admission(
    package: &SkillPackage,
    record: &nirman_domain::EnvironmentCapabilityRecord,
) -> SkillAdmission {
    let mut unavailable: Vec<(String, nirman_domain::PlatformCapabilityState)> = Vec::new();
    let mut user_required: Vec<(String, nirman_domain::PlatformCapabilityState)> = Vec::new();
    for capability in &package.required_capabilities {
        match record.capability_state(capability) {
            Some(nirman_domain::PlatformCapabilityState::Available) => {}
            Some(state) if state == nirman_domain::PlatformCapabilityState::Unavailable => {
                unavailable.push((capability.clone(), state));
            }
            Some(state) => {
                user_required.push((capability.clone(), state));
            }
            None => {
                unavailable.push((
                    capability.clone(),
                    nirman_domain::PlatformCapabilityState::Unavailable,
                ));
            }
        }
    }
    if unavailable.is_empty() && user_required.is_empty() {
        return SkillAdmission::Admitted;
    }
    let (state, blocked_capabilities) = if !unavailable.is_empty() {
        (
            nirman_domain::PlatformCapabilityState::Unavailable,
            unavailable,
        )
    } else {
        (
            nirman_domain::PlatformCapabilityState::UserRequired,
            user_required,
        )
    };
    let reason = format!(
        "skill {} gated steps must not execute under environment {}: required capability state is not available: {}",
        package.skill_id,
        record.environment_id,
        blocked_capabilities
            .iter()
            .map(|(id, state)| format!("{id}={state}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    SkillAdmission::Blocked {
        blocked_capabilities,
        state,
        reason,
    }
}

/// The outcome of resolving one requested skill id against the registry
/// and the current environment record (TA §19.1 selection, BS §79.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSelectionOutcome {
    /// The skill package is registered, invocable, and its gated steps are
    /// admissible under the environment record.
    Admitted { package: SkillPackage },
    /// No registered package carries the requested id.
    NotFound {
        requested: String,
        available: Vec<String>,
    },
    /// Registered, but not invocable (disabled, unscanned, or untrusted).
    NotInvocable {
        package: SkillPackage,
        reason: String,
    },
    /// Invocable, but the required capabilities are not AVAILABLE in the
    /// environment record (or no record exists for a capability-bearing
    /// skill): the gated steps must not execute.
    Blocked {
        package: SkillPackage,
        admission: SkillAdmission,
    },
}

/// Monotonic version key for skill package versions ("1.0.0" style);
/// unknown segments sort first, so a parseable version always wins.
fn version_key(version: &str) -> (u64, u64, u64) {
    let parts: Vec<u64> = version
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect();
    let part = |index: usize| parts.get(index).copied().unwrap_or(0);
    (part(0), part(1), part(2))
}

/// Deterministic skill selection for a worker contract (TA §19.1, BS
/// §79.7): each requested id resolves to the highest registered version;
/// the package must be invocable, and its required capabilities must be
/// AVAILABLE in the environment record. Requested ids are deduplicated
/// while preserving order. A missing record blocks any capability-bearing
/// skill fail-closed; capability-free skills (e.g. environment-preflight)
/// are admitted without one. The model never decides an outcome.
pub fn select_required_skills(
    requested: &[String],
    registry: &[SkillPackage],
    record: Option<&nirman_domain::EnvironmentCapabilityRecord>,
) -> Vec<(String, SkillSelectionOutcome)> {
    let mut seen: Vec<String> = Vec::new();
    for id in requested {
        if !seen.contains(id) {
            seen.push(id.clone());
        }
    }
    let available: Vec<String> = registry
        .iter()
        .map(|package| package.skill_id.clone())
        .collect();
    seen
        .iter()
        .map(|id| {
            let candidates: Vec<&SkillPackage> = registry
                .iter()
                .filter(|package| package.skill_id == *id)
                .collect();
            match candidates.iter().max_by_key(|package| version_key(&package.version)) {
                None => (
                    id.clone(),
                    SkillSelectionOutcome::NotFound {
                        requested: id.clone(),
                        available: available.clone(),
                    },
                ),
                Some(&package) => {
                    let outcome = if !package.enabled {
                        SkillSelectionOutcome::NotInvocable {
                            package: package.clone(),
                            reason: "the skill package is disabled".into(),
                        }
                    } else if !package.is_scanned() {
                        SkillSelectionOutcome::NotInvocable {
                            package: package.clone(),
                            reason: format!(
                                "the skill package scan status is {:?} (a clean scan is required before activation)",
                                package.scan_status
                            ),
                        }
                    } else if package.trust_status == TrustStatus::Revoked {
                        SkillSelectionOutcome::NotInvocable {
                            package: package.clone(),
                            reason: "the skill package trust has been revoked".into(),
                        }
                    } else if let Some(record) = record {
                        match evaluate_skill_admission(package, record) {
                            SkillAdmission::Admitted => {
                                SkillSelectionOutcome::Admitted {
                                    package: package.clone(),
                                }
                            }
                            SkillAdmission::Blocked { .. } => {
                                let admission = evaluate_skill_admission(package, record);
                                SkillSelectionOutcome::Blocked {
                                    package: package.clone(),
                                    admission,
                                }
                            }
                        }
                    } else if package.required_capabilities.is_empty() {
                        SkillSelectionOutcome::Admitted {
                            package: package.clone(),
                        }
                    } else {
                        let admission = SkillAdmission::Blocked {
                            blocked_capabilities: package
                                .required_capabilities
                                .iter()
                                .map(|id| {
                                    (
                                        id.clone(),
                                        nirman_domain::PlatformCapabilityState::Unavailable,
                                    )
                                })
                                .collect(),
                            state: nirman_domain::PlatformCapabilityState::Unavailable,
                            reason: format!(
                                "skill {} gated steps must not execute: no environment capability record exists for this task, so the required capabilities are unproven (the contract must declare platform requirements so the preflight record exists)",
                                package.skill_id
                            ),
                        };
                        SkillSelectionOutcome::Blocked {
                            package: package.clone(),
                            admission,
                        }
                    };
                    (id.clone(), outcome)
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_skill() -> SkillPackage {
        SkillPackage {
            skill_id: "skill-1".into(),
            name: "Demo".into(),
            description: "A demo skill".into(),
            version: "1.0.0".into(),
            scope: SkillScope::BuiltIn,
            compatible_worker_roles: vec!["test".into()],
            trigger_conditions: vec!["demo".into()],
            required_tools: vec![],
            required_capabilities: vec![],
            permission_requests: vec![],
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "object"}),
            source_path: "/builtin/demo".into(),
            scan_status: ScanStatus::Clean,
            trust_status: TrustStatus::Trusted,
            enabled: true,
            installed_at: 0,
            last_used_at: None,
        }
    }

    #[test]
    fn skill_package_round_trips_serde() {
        let skill = base_skill();
        let json = serde_json::to_string(&skill).expect("serialize");
        let back: SkillPackage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(skill, back);
    }

    #[test]
    fn permission_neutral_skill_is_invocable() {
        let skill = base_skill();
        assert!(skill.is_permission_neutral());
        assert!(skill.is_scanned());
        assert!(skill.is_invocable());
    }

    #[test]
    fn revoked_skill_is_not_invocable() {
        let mut skill = base_skill();
        assert!(skill.is_invocable());
        skill.trust_status = TrustStatus::Revoked;
        assert!(!skill.is_invocable());
    }

    #[test]
    fn unscanned_skill_is_not_invocable() {
        let mut skill = base_skill();
        skill.scan_status = ScanStatus::Pending;
        assert!(!skill.is_invocable());
    }

    #[test]
    fn invocation_record_round_trips_serde() {
        let record = SkillInvocationRecord {
            invocation_id: "inv-1".into(),
            skill_id: "skill-1".into(),
            skill_version: "1.0.0".into(),
            session_id: "session-1".into(),
            requested_by: "user".into(),
            trigger_reason: "explicit".into(),
            granted_permissions: vec![],
            tool_calls: vec![],
            started_at: 0,
            completed_at: Some(1),
            status: SkillInvocationStatus::Completed,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: SkillInvocationRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, back);
    }
}
