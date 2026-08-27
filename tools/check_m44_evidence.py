"""Validate observation-derived M44 Provider Bridge evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m44_provider_bridge.json"
REQUIRED_TRUE = {
    "authenticatedM115AdmissionObserved",
    "providerBridgeAuthorityObserved",
    "m39ContractPrerequisiteObserved",
    "m43AvailableLockPrerequisiteObserved",
    "lockHashAndSnapshotIdentityBoundObserved",
    "m3RuntimeDelegationObserved",
    "normalizedResponseObserved",
    "normalizedStreamingEventsObserved",
    "durableUsageRecordObserved",
    "durableExecutionRecordObserved",
    "executionRecordReloadObserved",
    "duplicateIdempotencyReloadObserved",
    "scopeAndMissingLockRejected",
    "protocolAuthenticationCapabilityFailuresObserved",
    "outageMalformedRateLimitTimeoutCancellationFailuresObserved",
    "secretRedactionObserved",
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
        raise SystemExit(f"M44 evidence is missing: {EVIDENCE}")
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if evidence.get("schema") != "nirman.m44.provider_bridge.v1":
        raise SystemExit("M44 evidence schema is incorrect")
    missing = sorted(key for key in REQUIRED_TRUE if evidence.get(key) is not True)
    if missing:
        raise SystemExit("M44 evidence has no observed positive result for: " + ", ".join(missing))
    if evidence.get("androidWorkspaceMutation") is not False:
        raise SystemExit("M44 evidence must prove no Android workspace mutation")
    if evidence.get("nativeWindowsTauriAndroidRuntimeObserved") is not False:
        raise SystemExit("M44 evidence must not claim native Windows/Tauri/Android runtime proof")
    if evidence.get("evidenceStatus") != "M44_HEADLESS_DURABLE_BRIDGE_TRACE_ONLY":
        raise SystemExit("M44 evidence must remain explicitly headless")
    if any(key in evidence for key in FORBIDDEN_KEYS):
        raise SystemExit("M44 evidence contains a forbidden credential-bearing key")
    forbidden_fragments = ("raw-secret", "raw-api-key", "sk-", "host-path-secret")
    if any(any(fragment in value for fragment in forbidden_fragments) for value in walk_strings(evidence)):
        raise SystemExit("M44 evidence contains raw credential material")
    print("M44 provider bridge evidence: PASS (headless durable bridge trace)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
