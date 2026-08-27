#!/usr/bin/env python3
"""Validate the observation-derived portable M5 edit/checkpoint/undo trace."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m5_worker_edit_undo_trace.json"
REQUIRED_TRUE = {
    "authenticatedAdmissionObserved",
    "planEventConsumed",
    "editWorkerStepObserved",
    "checkpointWorkerStepObserved",
    "rollbackWorkerStepObserved",
    "m6PolicyAllowObserved",
    "m6PolicyDecisionDurable",
    "workspaceMutationObserved",
    "singleFileCreated",
    "sourceRevisionAdvanced",
    "checkpointAfterEditPersisted",
    "undoRemovedFile",
    "preEditRevisionRestored",
    "rollbackEventDurable",
    "mutationTransactionReloaded",
    "restartReplayObserved",
}
REQUIRED_FALSE = {
    "gradleExecuted": False,
    "androidRuntimeObserved": False,
    "nativeTauriRuntimeObserved": False,
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(
            f"M5 edit/undo evidence is missing; run the portable fixture first: {EVIDENCE}"
        )
    try:
        record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M5 edit/undo evidence is not valid JSON: {exc}") from exc
    if not isinstance(record, dict):
        raise SystemExit("M5 edit/undo evidence must be a JSON object")
    if record.get("schema") != "nirman.m5.worker_edit_undo_trace.v1":
        raise SystemExit("unexpected M5 edit/undo evidence schema")
    if record.get("fixtureId") != "M5-WORKER-EDIT-UNDO-001":
        raise SystemExit("unexpected M5 edit/undo evidence fixture identity")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit("M5 edit/undo evidence lacks observed positives: " + ", ".join(missing))
    for key, expected in REQUIRED_FALSE.items():
        if record.get(key) is not expected:
            raise SystemExit(f"M5 edit/undo evidence must keep {key}=false")
    if record.get("filesystemAdapter") != "test-fixture-only":
        raise SystemExit("M5 edit/undo evidence must identify the fixture-only filesystem adapter")
    if record.get("evidenceStatus") != "M5_HEADLESS_EDIT_UNDO_TRACE_ONLY":
        raise SystemExit("M5 edit/undo evidence must remain explicitly headless")
    print("M5 edit/undo evidence: PASS (portable policy, mutation, checkpoint, and rollback trace only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
