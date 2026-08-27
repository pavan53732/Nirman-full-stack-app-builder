#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

pub const M49_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_percent: u8,
    pub memory_mb: u64,
    pub disk_free_mb: u64,
    pub checkpoint_storage_mb: u64,
    pub emulator_memory_mb: u64,
    pub gradle_memory_mb: u64,
    pub worker_concurrency: u16,
    pub provider_concurrency: u16,
    pub context_tokens: u64,
    pub log_volume_mb: u64,
    pub build_duration_seconds: u64,
    pub device_slots_used: u16,
    pub device_slots_total: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_cpu_percent: Option<u8>,
    pub max_memory_mb: Option<u64>,
    pub min_disk_free_mb: Option<u64>,
    pub max_checkpoint_storage_mb: Option<u64>,
    pub max_emulator_memory_mb: Option<u64>,
    pub max_gradle_memory_mb: Option<u64>,
    pub max_worker_concurrency: Option<u16>,
    pub max_provider_concurrency: Option<u16>,
    pub max_context_tokens: Option<u64>,
    pub max_log_volume_mb: Option<u64>,
    pub max_build_duration_seconds: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourcePressure {
    None,
    Elevated,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceAction {
    NoChange,
    CompactContext,
    ReduceWorkerConcurrency { to: u16 },
    ReduceProviderConcurrency { to: u16 },
    PruneSafeCaches,
    StopRedundantWorkers,
    SelectAffectedTests,
    DeferNonessentialVisualChecks,
    SwitchToApprovedLighterProvider,
    BlockNewWorkForProtection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceGovernorDecision {
    pub schema_version: u16,
    pub decision_id: String,
    pub pressure: ResourcePressure,
    pub triggered_limits: Vec<String>,
    pub actions: Vec<ResourceAction>,
    pub safety_gates_preserved: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceGovernorError {
    UnsupportedSchemaVersion,
    InvalidSnapshot(&'static str),
    InvalidBudget(&'static str),
    EmptyDecisionId,
}
impl fmt::Display for ResourceGovernorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion => f.write_str("M49 resource schema is unsupported"),
            Self::InvalidSnapshot(v) => write!(f, "M49 resource snapshot is invalid: {v}"),
            Self::InvalidBudget(v) => write!(f, "M49 resource budget is invalid: {v}"),
            Self::EmptyDecisionId => f.write_str("M49 resource decision id is empty"),
        }
    }
}
impl std::error::Error for ResourceGovernorError {}

impl ResourceSnapshot {
    pub fn validate(&self) -> Result<(), ResourceGovernorError> {
        if self.cpu_percent > 100 {
            return Err(ResourceGovernorError::InvalidSnapshot("cpuPercent"));
        }
        if self.device_slots_used > self.device_slots_total {
            return Err(ResourceGovernorError::InvalidSnapshot("deviceSlots"));
        }
        Ok(())
    }
}
impl ResourceBudget {
    pub fn validate(&self) -> Result<(), ResourceGovernorError> {
        if self.max_cpu_percent.is_some_and(|v| v > 100) {
            return Err(ResourceGovernorError::InvalidBudget("maxCpuPercent"));
        }
        if self.max_worker_concurrency == Some(0) || self.max_provider_concurrency == Some(0) {
            return Err(ResourceGovernorError::InvalidBudget("concurrency"));
        }
        Ok(())
    }
}

pub fn govern_resources(
    decision_id: &str,
    snapshot: &ResourceSnapshot,
    budget: &ResourceBudget,
) -> Result<ResourceGovernorDecision, ResourceGovernorError> {
    if decision_id.trim().is_empty() {
        return Err(ResourceGovernorError::EmptyDecisionId);
    }
    snapshot.validate()?;
    budget.validate()?;
    let mut limits: Vec<String> = Vec::new();
    if budget
        .max_cpu_percent
        .is_some_and(|v| snapshot.cpu_percent > v)
    {
        limits.push("cpu".into());
    }
    if budget.max_memory_mb.is_some_and(|v| snapshot.memory_mb > v) {
        limits.push("memory".into());
    }
    if budget
        .min_disk_free_mb
        .is_some_and(|v| snapshot.disk_free_mb < v)
    {
        limits.push("disk".into());
    }
    if budget
        .max_checkpoint_storage_mb
        .is_some_and(|v| snapshot.checkpoint_storage_mb > v)
    {
        limits.push("checkpoint-storage".into());
    }
    if budget
        .max_emulator_memory_mb
        .is_some_and(|v| snapshot.emulator_memory_mb > v)
    {
        limits.push("emulator-memory".into());
    }
    if budget
        .max_gradle_memory_mb
        .is_some_and(|v| snapshot.gradle_memory_mb > v)
    {
        limits.push("gradle-memory".into());
    }
    if budget
        .max_worker_concurrency
        .is_some_and(|v| snapshot.worker_concurrency > v)
    {
        limits.push("worker-concurrency".into());
    }
    if budget
        .max_provider_concurrency
        .is_some_and(|v| snapshot.provider_concurrency > v)
    {
        limits.push("provider-concurrency".into());
    }
    if budget
        .max_context_tokens
        .is_some_and(|v| snapshot.context_tokens > v)
    {
        limits.push("context-size".into());
    }
    if budget
        .max_log_volume_mb
        .is_some_and(|v| snapshot.log_volume_mb > v)
    {
        limits.push("log-volume".into());
    }
    if budget
        .max_build_duration_seconds
        .is_some_and(|v| snapshot.build_duration_seconds > v)
    {
        limits.push("build-duration".into());
    }
    if snapshot.device_slots_total > 0 && snapshot.device_slots_used == snapshot.device_slots_total
    {
        limits.push("device-slots".into());
    }
    let pressure = if limits.is_empty() {
        ResourcePressure::None
    } else if limits
        .iter()
        .any(|v| matches!(v.as_str(), "disk" | "memory" | "checkpoint-storage"))
    {
        ResourcePressure::Critical
    } else {
        ResourcePressure::Elevated
    };
    let mut actions = Vec::new();
    match pressure {
        ResourcePressure::None => actions.push(ResourceAction::NoChange),
        ResourcePressure::Elevated => {
            if limits.iter().any(|v| v == "context-size") {
                actions.push(ResourceAction::CompactContext);
            }
            if limits.iter().any(|v| v == "worker-concurrency") {
                actions.push(ResourceAction::ReduceWorkerConcurrency {
                    to: budget.max_worker_concurrency.unwrap_or(1),
                });
            }
            if limits.iter().any(|v| v == "provider-concurrency") {
                actions.push(ResourceAction::ReduceProviderConcurrency {
                    to: budget.max_provider_concurrency.unwrap_or(1),
                });
            }
            if limits.iter().any(|v| v == "device-slots") {
                actions.push(ResourceAction::DeferNonessentialVisualChecks);
            }
            actions.push(ResourceAction::SelectAffectedTests);
            actions.push(ResourceAction::SwitchToApprovedLighterProvider);
        }
        ResourcePressure::Critical => actions.extend([
            ResourceAction::PruneSafeCaches,
            ResourceAction::StopRedundantWorkers,
            ResourceAction::BlockNewWorkForProtection,
        ]),
    }
    Ok(ResourceGovernorDecision { schema_version: M49_SCHEMA_VERSION, decision_id: decision_id.into(), pressure, triggered_limits: limits, actions, safety_gates_preserved: true, reason: "resource adaptation changes scheduling only; sandbox, permission, evidence, signing, and artifact gates remain enforced".into() })
}

pub const M6_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyOutcome {
    Allow,
    Ask,
    Deny,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkCategory {
    None,
    Localhost,
    ApprovedHost,
    External,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerPolicy {
    pub schema_version: u16,
    pub worker_id: String,
    pub workspace_root: String,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub protected_path_patterns: Vec<String>,
    pub allowed_command_patterns: Vec<String>,
    pub denied_command_patterns: Vec<String>,
    pub allowed_network_categories: Vec<NetworkCategory>,
    pub allow_external_directories: bool,
    pub allow_destructive_commands: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub request_id: String,
    pub operation: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub network_category: NetworkCategory,
    pub destructive: bool,
    pub external_directory: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub schema_version: u16,
    pub decision_id: String,
    pub worker_id: String,
    pub request_id: String,
    pub outcome: PolicyOutcome,
    pub reasons: Vec<String>,
    pub authority: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    InvalidPolicy(&'static str),
    InvalidRequest(&'static str),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy(field) => write!(formatter, "M6 policy is invalid: {field}"),
            Self::InvalidRequest(field) => {
                write!(formatter, "M6 policy request is invalid: {field}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

impl WorkerPolicy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.schema_version != M6_SCHEMA_VERSION {
            return Err(PolicyError::InvalidPolicy("schemaVersion"));
        }
        if self.worker_id.trim().is_empty() || self.workspace_root.trim().is_empty() {
            return Err(PolicyError::InvalidPolicy("identity/workspace"));
        }
        if self.allowed_paths.is_empty() || self.protected_path_patterns.is_empty() {
            return Err(PolicyError::InvalidPolicy("path-rules"));
        }
        if self.allowed_paths.iter().any(|allowed| {
            self.denied_paths
                .iter()
                .any(|denied| same_path(allowed, denied))
        }) {
            return Err(PolicyError::InvalidPolicy("allowed-denied-overlap"));
        }
        Ok(())
    }

    pub fn authorize(&self, request: &PolicyRequest) -> Result<PolicyDecision, PolicyError> {
        self.validate()?;
        if request.request_id.trim().is_empty() || request.operation.trim().is_empty() {
            return Err(PolicyError::InvalidRequest("identity"));
        }
        let mut reasons: Vec<String> = Vec::new();
        if let Some(path) = request.path.as_deref() {
            if self
                .protected_path_patterns
                .iter()
                .any(|pattern| path_matches(path, pattern))
            {
                reasons.push("protected-path".into());
            }
            if self
                .denied_paths
                .iter()
                .any(|rule| path_matches(path, rule))
            {
                reasons.push("denied-path".into());
            } else if !self
                .allowed_paths
                .iter()
                .any(|rule| path_matches(path, rule))
            {
                reasons.push("outside-workspace".into());
            }
        }
        if let Some(command) = request.command.as_deref() {
            if self
                .denied_command_patterns
                .iter()
                .any(|pattern| text_matches(command, pattern))
            {
                reasons.push("denied-command".into());
            } else if !self
                .allowed_command_patterns
                .iter()
                .any(|pattern| text_matches(command, pattern))
            {
                reasons.push("unapproved-command".into());
            }
        }
        if request.external_directory && !self.allow_external_directories {
            reasons.push("external-directory".into());
        }
        if request.destructive && !self.allow_destructive_commands {
            reasons.push("destructive-operation".into());
        }
        if !self
            .allowed_network_categories
            .contains(&request.network_category)
        {
            reasons.push("network-category".into());
        }
        let outcome = if reasons.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "protected-path" | "denied-path" | "denied-command"
            )
        }) {
            PolicyOutcome::Deny
        } else if reasons.is_empty() {
            PolicyOutcome::Allow
        } else {
            PolicyOutcome::Ask
        };
        Ok(PolicyDecision {
            schema_version: M6_SCHEMA_VERSION,
            decision_id: format!("decision-{}", request.request_id),
            worker_id: self.worker_id.clone(),
            request_id: request.request_id.clone(),
            outcome,
            reasons,
            authority: "PolicyAuthority".into(),
        })
    }
}

fn same_path(left: &str, right: &str) -> bool {
    left.trim_end_matches(['/', '\\'])
        .eq_ignore_ascii_case(right.trim_end_matches(['/', '\\']))
}

fn path_matches(value: &str, pattern: &str) -> bool {
    let value = value.replace('\\', "/").to_ascii_lowercase();
    let pattern = pattern.replace('\\', "/").to_ascii_lowercase();
    if !pattern.contains('*') {
        return same_path(&value, &pattern) || value.starts_with(&format!("{pattern}/"));
    }
    let segments: Vec<&str> = pattern
        .split('*')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return true;
    }
    let mut cursor = 0usize;
    for (index, segment) in segments.iter().enumerate() {
        let Some(relative) = value[cursor..].find(segment) else {
            return false;
        };
        if index == 0 && !pattern.starts_with('*') && relative != 0 {
            return false;
        }
        cursor += relative + segment.len();
    }
    pattern.ends_with('*') || cursor == value.len()
}

fn text_matches(value: &str, pattern: &str) -> bool {
    pattern == "*"
        || value.eq_ignore_ascii_case(pattern)
        || value.starts_with(&format!("{pattern} "))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessRecord {
    pub process_id: u32,
    pub parent_process_id: Option<u32>,
    pub worker_id: String,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProcessRegistry {
    records: Vec<ProcessRecord>,
}

impl ProcessRegistry {
    pub fn register(&mut self, record: ProcessRecord) {
        self.records
            .retain(|current| current.process_id != record.process_id);
        self.records.push(record);
    }

    pub fn cancel_tree(&mut self, process_id: u32) -> Vec<u32> {
        let mut cancelled = Vec::new();
        let mut pending = vec![process_id];
        while let Some(parent) = pending.pop() {
            for record in self.records.iter_mut().filter(|record| {
                record.process_id == parent || record.parent_process_id == Some(parent)
            }) {
                if !record.cancelled {
                    record.cancelled = true;
                    cancelled.push(record.process_id);
                }
                if record.process_id != parent {
                    pending.push(record.process_id);
                }
            }
        }
        cancelled.sort_unstable();
        cancelled.dedup();
        cancelled
    }

    pub fn records(&self) -> &[ProcessRecord] {
        &self.records
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QuotaUsage {
    pub process_count: u16,
    pub memory_mb: u64,
    pub disk_write_mb: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProcessQuota {
    pub max_process_count: u16,
    pub max_memory_mb: u64,
    pub max_disk_write_mb: u64,
}

pub fn quota_allows(usage: &QuotaUsage, quota: &ProcessQuota) -> bool {
    usage.process_count <= quota.max_process_count
        && usage.memory_mb <= quota.max_memory_mb
        && usage.disk_write_mb <= quota.max_disk_write_mb
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DoomLoopDetector {
    fingerprints: Vec<String>,
}

impl DoomLoopDetector {
    pub fn record(&mut self, fingerprint: impl Into<String>, limit: usize) -> bool {
        let fingerprint = fingerprint.into();
        self.fingerprints.push(fingerprint.clone());
        self.fingerprints
            .iter()
            .rev()
            .take(limit)
            .all(|current| current == &fingerprint)
            && self.fingerprints.len() >= limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> ResourceSnapshot {
        ResourceSnapshot {
            cpu_percent: 50,
            memory_mb: 100,
            disk_free_mb: 1000,
            checkpoint_storage_mb: 100,
            emulator_memory_mb: 200,
            gradle_memory_mb: 300,
            worker_concurrency: 2,
            provider_concurrency: 2,
            context_tokens: 1000,
            log_volume_mb: 10,
            build_duration_seconds: 60,
            device_slots_used: 0,
            device_slots_total: 1,
        }
    }
    fn budget() -> ResourceBudget {
        ResourceBudget {
            max_cpu_percent: Some(90),
            max_memory_mb: Some(500),
            min_disk_free_mb: Some(500),
            max_checkpoint_storage_mb: Some(500),
            max_emulator_memory_mb: Some(500),
            max_gradle_memory_mb: Some(500),
            max_worker_concurrency: Some(4),
            max_provider_concurrency: Some(4),
            max_context_tokens: Some(2000),
            max_log_volume_mb: Some(50),
            max_build_duration_seconds: Some(120),
        }
    }
    #[test]
    fn no_pressure_is_deterministic() {
        let a = govern_resources("d1", &snapshot(), &budget()).unwrap();
        let b = govern_resources("d1", &snapshot(), &budget()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.pressure, ResourcePressure::None);
        assert_eq!(a.actions, vec![ResourceAction::NoChange]);
        assert!(a.safety_gates_preserved);
    }
    #[test]
    fn elevated_pressure_adapts_safely() {
        let mut s = snapshot();
        s.context_tokens = 5000;
        s.worker_concurrency = 8;
        let mut b = budget();
        b.max_worker_concurrency = Some(2);
        let d = govern_resources("d2", &s, &b).unwrap();
        assert_eq!(d.pressure, ResourcePressure::Elevated);
        assert!(d.actions.contains(&ResourceAction::CompactContext));
        assert!(d
            .actions
            .contains(&ResourceAction::ReduceWorkerConcurrency { to: 2 }));
        assert!(d.safety_gates_preserved);
    }
    #[test]
    fn critical_disk_pressure_blocks_new_work() {
        let mut s = snapshot();
        s.disk_free_mb = 10;
        let mut b = budget();
        b.min_disk_free_mb = Some(100);
        let d = govern_resources("d3", &s, &b).unwrap();
        assert_eq!(d.pressure, ResourcePressure::Critical);
        assert!(d
            .actions
            .contains(&ResourceAction::BlockNewWorkForProtection));
    }
}

#[cfg(test)]
mod m6_tests {
    use super::*;

    fn policy() -> WorkerPolicy {
        WorkerPolicy {
            schema_version: M6_SCHEMA_VERSION,
            worker_id: "worker-m6".into(),
            workspace_root: "/workspace/project".into(),
            allowed_paths: vec!["/workspace/project/*".into()],
            denied_paths: vec!["/workspace/project/.git/*".into()],
            protected_path_patterns: vec!["/home/user/.ssh/*".into(), "*.env".into()],
            allowed_command_patterns: vec!["gradlew".into(), "adb".into()],
            denied_command_patterns: vec!["rm".into(), "format".into()],
            allowed_network_categories: vec![NetworkCategory::None, NetworkCategory::Localhost],
            allow_external_directories: false,
            allow_destructive_commands: false,
        }
    }

    fn request(path: Option<&str>, command: Option<&str>) -> PolicyRequest {
        PolicyRequest {
            request_id: "request-m6".into(),
            operation: "test".into(),
            path: path.map(str::to_owned),
            command: command.map(str::to_owned),
            network_category: NetworkCategory::None,
            destructive: false,
            external_directory: false,
        }
    }

    #[test]
    fn policy_distinguishes_allow_ask_and_deny() {
        assert_eq!(
            policy()
                .authorize(&request(
                    Some("/workspace/project/app/Main.java"),
                    Some("gradlew assembleDebug")
                ))
                .unwrap()
                .outcome,
            PolicyOutcome::Allow
        );
        let mut ask = request(
            Some("/workspace/project/app/Main.java"),
            Some("gradlew assembleDebug"),
        );
        ask.network_category = NetworkCategory::External;
        assert_eq!(
            policy().authorize(&ask).unwrap().outcome,
            PolicyOutcome::Ask
        );
        let denied = policy()
            .authorize(&request(Some("/home/user/.ssh/id_rsa"), Some("cat")))
            .unwrap();
        assert_eq!(denied.outcome, PolicyOutcome::Deny);
        assert!(denied.reasons.contains(&"protected-path".into()));
    }

    #[test]
    fn process_tree_and_quota_are_enforced_deterministically() {
        let mut registry = ProcessRegistry::default();
        registry.register(ProcessRecord {
            process_id: 10,
            parent_process_id: None,
            worker_id: "worker-m6".into(),
            cancelled: false,
        });
        registry.register(ProcessRecord {
            process_id: 11,
            parent_process_id: Some(10),
            worker_id: "worker-m6".into(),
            cancelled: false,
        });
        registry.register(ProcessRecord {
            process_id: 12,
            parent_process_id: Some(11),
            worker_id: "worker-m6".into(),
            cancelled: false,
        });
        assert_eq!(registry.cancel_tree(10), vec![10, 11, 12]);
        assert!(registry.records().iter().all(|record| record.cancelled));
        assert!(quota_allows(
            &QuotaUsage {
                process_count: 1,
                memory_mb: 128,
                disk_write_mb: 20
            },
            &ProcessQuota {
                max_process_count: 2,
                max_memory_mb: 256,
                max_disk_write_mb: 50
            }
        ));
        assert!(!quota_allows(
            &QuotaUsage {
                process_count: 3,
                memory_mb: 128,
                disk_write_mb: 20
            },
            &ProcessQuota {
                max_process_count: 2,
                max_memory_mb: 256,
                max_disk_write_mb: 50
            }
        ));
    }

    #[test]
    fn doom_loop_detector_flags_only_repeated_identical_actions() {
        let mut detector = DoomLoopDetector::default();
        assert!(!detector.record("build:1", 3));
        assert!(!detector.record("build:1", 3));
        assert!(detector.record("build:1", 3));
        assert!(!detector.record("repair:1", 3));
    }
}
