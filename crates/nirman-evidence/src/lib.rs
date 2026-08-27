#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    pub runtime_session_id: String,
    pub package_name: String,
    pub apk_sha256: String,
    pub install_status: String,
    pub launch_status: String,
    pub interaction_status: String,
    pub logcat_reference: Option<String>,
    pub screenshot_references: Vec<String>,
    pub accessibility_reference: Option<String>,
    pub visual_comparison_reference: Option<String>,
    pub permission_result_reference: Option<String>,
    pub crash_trace_reference: Option<String>,
    pub observed_at_epoch_seconds: u64,
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
            ("runtimeSessionId", &self.runtime_session_id),
            ("packageName", &self.package_name),
            ("apkSha256", &self.apk_sha256),
        ] {
            if v.trim().is_empty() {
                return Err(M9Error::EmptyField(n));
            }
        }
        if !self.synthetic_data_only {
            return Err(M9Error::MissingSyntheticData);
        }
        if !self.apk_sha256.starts_with("sha256:") || self.observed_at_epoch_seconds == 0 {
            return Err(M9Error::InvalidObservation);
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
            runtime_session_id: "session-1".into(),
            package_name: "com.example.app".into(),
            apk_sha256: "sha256:fixture".into(),
            install_status: "OBSERVED_SUCCESS".into(),
            launch_status: "OBSERVED_SUCCESS".into(),
            interaction_status: "OBSERVED_PASS".into(),
            logcat_reference: Some("logcat-1".into()),
            screenshot_references: vec!["screen-1".into()],
            accessibility_reference: Some("a11y-1".into()),
            visual_comparison_reference: Some("visual-1".into()),
            permission_result_reference: Some("permission-1".into()),
            crash_trace_reference: None,
            observed_at_epoch_seconds: 1,
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
            runtime_session_id: "session-d".into(),
            package_name: "pkg".into(),
            apk_sha256: "sha256:fixture".into(),
            install_status: "".into(),
            launch_status: "".into(),
            interaction_status: "".into(),
            logcat_reference: None,
            screenshot_references: vec![],
            accessibility_reference: None,
            visual_comparison_reference: None,
            permission_result_reference: None,
            crash_trace_reference: None,
            observed_at_epoch_seconds: 1,
            synthetic_data_only: false,
        };
        assert_eq!(o.validate(), Err(M9Error::MissingSyntheticData));
        o.synthetic_data_only = true;
        assert_eq!(o.validate(), Err(M9Error::InvalidObservation));
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceSessionRequest {
    pub observation_id: String,
    pub project_id: String,
    pub task_id: String,
    pub project_revision_id: String,
    pub profile: AndroidDeviceProfile,
    pub package_name: String,
    pub apk_path: String,
    pub adb_executable: String,
    pub evidence_directory: String,
    pub timeout_ms: u64,
    pub selected_device_identity: Option<String>,
    pub synthetic_device_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceSessionError {
    InvalidRequest,
    DeviceUnavailable,
    CommandTimeout,
    CommandFailed,
    EvidenceWriteFailed,
}

impl fmt::Display for DeviceSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest => formatter.write_str("M9 device-session request is invalid"),
            Self::DeviceUnavailable => formatter.write_str("M9 Android device is unavailable"),
            Self::CommandTimeout => formatter.write_str("M9 Android device command timed out"),
            Self::CommandFailed => formatter.write_str("M9 Android device command failed"),
            Self::EvidenceWriteFailed => formatter.write_str("M9 evidence could not be written"),
        }
    }
}

impl std::error::Error for DeviceSessionError {}

