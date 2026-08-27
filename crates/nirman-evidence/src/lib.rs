#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::fmt;

pub const M9_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidDeviceProfile {
    pub profile_id: String,
    pub name: String,
    pub api_level: u32,
    pub architecture: String,
    pub width: u32,
    pub height: u32,
    pub density: u32,
    pub orientation: String,
    pub locale: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidDeviceObservation {
    pub schema_version: u16,
    pub observation_id: String,
    pub project_id: String,
    pub task_id: String,
    pub project_revision_id: String,
    pub device_profile_id: String,
    pub device_identity: String,
    pub package_name: String,
    pub install_status: String,
    pub launch_status: String,
    pub interaction_status: String,
    pub logcat_reference: Option<String>,
    pub screenshot_references: Vec<String>,
    pub accessibility_reference: Option<String>,
    pub visual_comparison_reference: Option<String>,
    pub synthetic_data_only: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualComparison {
    pub baseline_id: String,
    pub screenshot_id: String,
    pub metric: String,
    pub threshold: String,
    pub outcome: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum M9Error {
    InvalidProfile,
    InvalidObservation,
    MissingSyntheticData,
    EmptyField(&'static str),
}
impl fmt::Display for M9Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => f.write_str("M9 device profile is invalid"),
            Self::InvalidObservation => f.write_str("M9 device observation is invalid"),
            Self::MissingSyntheticData => {
                f.write_str("M9 observation must use synthetic data only")
            }
            Self::EmptyField(v) => write!(f, "M9 field is empty: {v}"),
        }
    }
}
impl std::error::Error for M9Error {}
impl AndroidDeviceProfile {
    pub fn validate(&self) -> Result<(), M9Error> {
        if self.profile_id.is_empty()
            || self.name.is_empty()
            || self.architecture.is_empty()
            || self.orientation.is_empty()
            || self.locale.is_empty()
        {
            return Err(M9Error::InvalidProfile);
        }
        if self.api_level == 0 || self.width == 0 || self.height == 0 || self.density == 0 {
            return Err(M9Error::InvalidProfile);
        }
        Ok(())
    }
}
impl AndroidDeviceObservation {
    pub fn validate(&self) -> Result<(), M9Error> {
        if self.schema_version != M9_SCHEMA_VERSION {
            return Err(M9Error::InvalidObservation);
        }
        for (n, v) in [
            ("observationId", &self.observation_id),
            ("projectId", &self.project_id),
            ("taskId", &self.task_id),
            ("projectRevisionId", &self.project_revision_id),
            ("deviceProfileId", &self.device_profile_id),
            ("deviceIdentity", &self.device_identity),
            ("packageName", &self.package_name),
        ] {
            if v.trim().is_empty() {
                return Err(M9Error::EmptyField(n));
            }
        }
        if !self.synthetic_data_only {
            return Err(M9Error::MissingSyntheticData);
        }
        if self.screenshot_references.is_empty() && self.logcat_reference.is_none() {
            return Err(M9Error::InvalidObservation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> AndroidDeviceProfile {
        AndroidDeviceProfile {
            profile_id: "pixel-phone".into(),
            name: "Pixel phone".into(),
            api_level: 35,
            architecture: "x86_64".into(),
            width: 1080,
            height: 2400,
            density: 420,
            orientation: "portrait".into(),
            locale: "en-US".into(),
        }
    }
    #[test]
    fn profile_and_observation_are_validated_and_round_trip() {
        profile().validate().unwrap();
        let o = AndroidDeviceObservation {
            schema_version: M9_SCHEMA_VERSION,
            observation_id: "obs-1".into(),
            project_id: "p".into(),
            task_id: "t".into(),
            project_revision_id: "r".into(),
            device_profile_id: profile().profile_id,
            device_identity: "emulator-1".into(),
            package_name: "com.example.app".into(),
            install_status: "OBSERVED_SUCCESS".into(),
            launch_status: "OBSERVED_SUCCESS".into(),
            interaction_status: "OBSERVED_PASS".into(),
            logcat_reference: Some("logcat-1".into()),
            screenshot_references: vec!["screen-1".into()],
            accessibility_reference: Some("a11y-1".into()),
            visual_comparison_reference: Some("visual-1".into()),
            synthetic_data_only: true,
        };
        o.validate().unwrap();
        assert_eq!(
            o,
            serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap()
        );
    }
    #[test]
    fn personal_device_data_and_empty_observation_are_rejected() {
        let mut o = AndroidDeviceObservation {
            schema_version: M9_SCHEMA_VERSION,
            observation_id: "o".into(),
            project_id: "p".into(),
            task_id: "t".into(),
            project_revision_id: "r".into(),
            device_profile_id: "d".into(),
            device_identity: "d".into(),
            package_name: "pkg".into(),
            install_status: "".into(),
            launch_status: "".into(),
            interaction_status: "".into(),
            logcat_reference: None,
            screenshot_references: vec![],
            accessibility_reference: None,
            visual_comparison_reference: None,
            synthetic_data_only: false,
        };
        assert_eq!(o.validate(), Err(M9Error::MissingSyntheticData));
        o.synthetic_data_only = true;
        assert_eq!(o.validate(), Err(M9Error::InvalidObservation));
    }
}
