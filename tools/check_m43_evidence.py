"""Validate observation-derived M43 Android toolchain evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m43_android_toolchain.json"
REQUIRED_TRUE = {
    "m39ContractReloadObserved",
    "manifestDerivedFromM39Observed",
    "requiredToolchainCapabilitiesObserved",
    "deterministicPreflightObserved",
    "availableClassificationObserved",
    "toolchainLockGenerated",
    "versionHashLicenseRecordsObserved",
    "environmentSnapshotObserved",
    "durablePreflightPersistenceObserved",
    "durablePreflightReloadObserved",
    "durableCheckpointObserved",
    "m115AuthenticatedCommandBoundaryObserved",
    "typedResponseObserved",
    "projectionEventObserved",
    "missingContractRejected",
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M43 evidence is missing: {EVIDENCE}")
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if evidence.get("schema") != "nirman.m43.android_toolchain.v1":
        raise SystemExit("M43 evidence schema is incorrect")
    missing = sorted(key for key in REQUIRED_TRUE if evidence.get(key) is not True)
    if missing:
        raise SystemExit("M43 evidence lacks observations: " + ", ".join(missing))
    if evidence.get("hostCapabilityProbeRuntime") is not False:
        raise SystemExit("M43 evidence must not claim native host probe execution")
    if evidence.get("toolchainRepairExecuted") is not False:
        raise SystemExit("M43 evidence must not claim toolchain repair execution")
    if evidence.get("androidBuildObserved") is not False:
        raise SystemExit("M43 evidence must not claim Android build execution")
    if evidence.get("evidenceStatus") != "M43_HEADLESS_LOCKED_PREFLIGHT_TRACE_ONLY":
        raise SystemExit("M43 evidence scope is incorrect")
    print("M43 Android toolchain evidence: PASS (headless locked preflight trace)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