pub fn execute_android_device_session(
    request: &DeviceSessionRequest,
) -> Result<AndroidDeviceObservation, DeviceSessionError> {
    request
        .profile
        .validate()
        .map_err(|_| DeviceSessionError::InvalidRequest)?;
    if !request.synthetic_device_only
        || request.observation_id.trim().is_empty()
        || request.project_id.trim().is_empty()
        || request.task_id.trim().is_empty()
        || request.project_revision_id.trim().is_empty()
        || request.package_name.trim().is_empty()
        || request.apk_path.trim().is_empty()
        || request.adb_executable.trim().is_empty()
        || request.evidence_directory.trim().is_empty()
        || request.timeout_ms == 0
    {
        return Err(DeviceSessionError::InvalidRequest);
    }
    let apk_bytes =
        std::fs::read(&request.apk_path).map_err(|_| DeviceSessionError::InvalidRequest)?;
    let apk_sha256 = format!("sha256:{:x}", Sha256::digest(&apk_bytes));
    std::fs::create_dir_all(&request.evidence_directory)
        .map_err(|_| DeviceSessionError::EvidenceWriteFailed)?;
    let state = run_adb(request, &["get-state"])?;
    if !state.stdout.contains("device") {
        return Err(DeviceSessionError::DeviceUnavailable);
    }
    let serial = run_adb(request, &["get-serialno"])?
        .stdout
        .trim()
        .to_owned();
    if serial.is_empty() || serial == "unknown" {
        return Err(DeviceSessionError::DeviceUnavailable);
    }
    if request
        .selected_device_identity
        .as_ref()
        .is_some_and(|selected| selected != &serial)
    {
        return Err(DeviceSessionError::DeviceUnavailable);
    }
    run_adb(request, &["install", "-r", &request.apk_path])?;
    run_adb(
        request,
        &["shell", "monkey", "-p", &request.package_name, "1"],
    )?;
    let logcat = run_adb(request, &["logcat", "-d", "-v", "brief", "-t", "200"])?;
    let sanitized_logcat = logcat
        .stdout
        .lines()
        .filter(|line| line.contains(&request.package_name) || line.contains("AndroidRuntime"))
        .collect::<Vec<_>>()
        .join("\n");
    let evidence_dir = std::path::Path::new(&request.evidence_directory);
    let logcat_path = evidence_dir.join("logcat.txt");
    let crash_trace_reference = sanitized_logcat
        .contains("AndroidRuntime")
        .then(|| logcat_path.to_string_lossy().into_owned());
    std::fs::write(&logcat_path, &sanitized_logcat)
        .map_err(|_| DeviceSessionError::EvidenceWriteFailed)?;
    let permission_output = run_adb(
        request,
        &["shell", "dumpsys", "package", &request.package_name],
    )?;
    let permission_path = evidence_dir.join("permissions.txt");
    std::fs::write(&permission_path, permission_output.stdout)
        .map_err(|_| DeviceSessionError::EvidenceWriteFailed)?;
    let screenshot = run_adb(request, &["exec-out", "screencap", "-p"])?;
    let screenshot_path = evidence_dir.join("screenshot.png");
    std::fs::write(&screenshot_path, screenshot.raw_stdout)
        .map_err(|_| DeviceSessionError::EvidenceWriteFailed)?;
    run_adb(
        request,
        &["shell", "uiautomator", "dump", "/sdcard/nirman-window.xml"],
    )?;
    let hierarchy = run_adb(request, &["exec-out", "cat", "/sdcard/nirman-window.xml"])?;
    let hierarchy_path = evidence_dir.join("ui-hierarchy.xml");
    std::fs::write(&hierarchy_path, hierarchy.raw_stdout)
        .map_err(|_| DeviceSessionError::EvidenceWriteFailed)?;
    let interaction = run_adb(
        request,
        &["shell", "am", "force-stop", &request.package_name],
    )?;
    let _ = interaction;
    let observation = AndroidDeviceObservation {
        schema_version: M9_SCHEMA_VERSION,
        observation_id: request.observation_id.clone(),
        project_id: request.project_id.clone(),
        task_id: request.task_id.clone(),
        project_revision_id: request.project_revision_id.clone(),
        device_profile_id: request.profile.profile_id.clone(),
        device_identity: serial,
        package_name: request.package_name.clone(),
        install_status: "OBSERVED_SUCCESS".into(),
        launch_status: "OBSERVED_SUCCESS".into(),
        runtime_session_id: format!("m9-session-{}", request.observation_id),
        apk_sha256,
        interaction_status: "OBSERVED_FORCE_STOP".into(),
        logcat_reference: Some(logcat_path.to_string_lossy().into_owned()),
        screenshot_references: vec![screenshot_path.to_string_lossy().into_owned()],
        accessibility_reference: Some(hierarchy_path.to_string_lossy().into_owned()),
        visual_comparison_reference: None,
        permission_result_reference: Some(permission_path.to_string_lossy().into_owned()),
        crash_trace_reference,
        observed_at_epoch_seconds: epoch_seconds(),
        synthetic_data_only: true,
    };
    observation
        .validate()
        .map_err(|_| DeviceSessionError::EvidenceWriteFailed)?;
    Ok(observation)
}

struct AdbOutput {
    stdout: String,
    raw_stdout: Vec<u8>,
}

