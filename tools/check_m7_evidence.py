#!/usr/bin/env python3
"""Validate the observation-derived portable M7 task-continuation trace."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m7_task_continuation_trace.json"
REQUIRED_TRUE = {
    "authenticatedTaskStartObserved",
    "threeWorkerStepsDurableBeforeRestart",
    "m6EditPolicyAllowObserved",
    "m6PolicyDecisionDurable",
    "taskRecordReloadedById",
    "recoverableAfterRestart",
    "checkpointResolvableAfterRestart",
    "sourceRevisionRestoredAfterRestart",
    "orderedWorkerReplayObserved",
    "resumedInspectAppendedToSameStream",
}
REQUIRED_FALSE = {
    "duplicateWorkerEventsObserved": False,
    "gradleExecuted": False,
    "androidRuntimeObserved": False,
    "nativeTauriRuntimeObserved": False,
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M7 continuation evidence is missing: {EVIDENCE}")
    try:
        record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M7 continuation evidence is not valid JSON: {exc}") from exc
    if not isinstance(record, dict):
        raise SystemExit("M7 continuation evidence must be a JSON object")
    if record.get("schema") != "nirman.m7.task_continuation_trace.v1":
        raise SystemExit("unexpected M7 continuation evidence schema")
    if record.get("fixtureId") != "M7-TASK-CONTINUATION-001":
        raise SystemExit("unexpected M7 continuation fixture identity")
    if record.get("taskId") != "m7-continuation-task":
        raise SystemExit("unexpected M7 continuation task identity")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit("M7 continuation evidence lacks observed positives: " + ", ".join(missing))
    for key, expected in REQUIRED_FALSE.items():
        if record.get(key) is not expected:
            raise SystemExit(f"M7 continuation evidence must keep {key}={expected}")
    if record.get("workerStepOrder") != ["Inspect", "Edit", "Checkpoint"]:
        raise SystemExit("M7 continuation WorkerStep order is not Inspect/Edit/Checkpoint")
    if record.get("postCheckpointSourceRevision") != 2:
        raise SystemExit("M7 continuation post-checkpoint revision is not the observed revision 2")
    if record.get("resumedEventSequence") != 7:
        raise SystemExit("M7 continuation resumed event sequence is not the same-stream sequence 7")
    if record.get("filesystemAdapter") != "test-fixture-only":
        raise SystemExit("M7 continuation evidence must identify the fixture-only filesystem adapter")
    if record.get("evidenceStatus") != "M7_HEADLESS_TASK_CONTINUATION_TRACE_ONLY":
        raise SystemExit("M7 continuation evidence must remain explicitly headless")
    print("M7 task continuation evidence: PASS (durable restart/replay/resume trace only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
