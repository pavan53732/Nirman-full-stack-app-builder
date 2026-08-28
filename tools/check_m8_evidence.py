#!/usr/bin/env python3
"""Validate the observation-derived portable M8 multi-worker coordination trace."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m8_multi_worker_trace.json"
REQUIRED_TRUE = {
    "sharedLedgerSeparateSqlite",
    "atomicClaimsObserved",
    "doubleClaimRejected",
    "typedConflictDetected",
    "originalT2HandoffPreserved",
    "conflictAcknowledgementDurable",
    "restartReloadObserved",
}
REQUIRED_FALSE = {
    "nativeWindowsRuntimeObserved": False,
    "androidRuntimeObserved": False,
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M8 evidence is missing: {EVIDENCE}")
    try:
        record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M8 evidence is not valid JSON: {exc}") from exc
    if not isinstance(record, dict):
        raise SystemExit("M8 evidence must be a JSON object")
    if record.get("schema") != "nirman.m8.multi_worker_trace.v1":
        raise SystemExit("unexpected M8 evidence schema")
    if record.get("fixtureId") != "M8-MULTI-WORKER-001":
        raise SystemExit("unexpected M8 fixture identity")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit("M8 evidence lacks observed positives: " + ", ".join(missing))
    for key, expected in REQUIRED_FALSE.items():
        if record.get(key) is not expected:
            raise SystemExit(f"M8 evidence must keep {key}={expected}")
    if record.get("workerFixtureCount") != 3 or record.get("isolatedWorkspaceCount") != 3:
        raise SystemExit("M8 evidence must contain three isolated worker fixtures")
    if record.get("tasksSeeded") != ["T1", "T2", "T3"]:
        raise SystemExit("M8 evidence task seed is not exactly T1/T2/T3")
    if record.get("missedTaskCount") != 0:
        raise SystemExit("M8 evidence reports a missed task")
    if record.get("workerStepEditCount") != 3 or record.get("m6AllowDecisionCount") != 3:
        raise SystemExit("M8 evidence does not prove one edit and M6 allow per worker")
    if record.get("structuredHandoffsPersisted") != 3:
        raise SystemExit("M8 evidence does not prove three persisted handoffs")
    if record.get("handoffChangedPaths") != ["src/A.kt", "src/B.kt", "src/C.kt"]:
        raise SystemExit("M8 handoff path set is not the expected isolated set")
    if record.get("forcedOverlapPath") != "src/B.kt":
        raise SystemExit("M8 forced conflict path is not src/B.kt")
    if record.get("reconciliationBlockedOnConflict") is not False:
        raise SystemExit("M8 evidence must distinguish rejected conflict from original-hand-off reconciliation")
    if record.get("reconciliationCheckpointAfterRejectedConflict") != "m8-integration-checkpoint-3":
        raise SystemExit("M8 evidence is missing the original-handoff reconciliation checkpoint")
    if record.get("filesystemAdapter") != "test-fixture-only":
        raise SystemExit("M8 evidence must identify the fixture-only filesystem adapter")
    if record.get("evidenceStatus") != "M8_HEADLESS_MULTI_WORKER_TRACE_ONLY":
        raise SystemExit("M8 evidence must remain explicitly headless")
    print("M8 multi-worker evidence: PASS (claims, isolated handoffs, conflict, and restart trace only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
