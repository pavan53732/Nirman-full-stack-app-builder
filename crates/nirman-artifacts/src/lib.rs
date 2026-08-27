#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, fs};
pub const M10_SCHEMA_VERSION: u16 = 1;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApkArtifact {
    pub schema_version: u16,
    pub artifact_id: String,
    pub project_id: String,
    pub task_id: String,
    pub project_revision_id: String,
    pub source_fingerprint: String,
    pub path: String,
    pub sha256: String,
    pub package_name: String,
    pub build_variant: String,
    pub secret_scan_status: String,
    pub signing_status: String,
    pub delivery_status: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum M10Error {
    InvalidArtifact,
    NotApk,
    MissingFile,
    HashMismatch,
    SecretScanFailed,
    EmptyField(&'static str),
}
impl fmt::Display for M10Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArtifact => f.write_str("M10 artifact is invalid"),
            Self::NotApk => f.write_str("M10 artifact is not an APK"),
            Self::MissingFile => f.write_str("M10 artifact file is missing"),
            Self::HashMismatch => f.write_str("M10 artifact checksum mismatch"),
            Self::SecretScanFailed => f.write_str("M10 artifact secret scan failed"),
            Self::EmptyField(v) => write!(f, "M10 field is empty: {v}"),
        }
    }
}
impl std::error::Error for M10Error {}
impl ApkArtifact {
    pub fn validate_metadata(&self) -> Result<(), M10Error> {
        if self.schema_version != M10_SCHEMA_VERSION {
            return Err(M10Error::InvalidArtifact);
        }
        for (n, v) in [
            ("artifactId", &self.artifact_id),
            ("projectId", &self.project_id),
            ("taskId", &self.task_id),
            ("projectRevisionId", &self.project_revision_id),
            ("sourceFingerprint", &self.source_fingerprint),
            ("path", &self.path),
            ("sha256", &self.sha256),
            ("packageName", &self.package_name),
            ("buildVariant", &self.build_variant),
        ] {
            if v.trim().is_empty() {
                return Err(M10Error::EmptyField(n));
            }
        }
        if !self.path.to_ascii_lowercase().ends_with(".apk") {
            return Err(M10Error::NotApk);
        }
        if self.secret_scan_status != "PASS" {
            return Err(M10Error::SecretScanFailed);
        }
        Ok(())
    }
    pub fn verify_file(&self) -> Result<(), M10Error> {
        self.validate_metadata()?;
        let bytes = fs::read(&self.path).map_err(|_| M10Error::MissingFile)?;
        let mut h = Sha256::new();
        h.update(bytes);
        let actual = format!("sha256:{:x}", h.finalize());
        if actual != self.sha256 {
            return Err(M10Error::HashMismatch);
        }
        Ok(())
    }
}
pub fn validate_apk_delivery(artifact: &ApkArtifact) -> Result<(), M10Error> {
    artifact.validate_metadata()?;
    if artifact.delivery_status != "READY_LOCAL" {
        return Err(M10Error::InvalidArtifact);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn apk_checksum_and_delivery_are_validated() {
        let p = std::env::temp_dir().join(format!(
            "m10-{}.apk",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&p, b"synthetic apk bytes").unwrap();
        let mut h = Sha256::new();
        h.update(b"synthetic apk bytes");
        let a = ApkArtifact {
            schema_version: M10_SCHEMA_VERSION,
            artifact_id: "a".into(),
            project_id: "p".into(),
            task_id: "t".into(),
            project_revision_id: "r".into(),
            source_fingerprint: "f".into(),
            path: p.to_string_lossy().into(),
            sha256: format!("sha256:{:x}", h.finalize()),
            package_name: "com.example.app".into(),
            build_variant: "debug".into(),
            secret_scan_status: "PASS".into(),
            signing_status: "UNSIGNED_DEBUG".into(),
            delivery_status: "READY_LOCAL".into(),
        };
        validate_apk_delivery(&a).unwrap();
        a.verify_file().unwrap();
        fs::remove_file(p).unwrap()
    }
    #[test]
    fn non_apk_or_bad_scan_is_rejected() {
        let a = ApkArtifact {
            schema_version: M10_SCHEMA_VERSION,
            artifact_id: "a".into(),
            project_id: "p".into(),
            task_id: "t".into(),
            project_revision_id: "r".into(),
            source_fingerprint: "f".into(),
            path: "x.zip".into(),
            sha256: "sha256:x".into(),
            package_name: "pkg".into(),
            build_variant: "debug".into(),
            secret_scan_status: "FAIL".into(),
            signing_status: "UNSIGNED_DEBUG".into(),
            delivery_status: "READY_LOCAL".into(),
        };
        assert_eq!(a.validate_metadata(), Err(M10Error::NotApk));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApkInspection {
    pub package_name: String,
    pub version_code: String,
    pub version_name: String,
    pub aapt_output_sha256: String,
}

pub fn inspect_apk(aapt_executable: &str, apk_path: &str) -> Result<ApkInspection, M10Error> {
    if aapt_executable.trim().is_empty() || apk_path.trim().is_empty() {
        return Err(M10Error::EmptyField("inspection input"));
    }
    let output = std::process::Command::new(aapt_executable)
        .args(["dump", "badging", apk_path])
        .output()
        .map_err(|_| M10Error::MissingFile)?;
    if !output.status.success() {
        return Err(M10Error::InvalidArtifact);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let package_line = text
        .lines()
        .find(|line| line.starts_with("package:"))
        .ok_or(M10Error::InvalidArtifact)?;
    let package_name = quoted_field(package_line, "name=")?;
    let version_code = quoted_field(package_line, "versionCode=")?;
    let version_name = quoted_field(package_line, "versionName=")?;
    use sha2::Digest;
    let output_hash = format!("{:x}", Sha256::digest(output.stdout));
    Ok(ApkInspection {
        package_name,
        version_code,
        version_name,
        aapt_output_sha256: output_hash,
    })
}

pub fn deliver_apk_local(
    artifact: &ApkArtifact,
    destination: &str,
) -> Result<ApkArtifact, M10Error> {
    artifact.verify_file()?;
    if destination.trim().is_empty() {
        return Err(M10Error::EmptyField("destination"));
    }
    let destination_path = std::path::Path::new(destination);
    if !destination_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("apk"))
    {
        return Err(M10Error::NotApk);
    }
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent).map_err(|_| M10Error::MissingFile)?;
    }
    fs::copy(&artifact.path, destination_path).map_err(|_| M10Error::MissingFile)?;
    let mut delivered = artifact.clone();
    delivered.path = destination.into();
    delivered.delivery_status = "READY_LOCAL".into();
    delivered.verify_file()?;
    validate_apk_delivery(&delivered)?;
    Ok(delivered)
}

fn quoted_field(line: &str, prefix: &str) -> Result<String, M10Error> {
    let start = line.find(prefix).ok_or(M10Error::InvalidArtifact)? + prefix.len();
    let value = &line[start..];
    let quote = value.chars().next().ok_or(M10Error::InvalidArtifact)?;
    if quote != '\'' && quote != '"' {
        return Err(M10Error::InvalidArtifact);
    }
    let value = &value[quote.len_utf8()..];
    let end = value.find(quote).ok_or(M10Error::InvalidArtifact)?;
    let field = value[..end].to_owned();
    if field.is_empty() {
        Err(M10Error::InvalidArtifact)
    } else {
        Ok(field)
    }
}

#[cfg(test)]
mod execution_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    #[test]
    fn inspection_and_local_delivery_are_observation_based() {
        use std::os::unix::fs::PermissionsExt;
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nirman-m10-execution-{suffix}"));
        fs::create_dir_all(&root).expect("root");
        let apk = root.join("source.apk");
        let bytes = b"apk-observation-bytes";
        fs::write(&apk, bytes).expect("apk");
        let aapt = root.join("aapt");
        fs::write(
            &aapt,
            "#!/bin/sh\nprintf \"package: name='com.nirman.fixture' versionCode='7' versionName='1.2.3'\\n\"\n",
        )
        .expect("aapt");
        let mut permissions = fs::metadata(&aapt).expect("aapt metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&aapt, permissions).expect("aapt permissions");
        let inspection = inspect_apk(
            aapt.to_string_lossy().as_ref(),
            apk.to_string_lossy().as_ref(),
        )
        .expect("inspection");
        assert_eq!(inspection.package_name, "com.nirman.fixture");
        assert_eq!(inspection.version_code, "7");
        assert_eq!(inspection.version_name, "1.2.3");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let artifact = ApkArtifact {
            schema_version: M10_SCHEMA_VERSION,
            artifact_id: "artifact-execution".into(),
            project_id: "project".into(),
            task_id: "task".into(),
            project_revision_id: "revision-1".into(),
            source_fingerprint: "fingerprint".into(),
            path: apk.to_string_lossy().into_owned(),
            sha256: format!("sha256:{:x}", hasher.finalize()),
            package_name: inspection.package_name,
            build_variant: "debug".into(),
            secret_scan_status: "PASS".into(),
            signing_status: "UNSIGNED_DEBUG".into(),
            delivery_status: "PENDING_LOCAL".into(),
        };
        let destination = root.join("delivery").join("fixture.apk");
        let delivered =
            deliver_apk_local(&artifact, destination.to_string_lossy().as_ref()).expect("delivery");
        assert_eq!(delivered.delivery_status, "READY_LOCAL");
        assert_eq!(fs::read(destination).expect("delivered bytes"), bytes);
        let _ = fs::remove_dir_all(root);
    }
}

pub fn scan_apk_for_secrets(apk_path: &str) -> Result<(), M10Error> {
    let bytes = fs::read(apk_path).map_err(|_| M10Error::MissingFile)?;
    let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
    let suspicious_markers = [
        "begin private key",
        "begin rsa private key",
        "akia",
        "-----begin",
        "api_key=",
        "apikey=",
        "secret_key=",
        "sk-live-",
    ];
    if suspicious_markers
        .iter()
        .any(|marker| text.contains(marker))
    {
        return Err(M10Error::SecretScanFailed);
    }
    Ok(())
}