fn epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn run_adb(
    request: &DeviceSessionRequest,
    arguments: &[&str],
) -> Result<AdbOutput, DeviceSessionError> {
    let mut child = std::process::Command::new(&request.adb_executable)
        .args(arguments)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| DeviceSessionError::DeviceUnavailable)?;
    let start = std::time::Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|_| DeviceSessionError::CommandFailed)?
        {
            Some(status) => break status,
            None if start.elapsed() >= std::time::Duration::from_millis(request.timeout_ms) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DeviceSessionError::CommandTimeout);
            }
            None => std::thread::sleep(std::time::Duration::from_millis(25)),
        }
    };
    let mut stdout = child
        .stdout
        .take()
        .ok_or(DeviceSessionError::CommandFailed)?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut stdout, &mut bytes)
        .map_err(|_| DeviceSessionError::CommandFailed)?;
    if !status.success() {
        return Err(DeviceSessionError::CommandFailed);
    }
    Ok(AdbOutput {
        stdout: String::from_utf8_lossy(&bytes).into_owned(),
        raw_stdout: bytes,
    })
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    #[test]
    fn disposable_adb_session_captures_runtime_evidence() {
        use std::os::unix::fs::PermissionsExt;
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nirman-m9-session-{suffix}"));
        fs::create_dir_all(&root).expect("root");
        let apk = root.join("fixture.apk");
        fs::write(&apk, b"fixture apk").expect("apk");
        let adb = root.join("adb");
        fs::write(
            &adb,
            "#!/bin/sh\ncase \"$1\" in\nget-state) printf 'device\\n' ;;\nget-serialno) printf 'emulator-test\\n' ;;\ninstall) exit 0 ;;\nlogcat) printf 'I/com.nirman.fixture: synthetic runtime\\n' ;;\nexec-out) if [ \"$2\" = \"screencap\" ]; then printf 'PNG' ; else printf '<hierarchy package=\"com.nirman.fixture\"/>' ; fi ;;\nshell) if [ \"$2\" = \"uiautomator\" ]; then exit 0; elif [ \"$2\" = \"am\" ]; then exit 0; else printf 'ok' ; fi ;;\n*) exit 1 ;;\nesac\n",
        )
        .expect("adb");
        let mut permissions = fs::metadata(&adb).expect("adb metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&adb, permissions).expect("adb permissions");
        let profile = AndroidDeviceProfile {
            profile_id: "pixel-api-35".into(),
            name: "Disposable Pixel API 35".into(),
            api_level: 35,
            architecture: "x86_64".into(),
            width: 1080,
            height: 2400,
            density: 420,
            orientation: "portrait".into(),
            locale: "en-US".into(),
        };
        let observation = execute_android_device_session(&DeviceSessionRequest {
            observation_id: "observation-session-test".into(),
            project_id: "project-test".into(),
            task_id: "task-test".into(),
            project_revision_id: "source-1".into(),
            profile,
            package_name: "com.nirman.fixture".into(),
            apk_path: apk.to_string_lossy().into_owned(),
            adb_executable: adb.to_string_lossy().into_owned(),
            evidence_directory: root.join("evidence").to_string_lossy().into_owned(),
            timeout_ms: 5_000,
            selected_device_identity: Some("emulator-test".into()),
            synthetic_device_only: true,
        })
        .expect("device observation");
        assert_eq!(observation.device_identity, "emulator-test");
        assert_eq!(observation.install_status, "OBSERVED_SUCCESS");
        assert_eq!(observation.launch_status, "OBSERVED_SUCCESS");
        assert_eq!(observation.interaction_status, "OBSERVED_FORCE_STOP");
        assert!(observation.logcat_reference.is_some());
        assert_eq!(observation.screenshot_references.len(), 1);
        assert!(observation.accessibility_reference.is_some());
        observation.validate().expect("valid observation");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    fn executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("permissions");
    }

    #[cfg(unix)]
    fn profile() -> AndroidDeviceProfile {
        AndroidDeviceProfile {
            profile_id: "pixel-api-35".into(),
            name: "Disposable Pixel API 35".into(),
            api_level: 35,
            architecture: "x86_64".into(),
            width: 1080,
            height: 2400,
            density: 420,
            orientation: "portrait".into(),
            locale: "en-US".into(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn selected_device_mismatch_and_adb_timeout_are_typed_and_bounded() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nirman-m9-failures-{suffix}"));
        fs::create_dir_all(&root).expect("root");
        let apk = root.join("fixture.apk");
        fs::write(&apk, b"fixture apk").expect("apk");
        let adb = root.join("adb");
        fs::write(
            &adb,
            "#!/bin/sh\ncase \"$1\" in\nget-state) printf 'device\\n' ;;\nget-serialno) printf 'actual-device\\n' ;;\n*) exit 0 ;;\nesac\n",
        )
        .expect("adb");
        executable(&adb);
        let base = DeviceSessionRequest {
            observation_id: "observation-failure".into(),
            project_id: "project-test".into(),
            task_id: "task-test".into(),
            project_revision_id: "source-1".into(),
            profile: profile(),
            package_name: "com.nirman.fixture".into(),
            apk_path: apk.to_string_lossy().into_owned(),
            adb_executable: adb.to_string_lossy().into_owned(),
            evidence_directory: root.join("evidence").to_string_lossy().into_owned(),
            timeout_ms: 1_000,
            selected_device_identity: Some("different-device".into()),
            synthetic_device_only: true,
        };
        assert_eq!(
            execute_android_device_session(&base),
            Err(DeviceSessionError::DeviceUnavailable)
        );
        let timeout_adb = root.join("adb-timeout");
        fs::write(
            &timeout_adb,
            "#!/bin/sh\nprintf 'device\\n'\nwhile :; do :; done\n",
        )
        .expect("timeout adb");
        executable(&timeout_adb);
        let timeout_request = DeviceSessionRequest {
            adb_executable: timeout_adb.to_string_lossy().into_owned(),
            selected_device_identity: Some("actual-device".into()),
            timeout_ms: 500,
            ..base
        };
        assert_eq!(
            execute_android_device_session(&timeout_request),
            Err(DeviceSessionError::CommandTimeout)
        );
        let _ = fs::remove_dir_all(root);
    }
}
