#![forbid(unsafe_code)]

use nirman_domain::AndroidConstructionContract;
use nirman_project::GraphNodeKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
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

pub const M47_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AndroidRequirementKind {
    Sdk,
    Abi,
    Manifest,
    Permission,
    Service,
    Resource,
    Accessibility,
    Localization,
    BackgroundBehavior,
    Release,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AndroidRequirementStatus {
    Satisfied,
    Missing,
    Excessive,
    Incompatible,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AndroidRequirement {
    pub requirement_id: String,
    pub kind: AndroidRequirementKind,
    pub subject: String,
    pub desired_value: String,
    pub observed_value: Option<String>,
    pub source: String,
    pub confidence_percent: u8,
    pub affected_files: Vec<String>,
    pub validation_rule: String,
    pub status: AndroidRequirementStatus,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AndroidRequirementManifest {
    pub schema_version: u16,
    pub manifest_id: String,
    pub project_id: String,
    pub task_id: String,
    pub project_revision: u64,
    pub project_fingerprint: String,
    pub requirements: Vec<AndroidRequirement>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AndroidWorkspaceValidation {
    pub manifest_path: String,
    pub manifest_present: bool,
    pub manifest_well_formed: bool,
    pub resource_files: Vec<String>,
    pub invalid_resource_files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RepairPattern {
    pub pattern_id: String,
    pub classifier: String,
    pub severity: String,
    pub likely_cause: String,
    pub allowed_scope: Vec<String>,
    pub preconditions: Vec<String>,
    pub operation_type: String,
    pub retry_budget: u8,
    pub checkpoint_rule: String,
    pub validation_command: String,
    pub evidence_requirements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RepairFailureFingerprint {
    pub classifier: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RepairSelection {
    pub pattern_id: String,
    pub classifier: String,
    pub allowed_scope: Vec<String>,
    pub preconditions: Vec<String>,
    pub retry_budget: u8,
    pub checkpoint_rule: String,
    pub validation_command: String,
    pub evidence_requirements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AndroidRepairRegistry {
    pub schema_version: u16,
    pub registry_id: String,
    pub patterns: Vec<RepairPattern>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum M47Error {
    EmptyField(&'static str),
    UnsupportedSchemaVersion,
    InvalidConfidence,
    InvalidRequirement(String),
    DuplicateRequirement(String),
    InvalidPattern(String),
    UnknownFailureClassifier(String),
}

impl fmt::Display for M47Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "M47 field is empty: {field}"),
            Self::UnsupportedSchemaVersion => {
                formatter.write_str("M47 schema version is unsupported")
            }
            Self::InvalidConfidence => formatter.write_str("M47 confidence is outside 0..=100"),
            Self::InvalidRequirement(id) => write!(formatter, "M47 requirement is invalid: {id}"),
            Self::DuplicateRequirement(id) => {
                write!(formatter, "M47 requirement is duplicated: {id}")
            }
            Self::InvalidPattern(id) => write!(formatter, "M47 repair pattern is invalid: {id}"),
            Self::UnknownFailureClassifier(classifier) => {
                write!(
                    formatter,
                    "M47 failure classifier is not registered: {classifier}"
                )
            }
        }
    }
}

impl std::error::Error for M47Error {}

impl AndroidRequirementManifest {
    pub fn validate(&self) -> Result<(), M47Error> {
        if self.schema_version != M47_SCHEMA_VERSION {
            return Err(M47Error::UnsupportedSchemaVersion);
        }
        for (field, value) in [
            ("manifestId", self.manifest_id.as_str()),
            ("projectId", self.project_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("projectFingerprint", self.project_fingerprint.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(M47Error::EmptyField(field));
            }
        }
        let mut ids = BTreeSet::new();
        for requirement in &self.requirements {
            if requirement.requirement_id.trim().is_empty()
                || requirement.subject.trim().is_empty()
                || requirement.desired_value.trim().is_empty()
                || requirement.source.trim().is_empty()
                || requirement.validation_rule.trim().is_empty()
            {
                return Err(M47Error::InvalidRequirement(
                    requirement.requirement_id.clone(),
                ));
            }
            if requirement.confidence_percent > 100 {
                return Err(M47Error::InvalidConfidence);
            }
            if !ids.insert(requirement.requirement_id.clone()) {
                return Err(M47Error::DuplicateRequirement(
                    requirement.requirement_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

impl AndroidRepairRegistry {
    pub fn validate(&self) -> Result<(), M47Error> {
        if self.schema_version != M47_SCHEMA_VERSION {
            return Err(M47Error::UnsupportedSchemaVersion);
        }
        if self.registry_id.trim().is_empty() {
            return Err(M47Error::EmptyField("registryId"));
        }
        let mut ids = BTreeSet::new();
        for pattern in &self.patterns {
            if pattern.pattern_id.trim().is_empty()
                || pattern.classifier.trim().is_empty()
                || pattern.likely_cause.trim().is_empty()
                || pattern.operation_type.trim().is_empty()
                || pattern.checkpoint_rule.trim().is_empty()
                || pattern.validation_command.trim().is_empty()
                || pattern.retry_budget == 0
            {
                return Err(M47Error::InvalidPattern(pattern.pattern_id.clone()));
            }
            if !ids.insert(pattern.pattern_id.clone()) {
                return Err(M47Error::InvalidPattern(pattern.pattern_id.clone()));
            }
        }
        Ok(())
    }

    pub fn select(&self, failure: &RepairFailureFingerprint) -> Result<RepairSelection, M47Error> {
        self.validate()?;
        let pattern = self
            .patterns
            .iter()
            .find(|pattern| pattern.classifier == failure.classifier)
            .ok_or_else(|| M47Error::UnknownFailureClassifier(failure.classifier.clone()))?;
        Ok(RepairSelection {
            pattern_id: pattern.pattern_id.clone(),
            classifier: pattern.classifier.clone(),
            allowed_scope: pattern.allowed_scope.clone(),
            preconditions: pattern.preconditions.clone(),
            retry_budget: pattern.retry_budget,
            checkpoint_rule: pattern.checkpoint_rule.clone(),
            validation_command: pattern.validation_command.clone(),
            evidence_requirements: pattern.evidence_requirements.clone(),
        })
    }
}

impl Default for AndroidRepairRegistry {
    fn default() -> Self {
        let families = [
            (
                "toolchain.missing",
                "repair locked toolchain component",
                "toolchain",
            ),
            (
                "dependency.conflict",
                "resolve dependency graph conflict",
                "dependencies",
            ),
            (
                "source.build.failure",
                "repair source or build configuration",
                "source",
            ),
            (
                "runtime.crash",
                "repair startup or runtime failure",
                "runtime",
            ),
            ("visual.failure", "repair visual regression", "preview"),
            (
                "accessibility.failure",
                "repair accessibility finding",
                "accessibility",
            ),
            ("emulator.failure", "repair emulator availability", "device"),
            ("adb.failure", "repair ADB connectivity", "device"),
            ("packaging.failure", "repair APK packaging", "packaging"),
            ("signing.failure", "repair signing configuration", "signing"),
        ];
        let patterns = families
            .into_iter()
            .map(|(classifier, cause, scope)| RepairPattern {
                pattern_id: format!("repair.{classifier}"),
                classifier: classifier.into(),
                severity: "blocking".into(),
                likely_cause: cause.into(),
                allowed_scope: vec![scope.into()],
                preconditions: vec![
                    "matching-failure-fingerprint".into(),
                    "active-checkpoint".into(),
                ],
                operation_type: "structured-authorized-repair".into(),
                retry_budget: 3,
                checkpoint_rule: "restore-before-repair-and-revalidate".into(),
                validation_command: "run-independent-validation".into(),
                evidence_requirements: vec![
                    "failure-fingerprint".into(),
                    "validation-result".into(),
                ],
            })
            .collect();
        Self {
            schema_version: M47_SCHEMA_VERSION,
            registry_id: "android-repair-registry-v1".into(),
            patterns,
        }
    }
}

pub fn infer_android_requirement_manifest(
    contract: &AndroidConstructionContract,
    index: &nirman_project::ProjectIndex,
    project_revision: u64,
) -> Result<AndroidRequirementManifest, M47Error> {
    contract
        .validate()
        .map_err(|_| M47Error::InvalidRequirement("contract".into()))?;
    if contract.target_platforms != vec!["android"] {
        return Err(M47Error::InvalidRequirement("targetPlatforms".into()));
    }
    let mut requirements = Vec::new();
    let has_manifest = index
        .files
        .iter()
        .any(|file| file.relative_path.ends_with("AndroidManifest.xml"));
    requirements.push(AndroidRequirement {
        requirement_id: "android.manifest.present".into(),
        kind: AndroidRequirementKind::Manifest,
        subject: "AndroidManifest.xml".into(),
        desired_value: "present".into(),
        observed_value: Some(if has_manifest { "present" } else { "missing" }.into()),
        source: "M39 AndroidConstructionContract and M45 ProjectIndex".into(),
        confidence_percent: 100,
        affected_files: vec!["app/src/main/AndroidManifest.xml".into()],
        validation_rule: "manifest file must exist in the Android source set".into(),
        status: if has_manifest {
            AndroidRequirementStatus::Satisfied
        } else {
            AndroidRequirementStatus::Missing
        },
        evidence_ids: vec!["m47.requirement.manifest".into()],
    });
    let manifest_permissions = index
        .graph
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, GraphNodeKind::Permission))
        .map(|node| node.label.clone())
        .collect::<BTreeSet<_>>();
    let requested_permissions = contract
        .device_matrix
        .iter()
        .flat_map(|device| device.permissions.iter().cloned())
        .collect::<BTreeSet<_>>();
    for permission in requested_permissions.union(&manifest_permissions) {
        let requested = requested_permissions.contains(permission);
        let observed = manifest_permissions.contains(permission);
        requirements.push(AndroidRequirement {
            requirement_id: format!("android.permission.{permission}"),
            kind: AndroidRequirementKind::Permission,
            subject: permission.clone(),
            desired_value: if requested {
                "declared"
            } else {
                "not-declared"
            }
            .into(),
            observed_value: Some(if observed { "declared" } else { "absent" }.into()),
            source: "M39 device matrix and M45 permission graph".into(),
            confidence_percent: 100,
            affected_files: vec!["app/src/main/AndroidManifest.xml".into()],
            validation_rule: "declared permissions must match the requested permission profile"
                .into(),
            status: if requested == observed {
                AndroidRequirementStatus::Satisfied
            } else if observed {
                AndroidRequirementStatus::Excessive
            } else {
                AndroidRequirementStatus::Missing
            },
            evidence_ids: vec![format!("m47.requirement.permission.{permission}")],
        });
    }
    let mut requirements = requirements;
    let generic_requirements = [
        (
            "android.sdk",
            AndroidRequirementKind::Sdk,
            "compileSdk",
            "declared by technology plan and device API level",
            vec!["build.gradle.kts"],
        ),
        (
            "android.abi",
            AndroidRequirementKind::Abi,
            "selected ABI",
            "declared by device matrix",
            vec!["device-profile"],
        ),
        (
            "android.service",
            AndroidRequirementKind::Service,
            "Android services",
            "declared by construction requirements when applicable",
            vec!["AndroidManifest.xml"],
        ),
        (
            "android.resource",
            AndroidRequirementKind::Resource,
            "Android resources",
            "declared by UI/assets requirements and indexed resource files",
            vec!["res/"],
        ),
        (
            "android.accessibility",
            AndroidRequirementKind::Accessibility,
            "accessibility behavior",
            "declared by construction requirements when applicable",
            vec!["res/", "app/src/"],
        ),
        (
            "android.localization",
            AndroidRequirementKind::Localization,
            "localization behavior",
            "declared by device locale and construction requirements",
            vec!["res/"],
        ),
        (
            "android.background-behavior",
            AndroidRequirementKind::BackgroundBehavior,
            "background behavior",
            "declared by construction requirements when applicable",
            vec!["app/src/"],
        ),
        (
            "android.release",
            AndroidRequirementKind::Release,
            "APK delivery",
            "required by artifact model",
            vec!["artifact-model"],
        ),
    ];
    let contract_text = format!(
        "{} {} {} {}",
        contract.user_intent,
        contract
            .features
            .iter()
            .map(|item| item.statement.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        contract
            .ui
            .iter()
            .map(|item| item.statement.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        contract
            .android_requirements
            .iter()
            .map(|item| item.statement.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_ascii_lowercase();
    for (requirement_id, kind, subject, desired, paths) in generic_requirements {
        let explicitly_requested = match kind {
            AndroidRequirementKind::Service => contract_text.contains("service"),
            AndroidRequirementKind::Resource => {
                !contract.assets.is_empty() || contract_text.contains("resource")
            }
            AndroidRequirementKind::Accessibility => contract_text.contains("accessib"),
            AndroidRequirementKind::Localization => {
                contract
                    .device_matrix
                    .iter()
                    .any(|device| !device.locale.is_empty())
                    || contract_text.contains("localiz")
                    || contract_text.contains("locale")
            }
            AndroidRequirementKind::BackgroundBehavior => {
                contract_text.contains("background") || contract_text.contains("doze")
            }
            _ => true,
        };
        let resource_nodes = index
            .graph
            .nodes
            .iter()
            .filter(|node| node.kind == GraphNodeKind::Resource)
            .count();
        let observed = match kind {
            AndroidRequirementKind::Resource if resource_nodes > 0 => {
                Some(format!("{resource_nodes} indexed resource node(s)"))
            }
            AndroidRequirementKind::Localization => Some(
                contract
                    .device_matrix
                    .iter()
                    .map(|device| device.locale.as_str())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            _ => None,
        };
        let status = if matches!(kind, AndroidRequirementKind::Resource) && explicitly_requested {
            if resource_nodes > 0 {
                AndroidRequirementStatus::Satisfied
            } else {
                AndroidRequirementStatus::Missing
            }
        } else {
            AndroidRequirementStatus::Unknown
        };
        requirements.push(AndroidRequirement {
            requirement_id: requirement_id.into(),
            kind,
            subject: subject.into(),
            desired_value: desired.into(),
            observed_value: observed,
            source: "M39 AndroidConstructionContract and M45 ProjectIndex".into(),
            confidence_percent: if explicitly_requested { 100 } else { 60 },
            affected_files: paths.into_iter().map(String::from).collect(),
            validation_rule: "requirement must be independently validated before promotion".into(),
            status,
            evidence_ids: vec![format!("m47.requirement.{requirement_id}")],
        });
    }
    requirements.sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
    let manifest = AndroidRequirementManifest {
        schema_version: M47_SCHEMA_VERSION,
        manifest_id: format!(
            "requirements-{}-{}",
            contract.project_id.0, contract.task_id.0
        ),
        project_id: contract.project_id.0.clone(),
        task_id: contract.task_id.0.clone(),
        project_revision,
        project_fingerprint: index.project_fingerprint.clone(),
        requirements,
    };
    manifest.validate()?;
    Ok(manifest)
}

pub fn validate_android_workspace(
    root: impl AsRef<Path>,
    index: &nirman_project::ProjectIndex,
) -> std::io::Result<AndroidWorkspaceValidation> {
    let root = root.as_ref().canonicalize()?;
    let manifest_relative = index
        .files
        .iter()
        .find(|file| file.relative_path.ends_with("AndroidManifest.xml"))
        .map(|file| file.relative_path.clone())
        .unwrap_or_else(|| "app/src/main/AndroidManifest.xml".into());
    let manifest_path = root.join(&manifest_relative);
    let manifest_present = manifest_path.is_file();
    let manifest_well_formed = if manifest_present {
        let text = fs::read_to_string(&manifest_path)?;
        let trimmed = text.trim();
        trimmed.starts_with("<manifest")
            && trimmed.contains("</manifest>")
            && trimmed.contains("<application")
    } else {
        false
    };
    let mut resource_files = Vec::new();
    let mut invalid_resource_files = Vec::new();
    for file in &index.files {
        if file.kind != nirman_project::FileKind::Resource {
            continue;
        }
        let path = file.relative_path.clone();
        let unsafe_path = path.starts_with('/')
            || path.split('/').any(|component| component == "..")
            || path.contains('\\');
        if unsafe_path {
            invalid_resource_files.push(path);
        } else {
            resource_files.push(path);
        }
    }
    resource_files.sort();
    invalid_resource_files.sort();
    Ok(AndroidWorkspaceValidation {
        manifest_path: manifest_relative,
        manifest_present,
        manifest_well_formed,
        resource_files,
        invalid_resource_files,
    })
}

pub const M4_SCHEMA_VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidTechnologyPlan {
    pub schema_version: u16,
    pub plan_id: String,
    pub language: String,
    pub ui_framework: String,
    pub data_strategy: String,
    pub source_revision: u64,
    pub rationale: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidSynthesisRequest {
    pub schema_version: u16,
    pub contract: AndroidConstructionContract,
    pub source_revision: u64,
    pub workspace_root: String,
    pub project_fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidSynthesisPlan {
    pub schema_version: u16,
    pub contract_id: String,
    pub project_id: String,
    pub task_id: String,
    pub target_platforms: Vec<String>,
    pub technology_plan: AndroidTechnologyPlan,
    pub gradle_tasks: Vec<String>,
    pub required_components: Vec<ToolchainComponentKind>,
    pub checkpoint_required: bool,
    pub workspace_root: String,
    pub source_revision: u64,
    pub project_fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidBuildRequest {
    pub schema_version: u16,
    pub project_id: String,
    pub task_id: String,
    pub source_revision: u64,
    pub project_fingerprint: String,
    pub workspace_root: String,
    pub build_variant: String,
    pub gradle_task: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidLocalRuntimeCapabilities {
    pub java_available: bool,
    pub gradle_available: bool,
    pub android_sdk_available: bool,
    pub adb_available: bool,
    pub emulator_available: bool,
    pub native_runtime_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum M4Error {
    InvalidContract,
    UnsupportedPlatform,
    EmptyField(&'static str),
    StaleRevision,
}
impl fmt::Display for M4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract => f.write_str("M4 Android construction contract is invalid"),
            Self::UnsupportedPlatform => f.write_str("M4 supports Android only"),
            Self::EmptyField(v) => write!(f, "M4 field is empty: {v}"),
            Self::StaleRevision => f.write_str("M4 source revision is stale"),
        }
    }
}
impl std::error::Error for M4Error {}

pub fn synthesize_android_plan(
    request: &AndroidSynthesisRequest,
) -> Result<AndroidSynthesisPlan, M4Error> {
    if request.schema_version != M4_SCHEMA_VERSION {
        return Err(M4Error::InvalidContract);
    }
    request
        .contract
        .validate()
        .map_err(|_| M4Error::InvalidContract)?;
    if request.contract.target_platforms != vec!["android".to_string()] {
        return Err(M4Error::UnsupportedPlatform);
    }
    for (name, value) in [
        ("workspaceRoot", request.workspace_root.as_str()),
        ("projectFingerprint", request.project_fingerprint.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(M4Error::EmptyField(name));
        }
    }
    if request.source_revision == 0 {
        return Err(M4Error::StaleRevision);
    }
    let language = if request
        .contract
        .user_intent
        .to_ascii_lowercase()
        .contains("java")
    {
        "java"
    } else {
        "kotlin"
    };
    let ui_framework = if request
        .contract
        .user_intent
        .to_ascii_lowercase()
        .contains("view")
    {
        "android-views"
    } else {
        "jetpack-compose"
    };
    let plan = AndroidTechnologyPlan {
        schema_version: M4_SCHEMA_VERSION,
        plan_id: format!("technology-plan-{}", request.contract.contract_id),
        language: language.into(),
        ui_framework: ui_framework.into(),
        data_strategy: "requirements-resolved".into(),
        source_revision: request.source_revision,
        rationale: "selected from validated Android intent and available contract evidence".into(),
    };
    Ok(AndroidSynthesisPlan {
        schema_version: M4_SCHEMA_VERSION,
        contract_id: request.contract.contract_id.clone(),
        project_id: request.contract.project_id.0.clone(),
        task_id: request.contract.task_id.0.clone(),
        target_platforms: vec!["android".into()],
        technology_plan: plan,
        gradle_tasks: vec!["assembleDebug".into()],
        required_components: vec![
            ToolchainComponentKind::Jdk,
            ToolchainComponentKind::Gradle,
            ToolchainComponentKind::AndroidSdk,
            ToolchainComponentKind::BuildTools,
        ],
        checkpoint_required: true,
        workspace_root: request.workspace_root.clone(),
        source_revision: request.source_revision,
        project_fingerprint: request.project_fingerprint.clone(),
    })
}

pub fn validate_android_build_request(request: &AndroidBuildRequest) -> Result<(), M4Error> {
    if request.schema_version != M4_SCHEMA_VERSION {
        return Err(M4Error::InvalidContract);
    }
    for (name, value) in [
        ("projectId", request.project_id.as_str()),
        ("taskId", request.task_id.as_str()),
        ("projectFingerprint", request.project_fingerprint.as_str()),
        ("workspaceRoot", request.workspace_root.as_str()),
        ("buildVariant", request.build_variant.as_str()),
        ("gradleTask", request.gradle_task.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(M4Error::EmptyField(name));
        }
    }
    if request.source_revision == 0 || !request.gradle_task.starts_with("assemble") {
        return Err(M4Error::InvalidContract);
    }
    Ok(())
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
    fn m4_synthesis_selects_android_plan_and_gradle_build_contract() {
        let request = AndroidSynthesisRequest {
            schema_version: M4_SCHEMA_VERSION,
            contract: contract(),
            source_revision: 1,
            workspace_root: "/workspace/android".into(),
            project_fingerprint: "fingerprint-1".into(),
        };
        let plan = synthesize_android_plan(&request).expect("M4 plan");
        assert_eq!(plan.target_platforms, vec!["android"]);
        assert_eq!(plan.technology_plan.ui_framework, "jetpack-compose");
        assert_eq!(plan.gradle_tasks, vec!["assembleDebug"]);
        assert!(plan.checkpoint_required);
        let build = AndroidBuildRequest {
            schema_version: M4_SCHEMA_VERSION,
            project_id: plan.project_id,
            task_id: plan.task_id,
            source_revision: plan.source_revision,
            project_fingerprint: plan.project_fingerprint,
            workspace_root: plan.workspace_root,
            build_variant: "debug".into(),
            gradle_task: "assembleDebug".into(),
        };
        validate_android_build_request(&build).expect("M4 build request");
    }

    #[test]
    fn m4_rejects_non_android_and_stale_synthesis_requests() {
        let mut request = AndroidSynthesisRequest {
            schema_version: M4_SCHEMA_VERSION,
            contract: contract(),
            source_revision: 1,
            workspace_root: "/workspace/android".into(),
            project_fingerprint: "fingerprint-1".into(),
        };
        request.contract.target_platforms = vec!["web".into()];
        assert_eq!(
            synthesize_android_plan(&request),
            Err(M4Error::InvalidContract)
        );
        request.contract.target_platforms = vec!["android".into()];
        request.source_revision = 0;
        assert_eq!(
            synthesize_android_plan(&request),
            Err(M4Error::StaleRevision)
        );
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

pub const M5_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Default)]
pub struct BuildCancellation {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl BuildCancellation {
    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AndroidBuildObservation {
    pub schema_version: u16,
    pub execution_id: String,
    pub command_id: String,
    pub project_id: String,
    pub task_id: String,
    pub source_revision: u64,
    pub project_fingerprint: String,
    pub workspace_root: String,
    pub build_variant: String,
    pub gradle_task: String,
    pub executable: String,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub cancelled: bool,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub artifact_path: Option<String>,
    pub artifact_sha256: Option<String>,
    pub started_at_epoch_seconds: u64,
    pub completed_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidBuildExecutionError {
    InvalidRequest(M4Error),
    InvalidLock(AndroidToolchainError),
    EmptyCommandId,
    WorkspaceUnavailable,
    GradleUnavailable,
    SpawnFailed,
    OutputReadFailed,
}

impl fmt::Display for AndroidBuildExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(error) => {
                write!(formatter, "M5 build request is invalid: {error}")
            }
            Self::InvalidLock(error) => write!(formatter, "M5 toolchain lock is invalid: {error}"),
            Self::EmptyCommandId => formatter.write_str("M5 build command identity is empty"),
            Self::WorkspaceUnavailable => formatter.write_str("M5 build workspace is unavailable"),
            Self::GradleUnavailable => {
                formatter.write_str("M5 locked Gradle executable is unavailable")
            }
            Self::SpawnFailed => formatter.write_str("M5 Gradle process could not be started"),
            Self::OutputReadFailed => formatter.write_str("M5 Gradle output could not be captured"),
        }
    }
}

impl std::error::Error for AndroidBuildExecutionError {}

pub fn execute_android_build(
    request: &AndroidBuildRequest,
    lock: &AndroidToolchainLock,
    command_id: &str,
    timeout_ms: u64,
    cancellation: &BuildCancellation,
) -> Result<AndroidBuildObservation, AndroidBuildExecutionError> {
    validate_android_build_request(request).map_err(AndroidBuildExecutionError::InvalidRequest)?;
    lock.validate()
        .map_err(AndroidBuildExecutionError::InvalidLock)?;
    if command_id.trim().is_empty() || timeout_ms == 0 {
        return Err(AndroidBuildExecutionError::EmptyCommandId);
    }
    let workspace = Path::new(&request.workspace_root)
        .canonicalize()
        .map_err(|_| AndroidBuildExecutionError::WorkspaceUnavailable)?;
    if !workspace.is_dir() {
        return Err(AndroidBuildExecutionError::WorkspaceUnavailable);
    }

    let wrapper = if cfg!(windows) {
        workspace.join("gradlew.bat")
    } else {
        workspace.join("gradlew")
    };
    let (program, mut arguments) = if wrapper.is_file() {
        if cfg!(windows) {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), wrapper.to_string_lossy().into_owned()],
            )
        } else {
            (wrapper.to_string_lossy().into_owned(), Vec::new())
        }
    } else {
        let locked_gradle = lock
            .entries
            .iter()
            .find(|entry| entry.component == ToolchainComponentKind::Gradle)
            .map(|entry| PathBuf::from(&entry.path))
            .ok_or(AndroidBuildExecutionError::GradleUnavailable)?;
        if locked_gradle.as_os_str().is_empty() {
            return Err(AndroidBuildExecutionError::GradleUnavailable);
        }
        let executable = if locked_gradle.is_file() {
            locked_gradle.to_string_lossy().into_owned()
        } else {
            locked_gradle.to_string_lossy().into_owned()
        };
        (executable, Vec::new())
    };
    arguments.extend([
        "--no-daemon".to_string(),
        "--console=plain".to_string(),
        request.gradle_task.clone(),
    ]);

    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut command = Command::new(&program);
    command
        .args(&arguments)
        .current_dir(&workspace)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => AndroidBuildExecutionError::GradleUnavailable,
        _ => AndroidBuildExecutionError::SpawnFailed,
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or(AndroidBuildExecutionError::OutputReadFailed)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(AndroidBuildExecutionError::OutputReadFailed)?;
    let stdout_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::BufReader::new(stdout), &mut bytes).map(|_| bytes)
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::BufReader::new(stderr), &mut bytes).map(|_| bytes)
    });

    let started = std::time::Instant::now();
    let mut timed_out = false;
    let mut cancelled = false;
    let exit_code = loop {
        if cancellation.is_cancelled() {
            cancelled = true;
            let _ = child.kill();
            break child.wait().ok().and_then(|status| status.code());
        }
        if started.elapsed() >= std::time::Duration::from_millis(timeout_ms) {
            timed_out = true;
            let _ = child.kill();
            break child.wait().ok().and_then(|status| status.code());
        }
        match child
            .try_wait()
            .map_err(|_| AndroidBuildExecutionError::SpawnFailed)?
        {
            Some(status) => break status.code(),
            None => std::thread::sleep(std::time::Duration::from_millis(25)),
        }
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| AndroidBuildExecutionError::OutputReadFailed)?
        .map_err(|_| AndroidBuildExecutionError::OutputReadFailed)?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| AndroidBuildExecutionError::OutputReadFailed)?
        .map_err(|_| AndroidBuildExecutionError::OutputReadFailed)?;
    let artifact = if exit_code == Some(0) && !timed_out && !cancelled {
        find_apk(&workspace)
    } else {
        None
    };
    let completed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(AndroidBuildObservation {
        schema_version: M5_SCHEMA_VERSION,
        execution_id: format!("build-execution-{command_id}"),
        command_id: command_id.into(),
        project_id: request.project_id.clone(),
        task_id: request.task_id.clone(),
        source_revision: request.source_revision,
        project_fingerprint: request.project_fingerprint.clone(),
        workspace_root: workspace.to_string_lossy().into_owned(),
        build_variant: request.build_variant.clone(),
        gradle_task: request.gradle_task.clone(),
        executable: program,
        exit_code,
        success: exit_code == Some(0) && !timed_out && !cancelled,
        timed_out,
        cancelled,
        stdout_sha256: hash_bytes(&stdout),
        stderr_sha256: hash_bytes(&stderr),
        stdout_bytes: stdout.len() as u64,
        stderr_bytes: stderr.len() as u64,
        artifact_path: artifact.as_ref().map(|(path, _)| path.clone()),
        artifact_sha256: artifact.map(|(_, hash)| hash),
        started_at_epoch_seconds: started_at,
        completed_at_epoch_seconds: completed_at,
    })
}

fn find_apk(workspace: &Path) -> Option<(String, String)> {
    let outputs = workspace.join("app").join("build").join("outputs");
    let mut stack = vec![outputs];
    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(path).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("apk") {
                let bytes = fs::read(&path).ok()?;
                return Some((path.to_string_lossy().into_owned(), hash_bytes(&bytes)));
            }
        }
    }
    None
}

fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod build_execution_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(name: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("nirman-{name}-{suffix}"));
        fs::create_dir_all(path.join("app/build/outputs/apk/debug")).expect("workspace");
        path
    }

    fn request(workspace: &Path) -> AndroidBuildRequest {
        AndroidBuildRequest {
            schema_version: M4_SCHEMA_VERSION,
            project_id: "project-test".into(),
            task_id: "task-test".into(),
            source_revision: 1,
            project_fingerprint: "fingerprint-test".into(),
            workspace_root: workspace.to_string_lossy().into_owned(),
            build_variant: "debug".into(),
            gradle_task: "assembleDebug".into(),
        }
    }

    fn lock(executable: &Path) -> AndroidToolchainLock {
        AndroidToolchainLock {
            schema_version: M43_SCHEMA_VERSION,
            lock_id: "lock-test".into(),
            manifest_id: "manifest-test".into(),
            entries: vec![ToolchainLockEntry {
                component: ToolchainComponentKind::Gradle,
                version: "test".into(),
                path: executable.to_string_lossy().into_owned(),
                binary_hash: "hash-test".into(),
                license_id: "license-test".into(),
                acquisition_source: "test-only".into(),
            }],
            lock_hash: "lock-hash-test".into(),
        }
    }

    #[cfg(unix)]
    fn write_gradle_script(workspace: &Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = workspace.join("gradlew");
        fs::write(&script, format!("#!/bin/sh\n{body}\n")).expect("script");
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("permissions");
        script
    }

    #[cfg(unix)]
    #[test]
    fn real_executor_observes_success_and_apk_fingerprint() {
        let workspace = temp_workspace("build-success");
        let apk = workspace.join("app/build/outputs/apk/debug/app-debug.apk");
        fs::write(&apk, b"real-test-apk-bytes").expect("apk");
        let wrapper = write_gradle_script(&workspace, "printf 'BUILD SUCCESSFUL\\n'");
        let result = execute_android_build(
            &request(&workspace),
            &lock(&wrapper),
            "command-success",
            5_000,
            &BuildCancellation::default(),
        )
        .expect("build observation");
        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(
            result.artifact_path,
            Some(apk.to_string_lossy().into_owned())
        );
        assert_eq!(
            result.artifact_sha256,
            Some(hash_bytes(b"real-test-apk-bytes"))
        );
        assert!(result.stdout_bytes > 0);
        let _ = fs::remove_dir_all(workspace);
    }

    #[cfg(unix)]
    #[test]
    fn real_executor_records_timeout_without_success() {
        let workspace = temp_workspace("build-timeout");
        let wrapper = write_gradle_script(&workspace, "sleep 1");
        let result = execute_android_build(
            &request(&workspace),
            &lock(&wrapper),
            "command-timeout",
            50,
            &BuildCancellation::default(),
        )
        .expect("timeout observation");
        assert!(!result.success);
        assert!(result.timed_out);
        assert!(!result.cancelled);
        let _ = fs::remove_dir_all(workspace);
    }

    #[cfg(unix)]
    #[test]
    fn real_executor_records_cancellation_without_success() {
        let workspace = temp_workspace("build-cancel");
        let wrapper = write_gradle_script(&workspace, "sleep 1");
        let cancellation = BuildCancellation::default();
        let trigger = cancellation.clone();
        let result_thread = std::thread::spawn(move || {
            execute_android_build(
                &request(&workspace),
                &lock(&wrapper),
                "command-cancel",
                5_000,
                &cancellation,
            )
            .expect("cancellation observation")
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        trigger.cancel();
        let result = result_thread.join().expect("build thread");
        assert!(!result.success);
        assert!(result.cancelled);
        assert!(!result.timed_out);
    }
}
