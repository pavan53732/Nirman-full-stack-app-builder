#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

pub const M49_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderModelProvenance {
    pub provider_id: String,
    pub model_id: String,
    pub request_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTrace {
    pub schema_version: u16,
    pub decision_id: String,
    pub session_id: String,
    pub task_id: String,
    pub worker_id: Option<String>,
    pub input_references: Vec<String>,
    pub constraints: Vec<String>,
    pub candidate_actions: Vec<String>,
    pub selected_action: String,
    pub deterministic_policy_checks: Vec<String>,
    pub provider_model_provenance: Option<ProviderModelProvenance>,
    pub confidence_percent: u8,
    pub outcome_event: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionTraceError {
    UnsupportedSchemaVersion,
    EmptyField(&'static str),
    InvalidConfidence,
    SelectedActionNotCandidate,
    MissingEvidence,
    SensitiveContent(&'static str),
}

impl fmt::Display for DecisionTraceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion => {
                formatter.write_str("M49 decision trace schema is unsupported")
            }
            Self::EmptyField(field) => {
                write!(formatter, "M49 decision trace field is empty: {field}")
            }
            Self::InvalidConfidence => {
                formatter.write_str("M49 decision confidence is outside 0..=100")
            }
            Self::SelectedActionNotCandidate => {
                formatter.write_str("M49 selected action is not in candidate actions")
            }
            Self::MissingEvidence => {
                formatter.write_str("M49 material decision has no evidence reference")
            }
            Self::SensitiveContent(field) => write!(
                formatter,
                "M49 decision trace contains sensitive content in {field}"
            ),
        }
    }
}
impl std::error::Error for DecisionTraceError {}

fn contains_sensitive_content(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "access_token",
        "password",
        "private_key",
        "secret",
        "authorization: bearer",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

impl DecisionTrace {
    pub fn validate(&self) -> Result<(), DecisionTraceError> {
        if self.schema_version != M49_SCHEMA_VERSION {
            return Err(DecisionTraceError::UnsupportedSchemaVersion);
        }
        for (field, value) in [
            ("decisionId", self.decision_id.as_str()),
            ("sessionId", self.session_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("selectedAction", self.selected_action.as_str()),
            ("outcomeEvent", self.outcome_event.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(DecisionTraceError::EmptyField(field));
            }
            if contains_sensitive_content(value) {
                return Err(DecisionTraceError::SensitiveContent(field));
            }
        }
        for (field, values) in [
            ("inputReferences", &self.input_references),
            ("constraints", &self.constraints),
            ("candidateActions", &self.candidate_actions),
            (
                "deterministicPolicyChecks",
                &self.deterministic_policy_checks,
            ),
            ("evidenceIds", &self.evidence_ids),
        ] {
            if values.iter().any(|value| value.trim().is_empty()) {
                return Err(DecisionTraceError::EmptyField(field));
            }
            if values.iter().any(|value| contains_sensitive_content(value)) {
                return Err(DecisionTraceError::SensitiveContent(field));
            }
        }
        if self.candidate_actions.is_empty() {
            return Err(DecisionTraceError::EmptyField("candidateActions"));
        }
        if !self
            .candidate_actions
            .iter()
            .any(|action| action == &self.selected_action)
        {
            return Err(DecisionTraceError::SelectedActionNotCandidate);
        }
        if self.confidence_percent > 100 {
            return Err(DecisionTraceError::InvalidConfidence);
        }
        if self.evidence_ids.is_empty() {
            return Err(DecisionTraceError::MissingEvidence);
        }
        if let Some(provenance) = &self.provider_model_provenance {
            for (field, value) in [
                ("providerId", provenance.provider_id.as_str()),
                ("modelId", provenance.model_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(DecisionTraceError::EmptyField(field));
                }
                if contains_sensitive_content(value) {
                    return Err(DecisionTraceError::SensitiveContent(field));
                }
            }
            if provenance
                .request_id
                .as_ref()
                .is_some_and(|value| contains_sensitive_content(value))
            {
                return Err(DecisionTraceError::SensitiveContent("requestId"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace() -> DecisionTrace {
        DecisionTrace {
            schema_version: M49_SCHEMA_VERSION,
            decision_id: "decision-1".into(),
            session_id: "session-1".into(),
            task_id: "task-1".into(),
            worker_id: Some("worker-1".into()),
            input_references: vec!["requirements:revision-1".into()],
            constraints: vec!["android-only".into(), "locked-toolchain".into()],
            candidate_actions: vec!["compose-reload".into(), "apk-reinstall".into()],
            selected_action: "compose-reload".into(),
            deterministic_policy_checks: vec!["preview-policy:allow".into()],
            provider_model_provenance: Some(ProviderModelProvenance {
                provider_id: "provider-1".into(),
                model_id: "model-1".into(),
                request_id: Some("request-1".into()),
            }),
            confidence_percent: 90,
            outcome_event: "PreviewFallbackSelected".into(),
            evidence_ids: vec!["evidence-1".into()],
        }
    }

    #[test]
    fn decision_trace_is_concise_deterministic_and_evidence_linked() {
        let value = trace();
        value.validate().expect("valid trace");
        let encoded = serde_json::to_string(&value).expect("trace JSON");
        assert_eq!(value, serde_json::from_str(&encoded).expect("trace reload"));
    }

    #[test]
    fn decision_trace_rejects_unselected_actions_and_sensitive_content() {
        let mut invalid = trace();
        invalid.selected_action = "unknown".into();
        assert_eq!(
            invalid.validate().expect_err("unselected action"),
            DecisionTraceError::SelectedActionNotCandidate
        );
        let mut secret = trace();
        secret.constraints = vec!["api_key must remain private".into()];
        assert_eq!(
            secret.validate().expect_err("secret content"),
            DecisionTraceError::SensitiveContent("constraints")
        );
    }
}
