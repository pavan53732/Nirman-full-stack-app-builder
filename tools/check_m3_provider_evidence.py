"""Validate observation-derived M3 provider-runtime evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m3_provider_runtime.json"
INTEGRATION_EVIDENCE = ROOT / "tests/evidence/m3_provider_integration.json"
REQUIRED_TRUE = {
    "profileValidated",
    "credentialReferenceOnly",
    "authenticatedRequestConstructed",
    "requestCorrelationIdentity",
    "normalizedResponseObserved",
    "timeoutObserved",
    "cancellationObserved",
    "normalizedFailureObserved",
    "durableUsageRecorded",
    "usageRestoredAfterRestart",
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
        raise SystemExit(f"M3 evidence is missing: {EVIDENCE}")
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if evidence.get("schema") != "nirman.m3.provider_runtime.v1":
        raise SystemExit("M3 evidence schema is incorrect")
    missing = sorted(key for key in REQUIRED_TRUE if evidence.get(key) is not True)
    if missing:
        raise SystemExit(f"M3 evidence has no observed positive result for: {', '.join(missing)}")
    if evidence.get("providerTransport") != "fixture_transport":
        raise SystemExit("M3 evidence must identify its deterministic fixture transport")
    if evidence.get("runtimeStatus") != "M3_FOUNDATION_FIXTURE_ONLY":
        raise SystemExit("M3 evidence must not claim provider production runtime certification")
    if any(key in evidence for key in FORBIDDEN_KEYS):
        raise SystemExit("M3 evidence contains a forbidden credential-bearing key")
    if any("raw-api-key" in value or "sk-" in value for value in walk_strings(evidence)):
        raise SystemExit("M3 evidence contains raw credential material")
    if not INTEGRATION_EVIDENCE.is_file():
        raise SystemExit(f"M3 integration evidence is missing: {INTEGRATION_EVIDENCE}")
    integration = json.loads(INTEGRATION_EVIDENCE.read_text(encoding="utf-8"))
    required_integration_true = {
        "authenticatedCommandEnvelopeObserved",
        "settingsUpdateProviderBoundaryObserved",
        "providerTestBoundaryObserved",
        "durableProfileTransactionObserved",
        "durableProviderEventEmissionObserved",
        "providerRuntimeExecutedAfterDurableAdmission",
        "typedProviderResultObserved",
        "providerUsageDurabilityObserved",
        "secretRedactionObserved",
    }
    if integration.get("schema") != "nirman.m3.provider_integration.v1":
        raise SystemExit("M3 integration evidence schema is incorrect")
    missing_integration = sorted(
        key for key in required_integration_true if integration.get(key) is not True
    )
    if missing_integration:
        raise SystemExit(
            "M3 integration evidence has no observed positive result for: "
            + ", ".join(missing_integration)
        )
    if integration.get("windowsCredentialManagerRuntimeObserved") is not False:
        raise SystemExit("M3 integration evidence must identify the Windows runtime boundary")
    if integration.get("evidenceStatus") != "M3_M115_HEADLESS_INTEGRATION_TRACE_ONLY":
        raise SystemExit("M3 integration evidence must remain honest about native runtime scope")
    if any(key in integration for key in FORBIDDEN_KEYS):
        raise SystemExit("M3 integration evidence contains a forbidden credential-bearing key")
    if any("raw-api-key" in value or "sk-" in value or "host-path-secret" in value for value in walk_strings(integration)):
        raise SystemExit("M3 integration evidence contains raw credential material")
    print("M3 provider evidence: PASS (foundation fixture plus headless M115 integration trace)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
