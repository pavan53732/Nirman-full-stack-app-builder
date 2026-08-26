"""Validate observation-derived M39 Android construction evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m39_android_construction.json"
REQUIRED_TRUE = {
    "validAndroidIntentObserved",
    "targetPlatformsExactAndroidObserved",
    "requiredConstructionFieldsObserved",
    "invalidContractRejected",
    "deterministicValidationObserved",
    "unknownFieldsRejected",
    "m115AuthenticatedCommandBoundaryObserved",
    "durableContractPersistenceObserved",
    "durableContractReloadObserved",
    "projectionEventObserved",
    "unsupportedTargetRejected",
}
FORBIDDEN_KEYS = {"apiKey", "api_key", "password", "token", "secret", "rawCredential"}


def walk_strings(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, dict):
        return [item for child in value.values() for item in walk_strings(child)]
    if isinstance(value, list):
        return [item for child in value for item in walk_strings(child)]
    return []


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M39 evidence is missing: {EVIDENCE}")
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if evidence.get("schema") != "nirman.m39.android_construction.v1":
        raise SystemExit("M39 evidence schema is incorrect")
    missing = sorted(key for key in REQUIRED_TRUE if evidence.get(key) is not True)
    if missing:
        raise SystemExit("M39 evidence lacks observations: " + ", ".join(missing))
    if evidence.get("androidWorkspaceMutation") is not False:
        raise SystemExit("M39 evidence must not claim workspace mutation")
    if evidence.get("evidenceStatus") != "M39_HEADLESS_DURABLE_CONTRACT_TRACE_ONLY":
        raise SystemExit("M39 evidence must remain scoped to the contract trace")
    if any(key in evidence for key in FORBIDDEN_KEYS):
        raise SystemExit("M39 evidence contains a forbidden credential-bearing key")
    if any("raw-api-key" in value or "sk-" in value for value in walk_strings(evidence)):
        raise SystemExit("M39 evidence contains raw credential material")
    print("M39 Android construction evidence: PASS (headless durable contract trace)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
