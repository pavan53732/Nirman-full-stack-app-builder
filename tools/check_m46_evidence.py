"""Validate observation-derived M46 structured-mutation evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m46_structured_mutation.json"
REQUIRED_TRUE = {
    "validStructuredMutationObserved",
    "scopeValidationObserved",
    "pathNormalizationObserved",
    "baseRevisionValidationObserved",
    "fileOwnershipValidationObserved",
    "syntaxValidationObserved",
    "graphReindexObserved",
    "contentIntegrityObserved",
    "dependencyPolicyObserved",
    "mutationBudgetObserved",
    "wholeFileFallbackRestrictionObserved",
    "adversarialRejectionsObserved",
    "workspaceMutationStayedInsideDeclaredPath",
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M46 evidence is missing: {EVIDENCE}")
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if evidence.get("schema") != "nirman.m46.structured_mutation.v1":
        raise SystemExit("M46 evidence schema is incorrect")
    missing = sorted(key for key in REQUIRED_TRUE if evidence.get(key) is not True)
    if missing:
        raise SystemExit("M46 evidence has no observed positive result for: " + ", ".join(missing))
    if evidence.get("androidBuildObserved") is not False:
        raise SystemExit("M46 evidence must not claim Android build execution")
    if evidence.get("nativeWindowsTauriRuntimeObserved") is not False:
        raise SystemExit("M46 evidence must not claim native Windows/Tauri runtime proof")
    allowed_statuses = {
        "M46_HEADLESS_STRUCTURED_MUTATION_TRACE_ONLY",
        "M46_HEADLESS_DURABLE_STRUCTURED_MUTATION_TRACE_ONLY",
    }
    if evidence.get("m46Status") not in allowed_statuses:
        raise SystemExit("M46 evidence must remain explicitly headless")
    if evidence.get("m46Status") == "M46_HEADLESS_DURABLE_STRUCTURED_MUTATION_TRACE_ONLY":
        durable_keys = {
            "atomicM115MutationAdmissionObserved",
            "preparedTransactionCheckpointObserved",
            "leaseAndCapabilityAuthorityObserved",
            "durableCommittedTransactionObserved",
            "duplicateResultReloadObserved",
            "restartResultReloadObserved",
        }
        missing_durable = sorted(key for key in durable_keys if evidence.get(key) is not True)
        if missing_durable:
            raise SystemExit("M46 durable evidence is missing: " + ", ".join(missing_durable))
    print("M46 structured mutation evidence: PASS (headless parser-aware mutation trace)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
