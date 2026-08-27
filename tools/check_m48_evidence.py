"""Validate observation-derived M48 preview evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_FILES = (
    ROOT / "tests/evidence/m48_preview_authority.json",
    ROOT / "tests/evidence/m48_host_integration.json",
)


def load(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"M48 evidence is missing: {path}")
    evidence = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(evidence, dict) or not evidence.get("schema", "").startswith("nirman.m48."):
        raise SystemExit(f"M48 evidence schema is invalid: {path}")
    return evidence


def require_true(evidence: dict, keys: set[str], path: Path) -> None:
    missing = sorted(key for key in keys if evidence.get(key) is not True)
    if missing:
        raise SystemExit(f"M48 evidence lacks observed positives in {path.name}: {', '.join(missing)}")


def require_false(evidence: dict, keys: set[str], path: Path) -> None:
    invalid = sorted(key for key in keys if evidence.get(key) is not False)
    if invalid:
        raise SystemExit(f"M48 evidence must keep runtime claims false in {path.name}: {', '.join(invalid)}")


def main() -> int:
    authority, host = (load(path) for path in EVIDENCE_FILES)
    require_true(
        authority,
        {
            "composeReloadObserved",
            "reactNativeExpoRefreshObserved",
            "incrementalEmulatorInstallObserved",
            "headlessSmokeFallbackObserved",
            "diagnosticFallbackObserved",
            "revisionIdentityBindingObserved",
            "staleIdentityRejectedObserved",
            "predictedPromotionRejectedObserved",
            "observedPromotionObserved",
            "lastKnownGoodPreservedObserved",
        },
        EVIDENCE_FILES[0],
    )
    require_true(
        host,
        {
            "authenticatedPreviewCommandObserved",
            "fallbackSelectionObserved",
            "revisionBindingObserved",
            "durablePreviewRevisionObserved",
            "durablePreviewProjectionObserved",
            "duplicateDurableReloadObserved",
            "staleSourceRejectedObserved",
        },
        EVIDENCE_FILES[1],
    )
    for evidence, path in zip((authority, host), EVIDENCE_FILES):
        require_false(
            evidence,
            {"buildObserved", "installObserved", "launchObserved", "androidDeviceObserved", "nativeWindowsTauriRuntimeObserved"},
            path,
        )
    if authority.get("m48Status") != "M48_HEADLESS_PREVIEW_AUTHORITY_TRACE_ONLY":
        raise SystemExit("M48 authority evidence must remain explicitly headless")
    if host.get("evidenceStatus") != "M48_HEADLESS_AUTHENTICATED_PREVIEW_BOUNDARY_TRACE_ONLY":
        raise SystemExit("M48 host evidence must remain explicitly headless")
    print("M48 preview evidence: PASS (headless authority and authenticated-host traces)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
