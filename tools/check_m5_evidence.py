#!/usr/bin/env python3
"""Validate the observation-derived portable M5 inspection/checkpoint trace."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m5_worker_trace.json"
REQUIRED_TRUE = {
    "authenticatedAdmissionObserved",
    "planEventConsumed",
    "inspectionObserved",
    "workerStepDurable",
    "checkpointPersisted",
    "workerRecordReloaded",
    "restartReplayObserved",
}
REQUIRED_FALSE = {
    "mutationObserved": False,
    "gradleExecuted": False,
    "androidRuntimeObserved": False,
    "nativeTauriRuntimeObserved": False,
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M5 evidence is missing; run the portable integration test first: {EVIDENCE}")
    try:
        record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M5 evidence is not valid JSON: {exc}") from exc
    if not isinstance(record, dict):
        raise SystemExit("M5 evidence must be a JSON object")
    if record.get("schema") != "nirman.m5.worker_trace.v1":
        raise SystemExit("unexpected M5 evidence schema")
    if record.get("fixtureId") != "M5-WORKER-TRACE-001":
        raise SystemExit("unexpected M5 evidence fixture identity")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit("M5 evidence lacks observed positives: " + ", ".join(missing))
    for key, expected in REQUIRED_FALSE.items():
        if record.get(key) is not expected:
            raise SystemExit(f"M5 evidence must keep {key}={str(expected).lower()}")
    if record.get("checkpointId") != "checkpoint-m5-inspect":
        raise SystemExit("M5 evidence has an unexpected checkpoint identity")
    if record.get("evidenceStatus") != "M5_HEADLESS_INSPECTION_CHECKPOINT_TRACE_ONLY":
        raise SystemExit("M5 evidence must remain explicitly headless")
    print("M5 portable worker evidence: PASS (inspection/checkpoint trace only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
