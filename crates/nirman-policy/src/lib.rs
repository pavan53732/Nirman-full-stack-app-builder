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
