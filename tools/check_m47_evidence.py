"""Validate observation-derived M47 requirement and repair evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_FILES = (
    ROOT / "tests/evidence/m47_requirements_repair.json",
    ROOT / "tests/evidence/m47_host_integration.json",
)
COMMON_FALSE = {
    "androidBuildObserved": False,
    "androidDeviceObserved": False,
    "nativeWindowsTauriRuntimeObserved": False,
}


def load(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"M47 evidence is missing: {path}")
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M47 evidence is not valid JSON: {path}: {exc}") from exc
    if not isinstance(evidence, dict):
        raise SystemExit(f"M47 evidence must be an object: {path}")
    if evidence.get("schema", "").startswith("nirman.m47.") is False:
        raise SystemExit(f"M47 evidence schema is incorrect: {path}")
    for key, expected in COMMON_FALSE.items():
        if evidence.get(key) is not expected:
            raise SystemExit(f"M47 evidence must keep {key}=false: {path}")
    return evidence


def main() -> int:
    authority, host = (load(path) for path in EVIDENCE_FILES)
    required_authority = {
        "fileBackedIndexObserved",
        "androidOnlyContractObserved",
        "manifestPresentSatisfiedObserved",
        "permissionSatisfiedObserved",
        "excessivePermissionObserved",
        "manifestResourceValidationObserved",
        "missingManifestObserved",
        "deterministicSerializationReloadObserved",
        "repairFamiliesObserved",
        "allowedRepairSelectionObserved",
        "retryBudgetObserved",
        "checkpointRuleObserved",
        "unknownFailureRejectedObserved",
    }
    missing = sorted(key for key in required_authority if authority.get(key) is not True)
    if missing:
        raise SystemExit("M47 authority evidence lacks observed positives: " + ", ".join(missing))
    required_host = {
        "m39ContractAcceptedObserved",
        "m45FingerprintObserved",
        "m47ManifestResponseObserved",
        "durableManifestObserved",
        "durableRepairSelectionObserved",
        "duplicateDurableReloadObserved",
        "unauthorizedWorkspaceRejectedObserved",
    }
    missing = sorted(key for key in required_host if host.get(key) is not True)
    if missing:
        raise SystemExit("M47 host evidence lacks observed positives: " + ", ".join(missing))
    if authority.get("androidWorkspaceMutation") is not False:
        raise SystemExit("M47 authority fixture must remain read-only")
    if host.get("m46MutationBrokerInvoked") is not False:
        raise SystemExit("M47 host fixture must not claim M46 mutation")
    if authority.get("m47Status") != "M47_HEADLESS_REQUIREMENT_REPAIR_TRACE_ONLY":
        raise SystemExit("M47 authority evidence must remain explicitly headless")
    if host.get("evidenceStatus") != "M47_HEADLESS_AUTHENTICATED_HOST_TRACE_ONLY":
        raise SystemExit("M47 host evidence must remain explicitly headless")
    print("M47 requirement and repair evidence: PASS (headless authority and authenticated-host traces)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
