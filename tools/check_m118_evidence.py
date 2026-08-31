"""Validate the M118 platform capability and cross-compilation fixture evidence.

TEST-PLAT-001 / EV-PLAT-001 (BS §79.13, TA §84.5, ADR-206,
CONTRACT.RUNTIME.PLATFORM_CAPABILITY).

The trace is produced deterministically by
`cargo test -p nirman-tools --test m118_platform_capability`. The checker
enforces the four hallucination-prevention fixture outcomes and the honest
"nothing was actually executed" flags: no Windows runtime, Android device,
or real cross-build observation may be claimed by this evidence.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_FILE = ROOT / "tests/evidence/m118_platform_capability.json"

# These must stay false: the M118 fixtures exercise deterministic gate logic
# over synthetic observed records, not real platform execution.
COMMON_FALSE = {
    "windowsRuntimeObserved": False,
    "androidDeviceObserved": False,
    "crossBuildExecutedOnHost": False,
}


def load(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"M118 evidence is missing: {path} (run cargo test -p nirman-tools --test m118_platform_capability)")
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M118 evidence is not valid JSON: {path}: {exc}") from exc
    if not isinstance(evidence, dict):
        raise SystemExit(f"M118 evidence must be an object: {path}")
    if not str(evidence.get("schema", "")).startswith("nirman.m118."):
        raise SystemExit(f"M118 evidence schema is incorrect: {path}: {evidence.get('schema')!r}")
    for key, expected in COMMON_FALSE.items():
        if evidence.get(key) is not expected:
            raise SystemExit(f"M118 evidence must keep {key}=false: {path}")
    return evidence


def main() -> int:
    evidence = load(EVIDENCE_FILE)

    if evidence.get("testFamily") != "TEST-PLAT-001":
        raise SystemExit(f"M118 evidence test family must be TEST-PLAT-001: {evidence.get('testFamily')!r}")
    if evidence.get("evidenceId") != "EV-PLAT-001":
        raise SystemExit(f"M118 evidence id must be EV-PLAT-001: {evidence.get('evidenceId')!r}")
    if evidence.get("contract") != "CONTRACT.RUNTIME.PLATFORM_CAPABILITY":
        raise SystemExit(f"M118 evidence contract is incorrect: {evidence.get('contract')!r}")

    a, b, c, d = (evidence.get(k) for k in ("fixtureA", "fixtureB", "fixtureC", "fixtureD"))
    for name, fixture in (("fixtureA", a), ("fixtureB", b), ("fixtureC", c), ("fixtureD", d)):
        if not isinstance(fixture, dict):
            raise SystemExit(f"M118 {name} fixture is missing or not an object")

    # Fixture A: cross-build may execute; native validation must not be claimed.
    if a.get("crossBuildAdmitted") is not True:
        raise SystemExit("M118 fixtureA: cross-build must be admitted with a proven toolchain")
    if a.get("nativeValidationClaimed") is not False:
        raise SystemExit("M118 fixtureA: native Windows validation must NOT be claimed from a Linux host")
    if a.get("runtimeClaimRejectedBeforeExecution") is not True:
        raise SystemExit("M118 fixtureA: the runtime-validation claim must be rejected before execution")
    if a.get("blockedState") not in ("USER_REQUIRED", "UNAVAILABLE"):
        raise SystemExit(f"M118 fixtureA: blocked node must record USER_REQUIRED/UNAVAILABLE, got {a.get('blockedState')!r}")
    can_continue = " ".join(a.get("canContinue", []))
    cannot_continue = " ".join(a.get("cannotContinue", []))
    if "cross-build" not in can_continue:
        raise SystemExit("M118 fixtureA: can-continue list must include cross-build work")
    if "Windows runtime certification" not in cannot_continue:
        raise SystemExit("M118 fixtureA: cannot-continue list must include Windows runtime certification")

    # Fixture B: artifact verified, runtime unverified, never SUPPORTED.
    if b.get("artifactBuild") != "Verified":
        raise SystemExit(f"M118 fixtureB: artifact build must be Verified, got {b.get('artifactBuild')!r}")
    if b.get("windowsRuntime") != "Unverified":
        raise SystemExit(f"M118 fixtureB: windows runtime must be Unverified, got {b.get('windowsRuntime')!r}")
    if b.get("aggregate") != "SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS":
        raise SystemExit(f"M118 fixtureB: aggregate must be SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS, got {b.get('aggregate')!r}")

    # Fixture C: fake completion is rejected and cites the missing evidence.
    if c.get("completionClaimAccepted") is not False:
        raise SystemExit("M118 fixtureC: a fake completion claim must be rejected")
    if c.get("rejectionCitesMissingEvidence") is not True:
        raise SystemExit("M118 fixtureC: the rejection must cite the missing evidence")
    if "evidence" not in str(c.get("rejection", "")).lower():
        raise SystemExit(f"M118 fixtureC: rejection must name the evidence gap: {c.get('rejection')!r}")

    # Fixture D: stale target evidence is invalidated; the gate re-closes.
    if d.get("priorEvidenceInvalidated") is not True:
        raise SystemExit("M118 fixtureD: prior target evidence must be INVALIDATED after an identity change")
    if d.get("certificationGateReClosed") is not True:
        raise SystemExit("M118 fixtureD: the certification gate must re-close until re-validation")

    # TA §84.5 additional proofs.
    additional = evidence.get("additional")
    if not isinstance(additional, dict):
        raise SystemExit("M118 additional fixture results are missing")
    for key in (
        "targetMismatchGuardRejectedBeforeExecution",
        "schedulingHonorsPlatformFields",
        "leaseLossFencesValidation",
        "matrixVersionChangeRerunsPreflight",
        "traceabilityChainPopulated",
    ):
        if additional.get(key) is not True:
            raise SystemExit(f"M118 additional proof {key} must be true")

    print("M118 platform capability evidence: PASS (TEST-PLAT-001, EV-PLAT-001)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
