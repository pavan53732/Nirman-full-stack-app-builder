#!/usr/bin/env python3
"""Validate the observation-derived portable M4 planning trace."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m4_control_plane_trace.json"
REQUIRED_TRUE = {
    "projectOpenObserved",
    "intentParsed",
    "authenticatedAdmissionObserved",
    "androidOnlyObserved",
    "frameworkSelectionObserved",
    "planEventDurable",
    "noopEditEventDurable",
    "restartReplayObserved",
}
FORBIDDEN_RUNTIME_TRUE = {
    "gradleExecuted": False,
    "emulatorExecuted": False,
    "androidRuntimeObserved": False,
    "nativeTauriRuntimeObserved": False,
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M4 evidence is missing; run the portable integration test first: {EVIDENCE}")
    try:
        record = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M4 evidence is not valid JSON: {exc}") from exc
    if not isinstance(record, dict):
        raise SystemExit("M4 evidence must be a JSON object")
    if record.get("schema") != "nirman.m4.control_plane_trace.v1":
        raise SystemExit("unexpected M4 evidence schema")
    if record.get("fixtureId") != "M4-CONTROL-PLANE-TRACE-001":
        raise SystemExit("unexpected M4 evidence fixture identity")
    missing = sorted(key for key in REQUIRED_TRUE if record.get(key) is not True)
    if missing:
        raise SystemExit("M4 evidence lacks observed positives: " + ", ".join(missing))
    for key, expected in FORBIDDEN_RUNTIME_TRUE.items():
        if record.get(key) is not expected:
            raise SystemExit(f"M4 evidence must keep {key}=false")
    if record.get("selectedLanguage") not in {"kotlin", "java"}:
        raise SystemExit("M4 evidence has an invalid selected language")
    if record.get("selectedUiFramework") not in {"android-views", "jetpack-compose"}:
        raise SystemExit("M4 evidence has an invalid selected Android UI framework")
    if record.get("evidenceStatus") != "M4_SOURCE_ONLY_DURABLE_PLANNING_TRACE":
        raise SystemExit("M4 evidence must remain source-only and headless")
    print("M4 portable planning evidence: PASS (authenticated durable planning trace only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
