"""Validate executable M115 desktop boundary evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/desktop_ipc_trace.json"
FINAL_EVIDENCE = ROOT / "tests/evidence/m115_final_acceptance.json"
REQUIRED_TRUE = {
    "fileBackedSqlite",
    "durableCommandCommit",
    "persistedIdempotencyAfterRestart",
    "typedEnvelopeRoundTrip",
    "commandRegistryComplete",
    "subscriptionEnvelopeRoundTrip",
    "errorEnvelopeRoundTrip",
    "androidServiceErrorNormalized",
    "authenticatedProjectScope",
    "snapshotBootstrap",
    "orderedEventDelivery",
    "duplicateRejected",
    "gapAndOutOfOrderRejected",
    "wrongProjectEventRejected",
    "staleProjectionRejected",
}

FINAL_REQUIRED_TRUE = {
    "checkpointReloaded",
    "oldLeaseFenced",
    "reconciliationObserved",
    "newLeaseHeartbeatRecovered",
    "retentionGapObserved",
    "snapshotCursorRecoveryObserved",
    "durableTimeoutAfterRestart",
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit("desktop IPC evidence is missing; run the M115 boundary trace first")
    record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if record.get("schema") != "nirman.desktop_ipc_trace.v2":
        raise SystemExit("unexpected desktop IPC evidence schema")
    if record.get("status") != "M115_HEADLESS_DURABLE_BOUNDARY_TRACE_ONLY":
        raise SystemExit("desktop IPC evidence must remain headless boundary-trace-only")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit(f"desktop IPC evidence is missing executable assertions: {', '.join(missing)}")
    for key in ("tauriCommandRuntime", "reactDomRuntime", "androidRuntime", "apkExport"):
        if record.get(key) is not False:
            raise SystemExit(f"desktop IPC evidence must not claim {key}")
    if not FINAL_EVIDENCE.is_file():
        raise SystemExit("M115 final acceptance evidence is missing; run the final acceptance fixture first")
    final_record = json.loads(FINAL_EVIDENCE.read_text(encoding="utf-8"))
    if final_record.get("schema") != "nirman.m115.final_acceptance.v1":
        raise SystemExit("unexpected M115 final acceptance evidence schema")
    if final_record.get("evidenceStatus") != "M115_HEADLESS_DURABLE_BOUNDARY_TRACE_ONLY":
        raise SystemExit("M115 final acceptance evidence must remain headless boundary-trace-only")
    final_missing = sorted(key for key in FINAL_REQUIRED_TRUE if final_record.get(key) is not True)
    if final_missing:
        raise SystemExit(f"M115 final acceptance evidence is missing executable assertions: {', '.join(final_missing)}")
    for key in ("tauriCommandRuntime", "reactDomRuntime", "androidRuntime", "apkExport"):
        if final_record.get(key) is not False:
            raise SystemExit(f"M115 final acceptance evidence must not claim {key}")
    print("Desktop IPC evidence: PASS (file-backed durable boundary and typed projection semantics only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
