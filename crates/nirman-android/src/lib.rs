#![forbid(unsafe_code)]

use nirman_domain::AndroidConstructionContract;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const M43_SCHEMA_VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ToolchainComponentKind {
    Jdk,
    Gradle,
    AndroidGradlePlugin,
    Kotlin,
    AndroidSdk,
    BuildTools,
    PlatformTools,
    Ndk,
    Cmake,
    Adb,
    Emulator,
    Node,
    PackageManager,
    ReactNativeExpo,
}

impl ToolchainComponentKind {
    fn command_name(&self) -> Option<&'static str> {
        match self {
            Self::Jdk => Some("java"),
            Self::Gradle => Some("gradle"),
            Self::Adb => Some("adb"),
            Self::Emulator => Some("emulator"),
            Self::Node => Some("node"),
            Self::PackageManager => Some("npm"),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Jdk => "jdk",
            Self::Gradle => "gradle",
            Self::AndroidGradlePlugin => "android_gradle_plugin",
            Self::Kotlin => "kotlin",
            Self::AndroidSdk => "android_sdk",
            Self::BuildTools => "build_tools",
            Self::PlatformTools => "platform_tools",
            Self::Ndk => "ndk",
            Self::Cmake => "cmake",
            Self::Adb => "adb",
            Self::Emulator => "emulator",
            Self::Node => "node",
            Self::PackageManager => "package_manager",
            Self::ReactNativeExpo => "react_native_expo",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ToolchainRequirement {
    pub component: ToolchainComponentKind,
    pub required: bool,
    pub expected_version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidToolchainManifest {
    pub schema_version: u16,
    pub manifest_id: String,
    pub project_id: String,
    pub task_id: String,
    pub technology_plan_revision: u64,
    pub requirements: Vec<ToolchainRequirement>,
    pub selected_device_ids: Vec<String>,
    pub isolated_environment_policy: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolchainLockEntry {
    pub component: ToolchainComponentKind,
    pub version: String,
    pub path: String,
    pub binary_hash: String,
    pub license_id: String,
    pub acquisition_source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidToolchainLock {
    pub schema_version: u16,
    pub lock_id: String,
    pub manifest_id: String,
    pub entries: Vec<ToolchainLockEntry>,
    pub lock_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityStatus {
    Available,
    Repairable,
    UserRequired,
    Unavailable,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentCapabilityRecord {
    pub capability_id: String,
    pub component: ToolchainComponentKind,
    pub status: CapabilityStatus,
    pub observed_version: Option<String>,
    pub path: Option<String>,
    pub fingerprint: Option<String>,
    pub detail: String,
    pub evidence_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentSnapshot {
    pub schema_version: u16,
    pub snapshot_id: String,
    pub project_id: String,
    pub task_id: String,
    pub toolchain_lock_hash: Option<String>,
    pub tool_versions: BTreeMap<String, String>,
    pub tool_hashes: BTreeMap<String, String>,
    pub selected_device_identity: Option<String>,
    pub selected_api_level: Option<u32>,
    pub selected_abi: Option<String>,
    pub build_variant: String,
    pub environment_variables: BTreeMap<String, String>,
    pub gradle_lock_hash: Option<String>,
    pub package_lock_hash: Option<String>,
    pub provider_metadata: BTreeMap<String, String>,
    pub project_fingerprint: String,
    pub command_policy: String,
    pub captured_at_epoch_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreflightStatus {
    Available,
    Repairable,
    UserRequired,
    Unavailable,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidToolchainPreflight {
    pub schema_version: u16,
    pub preflight_id: String,
    pub manifest: AndroidToolchainManifest,
    pub lock: Option<AndroidToolchainLock>,
    pub capabilities: Vec<EnvironmentCapabilityRecord>,
    pub environment_snapshot: EnvironmentSnapshot,
    pub status: PreflightStatus,
    pub repair_actions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeObservation {
    pub available: bool,
    pub repairable: bool,
    pub user_required: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub fingerprint: Option<String>,
    pub license_id: Option<String>,
    pub detail: String,
}

impl ProbeObservation {
    pub fn available(version: &str, path: &str, license_id: &str) -> Self {
        Self {
            available: true,
            repairable: false,
            user_required: false,
            version: Some(version.into()),
            path: Some(path.into()),
            fingerprint: Some(hash_text(&format!("{path}\n{version}"))),
            license_id: Some(license_id.into()),
            detail: "component is available".into(),
        }
    }

    pub fn missing(detail: &str) -> Self {
        Self {
            available: false,
            repairable: false,
            user_required: false,
            version: None,
            path: None,
            fingerprint: None,
            license_id: None,
            detail: detail.into(),
        }
    }

    pub fn repairable(detail: &str) -> Self {
        Self {
            available: false,
            repairable: true,
            user_required: false,
            version: None,
            path: None,
            fingerprint: None,
            license_id: None,
            detail: detail.into(),
        }
    }
}

pub trait CapabilityProbe: Send + Sync {
    fn observe(&self, component: &ToolchainComponentKind) -> ProbeObservation;
}

#[derive(Clone, Debug, Default)]
pub struct StaticCapabilityProbe {
    observations: BTreeMap<ToolchainComponentKind, ProbeObservation>,
}

impl StaticCapabilityProbe {
    pub fn with_available(
        mut self,
        component: ToolchainComponentKind,
        version: &str,
        path: &str,
        license_id: &str,
    ) -> Self {
        self.observations.insert(
            component,
            ProbeObservation::available(version, path, license_id),
        );
        self
    }

    pub fn with_observation(
        mut self,
        component: ToolchainComponentKind,
        observation: ProbeObservation,
    ) -> Self {
        self.observations.insert(component, observation);
        self
    }
}

impl CapabilityProbe for StaticCapabilityProbe {
    fn observe(&self, component: &ToolchainComponentKind) -> ProbeObservation {
        self.observations
            .get(component)
            .cloned()
            .unwrap_or_else(|| ProbeObservation::missing("fixture did not provide this capability"))
    }
}

#[derive(Clone, Debug, Default)]
pub struct HostCapabilityProbe;

impl HostCapabilityProbe {
    fn command_observation(&self, component: &ToolchainComponentKind) -> ProbeObservation {
        let Some(command) = component.command_name() else {
            return ProbeObservation::repairable(
                "component is project-declared and requires project/toolchain inspection",
            );
        };
        let output = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", command, "--version"])
                .output()
        } else {
            Command::new(command).arg("--version").output()
        };
        match output {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                let text = if text.trim().is_empty() {
                    String::from_utf8_lossy(&output.stderr).to_string()
                } else {
                    text.to_string()
                };
                let version = text.lines().next().unwrap_or("unknown").trim();
                ProbeObservation::available(version, command, "host-declared")
            }
            Ok(_) => ProbeObservation::missing("tool command returned a non-success status"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ProbeObservation::missing("tool command is not installed or not discoverable")
            }
            Err(_) => ProbeObservation::user_required("tool command could not be inspected"),
        }
    }
}

impl ProbeObservation {
    fn user_required(detail: &str) -> Self {
        Self {
            available: false,
            repairable: false,
            user_required: true,
            version: None,
            path: None,
            fingerprint: None,
            license_id: None,
            detail: detail.into(),
        }
    }
}

impl CapabilityProbe for HostCapabilityProbe {
    fn observe(&self, component: &ToolchainComponentKind) -> ProbeObservation {
        match component {
            ToolchainComponentKind::AndroidSdk => {
                let path = env::var("ANDROID_HOME")
                    .or_else(|_| env::var("ANDROID_SDK_ROOT"))
                    .ok();
                match path {
                    Some(path) if Path::new(&path).is_dir() => {
                        ProbeObservation::available("host-sdk", &path, "android-sdk-license")
                    }
                    Some(_) => {
                        ProbeObservation::missing("Android SDK environment path is not a directory")
                    }
                    None => {
                        ProbeObservation::missing("ANDROID_HOME/ANDROID_SDK_ROOT is not configured")
                    }
                }
            }
            ToolchainComponentKind::BuildTools
            | ToolchainComponentKind::PlatformTools
            | ToolchainComponentKind::AndroidGradlePlugin
            | ToolchainComponentKind::Kotlin
            | ToolchainComponentKind::Ndk
            | ToolchainComponentKind::Cmake
            | ToolchainComponentKind::ReactNativeExpo => ProbeObservation::repairable(
                "component must be resolved from the locked project/toolchain environment",
            ),
            _ => self.command_observation(component),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidToolchainError {
    UnsupportedSchemaVersion,
    EmptyField(&'static str),
    DuplicateComponent,
    MissingRequiredComponent,
    UnsupportedRequiredToolchain(String),
    InvalidLock,
}

impl fmt::Display for AndroidToolchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedSchemaVersion => "M43 toolchain schema version is unsupported",
            Self::EmptyField(_) => "M43 toolchain record has a required field missing",
            Self::DuplicateComponent => "M43 toolchain manifest contains a duplicate component",
            Self::MissingRequiredComponent => {
                "M43 toolchain manifest misses a required core component"
            }
            Self::UnsupportedRequiredToolchain(_) => "M43 toolchain requirement is unsupported",
            Self::InvalidLock => "M43 toolchain lock is invalid",
        };
        f.write_str(message)
    }
}

impl std::error::Error for AndroidToolchainError {}

pub trait AndroidConstructionToolchainSource {
    fn toolchain_manifest(&self) -> Result<AndroidToolchainManifest, AndroidToolchainError>;
}

impl AndroidConstructionToolchainSource for AndroidConstructionContract {
    fn toolchain_manifest(&self) -> Result<AndroidToolchainManifest, AndroidToolchainError> {
        self.validate()
            .map_err(|_| AndroidToolchainError::InvalidLock)?;
        let mut requirements = vec![
            ToolchainRequirement {
                component: ToolchainComponentKind::Jdk,
                required: true,
                expected_version: None,
            },
            ToolchainRequirement {
                component: ToolchainComponentKind::Gradle,
                required: true,
                expected_version: None,
            },
            ToolchainRequirement {
                component: ToolchainComponentKind::AndroidSdk,
                required: true,
                expected_version: None,
            },
            ToolchainRequirement {
                component: ToolchainComponentKind::PlatformTools,
                required: true,
                expected_version: None,
            },
            ToolchainRequirement {
                component: ToolchainComponentKind::Adb,
                required: true,
                expected_version: None,
            },
            ToolchainRequirement {
                component: ToolchainComponentKind::Emulator,
                required: true,
                expected_version: None,
            },
        ];
        for required in &self.technology_plan.required_toolchains {
            let component = match required.as_str() {
                "jdk" | "java" => ToolchainComponentKind::Jdk,
                "gradle" => ToolchainComponentKind::Gradle,
                "android-sdk" | "sdk" => ToolchainComponentKind::AndroidSdk,
                "platform-tools" => ToolchainComponentKind::PlatformTools,
                "adb" => ToolchainComponentKind::Adb,
                "emulator" => ToolchainComponentKind::Emulator,
                "build-tools" => ToolchainComponentKind::BuildTools,
                "ndk" => ToolchainComponentKind::Ndk,
                "cmake" => ToolchainComponentKind::Cmake,
                "node" => ToolchainComponentKind::Node,
                "package-manager" | "npm" | "pnpm" => ToolchainComponentKind::PackageManager,
                other => {
                    return Err(AndroidToolchainError::UnsupportedRequiredToolchain(
                        other.into(),
                    ))
                }
            };
            requirements.push(ToolchainRequirement {
                component,
                required: true,
                expected_version: None,
            });
        }
        if self
            .technology_plan
            .selected_languages
            .iter()
            .any(|value| value.eq_ignore_ascii_case("kotlin"))
        {
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::Kotlin,
                required: true,
                expected_version: None,
            });
        }
        if self
            .technology_plan
            .selected_ui_frameworks
            .iter()
            .any(|value| value.to_ascii_lowercase().contains("compose"))
        {
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::AndroidGradlePlugin,
                required: true,
                expected_version: None,
            });
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::BuildTools,
                required: true,
                expected_version: None,
            });
        }
        if self
            .technology_plan
            .selected_ui_frameworks
            .iter()
            .any(|value| {
                let value = value.to_ascii_lowercase();
                value.contains("react") || value.contains("expo")
            })
        {
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::Node,
                required: true,
                expected_version: None,
            });
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::PackageManager,
                required: true,
                expected_version: None,
            });
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::ReactNativeExpo,
                required: true,
                expected_version: None,
            });
        }
        if !self.technology_plan.selected_native_modules.is_empty() {
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::Ndk,
                required: true,
                expected_version: None,
            });
            requirements.push(ToolchainRequirement {
                component: ToolchainComponentKind::Cmake,
                required: true,
                expected_version: None,
            });
        }
        let mut seen = BTreeSet::new();
        requirements.retain(|requirement| seen.insert(requirement.component.clone()));
        let manifest = AndroidToolchainManifest {
            schema_version: M43_SCHEMA_VERSION,
            manifest_id: format!("manifest-{}", self.contract_id),
            project_id: self.project_id.0.clone(),
            task_id: self.task_id.0.clone(),
            technology_plan_revision: self.technology_plan.revision.0,
            requirements,
            selected_device_ids: self
                .device_matrix
                .iter()
                .map(|device| device.device_id.clone())
                .collect(),
            isolated_environment_policy: "locked-toolchain-isolated-environment".into(),
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

impl AndroidToolchainManifest {
    pub fn validate(&self) -> Result<(), AndroidToolchainError> {
        if self.schema_version != M43_SCHEMA_VERSION {
            return Err(AndroidToolchainError::UnsupportedSchemaVersion);
        }
        for value in [
            &self.manifest_id,
            &self.project_id,
            &self.task_id,
            &self.isolated_environment_policy,
        ] {
            if value.trim().is_empty() {
                return Err(AndroidToolchainError::EmptyField("manifest"));
            }
        }
        let mut seen = BTreeSet::new();
        for requirement in &self.requirements {
            if !seen.insert(requirement.component.clone()) {
                return Err(AndroidToolchainError::DuplicateComponent);
            }
        }
        for required in [
            ToolchainComponentKind::Jdk,
            ToolchainComponentKind::Gradle,
            ToolchainComponentKind::AndroidSdk,
            ToolchainComponentKind::PlatformTools,
            ToolchainComponentKind::Adb,
            ToolchainComponentKind::Emulator,
        ] {
            if !self
                .requirements
                .iter()
                .any(|item| item.component == required && item.required)
            {
                return Err(AndroidToolchainError::MissingRequiredComponent);
            }
        }
        if self.selected_device_ids.is_empty() {
            return Err(AndroidToolchainError::EmptyField("selectedDeviceIds"));
        }
        Ok(())
    }
}

impl AndroidToolchainLock {
    pub fn validate(&self) -> Result<(), AndroidToolchainError> {
        if self.schema_version != M43_SCHEMA_VERSION
            || self.lock_id.trim().is_empty()
            || self.manifest_id.trim().is_empty()
            || self.lock_hash.trim().is_empty()
        {
            return Err(AndroidToolchainError::InvalidLock);
        }
        let mut seen = BTreeSet::new();
        for entry in &self.entries {
            if !seen.insert(entry.component.clone())
                || entry.version.trim().is_empty()
                || entry.path.trim().is_empty()
                || entry.binary_hash.trim().is_empty()
                || entry.license_id.trim().is_empty()
                || entry.acquisition_source.trim().is_empty()
            {
                return Err(AndroidToolchainError::InvalidLock);
            }
        }
        Ok(())
    }
}

pub fn plan_preflight<P: CapabilityProbe + ?Sized>(
    contract: &AndroidConstructionContract,
    build_variant: &str,
    probe: &P,
) -> Result<AndroidToolchainPreflight, AndroidToolchainError> {
    if build_variant.trim().is_empty() {
        return Err(AndroidToolchainError::EmptyField("buildVariant"));
    }
    let manifest = contract.toolchain_manifest()?;
    let mut capabilities = Vec::with_capacity(manifest.requirements.len());
    let mut observations = BTreeMap::new();
    for requirement in &manifest.requirements {
        let observation = probe.observe(&requirement.component);
        let status = if observation.available {
            CapabilityStatus::Available
        } else if observation.user_required {
            CapabilityStatus::UserRequired
        } else if observation.repairable {
            CapabilityStatus::Repairable
        } else {
            CapabilityStatus::Unavailable
        };
        observations.insert(requirement.component.clone(), observation.clone());
        capabilities.push(EnvironmentCapabilityRecord {
            capability_id: format!(
                "capability.{}.{}",
                contract.task_id.0,
                requirement.component.as_str()
            ),
            component: requirement.component.clone(),
            status,
            observed_version: observation.version,
            path: observation.path,
            fingerprint: observation.fingerprint,
            detail: observation.detail,
            evidence_id: format!("evidence.m43.{}", requirement.component.as_str()),
        });
    }
    let lock = if capabilities
        .iter()
        .all(|capability| capability.status == CapabilityStatus::Available)
    {
        let entries = capabilities
            .iter()
            .map(|capability| ToolchainLockEntry {
                component: capability.component.clone(),
                version: capability.observed_version.clone().unwrap_or_default(),
                path: capability.path.clone().unwrap_or_default(),
                binary_hash: capability.fingerprint.clone().unwrap_or_default(),
                license_id: observations
                    .get(&capability.component)
                    .and_then(|observation| observation.license_id.clone())
                    .unwrap_or_else(|| "declared-by-component".into()),
                acquisition_source: "observed-local-capability".into(),
            })
            .collect::<Vec<_>>();
        let lock_hash =
            hash_text(&serde_json::to_string(&entries).expect("lock entries serialize"));
        Some(AndroidToolchainLock {
            schema_version: M43_SCHEMA_VERSION,
            lock_id: format!("lock-{}", manifest.manifest_id),
            manifest_id: manifest.manifest_id.clone(),
            entries,
            lock_hash,
        })
    } else {
        None
    };
    if let Some(lock) = &lock {
        lock.validate()?;
    }
    let mut tool_versions = BTreeMap::new();
    let mut tool_hashes = BTreeMap::new();
    for capability in &capabilities {
        if let Some(version) = &capability.observed_version {
            tool_versions.insert(capability.component.as_str().into(), version.clone());
        }
        if let Some(fingerprint) = &capability.fingerprint {
            tool_hashes.insert(capability.component.as_str().into(), fingerprint.clone());
        }
    }
    let selected_device = contract.device_matrix.first();
    let environment_variables = [
        "ANDROID_HOME",
        "ANDROID_SDK_ROOT",
        "JAVA_HOME",
        "GRADLE_HOME",
    ]
    .into_iter()
    .filter_map(|name| env::var(name).ok().map(|value| (name.into(), value)))
    .collect();
    let environment_snapshot = EnvironmentSnapshot {
        schema_version: M43_SCHEMA_VERSION,
        snapshot_id: format!("snapshot-{}", manifest.manifest_id),
        project_id: contract.project_id.0.clone(),
        task_id: contract.task_id.0.clone(),
        toolchain_lock_hash: lock.as_ref().map(|value| value.lock_hash.clone()),
        tool_versions,
        tool_hashes,
        selected_device_identity: selected_device.map(|device| device.device_id.clone()),
        selected_api_level: selected_device.map(|device| device.api_level),
        selected_abi: selected_device.map(|device| device.architecture.clone()),
        build_variant: build_variant.into(),
        environment_variables,
        gradle_lock_hash: None,
        package_lock_hash: None,
        provider_metadata: BTreeMap::new(),
        project_fingerprint: hash_text(
            &serde_json::to_string(contract).expect("contract serializes"),
        ),
        command_policy: "local-authorized-preflight-read-only".into(),
        captured_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let status = if capabilities
        .iter()
        .any(|item| item.status == CapabilityStatus::Unavailable)
    {
        PreflightStatus::Unavailable
    } else if capabilities
        .iter()
        .any(|item| item.status == CapabilityStatus::UserRequired)
    {
        PreflightStatus::UserRequired
    } else if capabilities
        .iter()
        .any(|item| item.status == CapabilityStatus::Repairable)
    {
        PreflightStatus::Repairable
    } else {
        PreflightStatus::Available
    };
    Ok(AndroidToolchainPreflight {
        schema_version: M43_SCHEMA_VERSION,
        preflight_id: format!("preflight-{}", manifest.manifest_id),
        manifest,
        lock,
        capabilities,
        environment_snapshot,
        status,
        repair_actions: vec![],
    })
}

fn hash_text(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_domain::{
        AndroidDeviceProfile, AndroidTechnologyPlan, ArtifactKind, ArtifactModel, ProjectId,
        Revision, TaskId, ValidationModel,
    };

    fn contract() -> AndroidConstructionContract {
        AndroidConstructionContract {
            schema_version: 1,
            contract_id: "m43-contract".into(),
            project_id: ProjectId("m43-project".into()),
            target_platforms: vec!["android".into()],
            task_id: TaskId("m43-task".into()),
            user_intent: "Build an Android app".into(),
            screenshots: vec![],
            assets: vec![],
            features: vec![nirman_domain::ConstructionRequirement {
                requirement_id: "feature".into(),
                statement: "feature".into(),
                origin: nirman_domain::RequirementOrigin::UserFact,
                source_reference_ids: vec![],
            }],
            ui: vec![nirman_domain::ConstructionRequirement {
                requirement_id: "ui".into(),
                statement: "ui".into(),
                origin: nirman_domain::RequirementOrigin::UserFact,
                source_reference_ids: vec![],
            }],
            data: vec![],
            integrations: vec![],
            technology_plan: AndroidTechnologyPlan {
                plan_id: "plan".into(),
                task_id: TaskId("m43-task".into()),
                requested_capabilities: vec!["offline-storage".into()],
                visual_requirements: vec![],
                selected_languages: vec!["kotlin".into()],
                selected_ui_frameworks: vec!["jetpack-compose".into()],
                selected_runtime_layers: vec![],
                selected_native_modules: vec![],
                selected_build_plugins: vec![],
                selected_device_apis: vec![],
                selected_libraries: vec![],
                compatibility_constraints: vec![],
                rejected_alternatives: vec![],
                required_toolchains: vec!["jdk".into(), "gradle".into(), "android-sdk".into()],
                validation_plan: vec!["compile".into()],
                confidence: None,
                revision: Revision(1),
            },
            android_requirements: vec![],
            device_matrix: vec![AndroidDeviceProfile {
                device_id: "pixel".into(),
                name: "Pixel".into(),
                platform_version: "Android".into(),
                api_level: 35,
                architecture: "x86_64".into(),
                width: 1080,
                height: 2400,
                density: 420,
                orientation: "portrait".into(),
                locale: "en-US".into(),
                permissions: vec![],
                network_profile: "offline".into(),
            }],
            validation_model: ValidationModel {
                required_checks: vec!["compile".into()],
                acceptance_criteria: vec!["works".into()],
            },
            artifact_model: ArtifactModel {
                required_artifact: ArtifactKind::Apk,
                aab_declared: false,
            },
        }
    }

    #[test]
    fn manifest_is_android_contract_derived_and_contains_required_core_capabilities() {
        let manifest = contract().toolchain_manifest().expect("manifest");
        manifest.validate().expect("manifest valid");
        assert!(manifest
            .requirements
            .iter()
            .any(|item| item.component == ToolchainComponentKind::Adb));
        assert!(manifest
            .requirements
            .iter()
            .any(|item| item.component == ToolchainComponentKind::Emulator));
        assert_eq!(manifest.selected_device_ids, vec!["pixel"]);
    }

    #[test]
    fn deterministic_fixture_preflight_generates_lock_snapshot_and_license_hash_records() {
        let probe = StaticCapabilityProbe::default()
            .with_available(
                ToolchainComponentKind::Jdk,
                "17",
                "/locked/jdk",
                "jdk-license",
            )
            .with_available(
                ToolchainComponentKind::Gradle,
                "8.7",
                "/locked/gradle",
                "gradle-license",
            )
            .with_available(
                ToolchainComponentKind::AndroidSdk,
                "35",
                "/locked/sdk",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::PlatformTools,
                "35",
                "/locked/platform-tools",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::Adb,
                "35",
                "/locked/adb",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::Emulator,
                "35",
                "/locked/emulator",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::Kotlin,
                "2.0",
                "/locked/kotlin",
                "kotlin-license",
            )
            .with_available(
                ToolchainComponentKind::AndroidGradlePlugin,
                "8.7",
                "/locked/agp",
                "android-sdk-license",
            )
            .with_available(
                ToolchainComponentKind::BuildTools,
                "35.0.0",
                "/locked/build-tools",
                "android-sdk-license",
            );
        let report = plan_preflight(&contract(), "debug", &probe).expect("preflight");
        assert_eq!(report.status, PreflightStatus::Available);
        assert!(report.lock.is_some());
        assert!(report.environment_snapshot.toolchain_lock_hash.is_some());
        assert!(report
            .capabilities
            .iter()
            .all(|item| item.status == CapabilityStatus::Available));
        report
            .lock
            .as_ref()
            .unwrap()
            .validate()
            .expect("lock valid");
    }

    #[test]
    fn missing_capability_is_deterministically_classified_without_fake_success() {
        let probe = StaticCapabilityProbe::default()
            .with_available(
                ToolchainComponentKind::Jdk,
                "17",
                "/locked/jdk",
                "jdk-license",
            )
            .with_available(
                ToolchainComponentKind::Gradle,
                "8.7",
                "/locked/gradle",
                "gradle-license",
            );
        let first = plan_preflight(&contract(), "debug", &probe).expect("preflight");
        let second = plan_preflight(&contract(), "debug", &probe).expect("preflight");
        assert_eq!(first.status, PreflightStatus::Unavailable);
        assert_eq!(first.capabilities, second.capabilities);
        assert!(first.lock.is_none());
    }
}
