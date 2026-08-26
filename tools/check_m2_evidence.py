#!/usr/bin/env python3
"""Validate the machine-readable M2 foundation trace emitted by the integration test."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m2_vertical_trace.json"

REQUIRED_TRUE = {
    "pausePersisted",
    "resumePersisted",
    "cancellationReachedControlPlane",
    "workerCancellationRequested",
    "staleLeaseFenced",
    "duplicateAndOutOfOrderEventsRejected",
    "typedProjectionBoundaryObserved",
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit("M2 evidence is missing; run the vertical integration test first")
    record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if record.get("fixtureId") != "M2-VERTICAL-TRACE-001":
        raise SystemExit("unexpected M2 evidence fixture identity")
    if record.get("eventSequences") != [1, 2, 3, 4]:
        raise SystemExit("M2 evidence event sequence is not deterministic")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit(f"M2 evidence is missing required positive assertions: {', '.join(missing)}")
    if record.get("recoveredFinalState") != "CANCELLED":
        raise SystemExit("M2 evidence has an unexpected recovered final state")
    if record.get("productionReactUiRuntime") is not False or record.get("androidRuntime") is not False:
        raise SystemExit("M2 foundation evidence must not claim production UI or Android runtime proof")
    if record.get("evidenceStatus") != "M2_FOUNDATION_TRACE_ONLY":
        raise SystemExit("M2 evidence status must remain foundation-trace-only")
    print("M2 foundation evidence: PASS (runtime trace only; production UI/Android runtime not claimed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
