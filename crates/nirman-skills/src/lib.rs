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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
