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
