"""Validate observation-derived M45 AndroidCodeIntelligence evidence."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/evidence/m45_android_code_intelligence.json"
REQUIRED_TRUE = {
    "representativeLanguageAdaptersObserved",
    "fullSemanticGraphObserved",
    "projectFingerprintObserved",
    "deterministicReloadObserved",
    "deterministicRepeatIndexObserved",
    "excludedBuildCacheVendorPathsObserved",
    "affectedFilesAndModulesObserved",
    "affectedTestsObserved",
    "affectedPermissionsResourcesPreviewArtifactsObserved",
    "readOnlyNoWorkspaceMutationObserved",
}


def main() -> int:
    if not EVIDENCE.is_file():
        raise SystemExit(f"M45 evidence is missing: {EVIDENCE}")
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    if evidence.get("schema") != "nirman.m45.android_code_intelligence.v1":
        raise SystemExit("M45 evidence schema is incorrect")
    missing = sorted(key for key in REQUIRED_TRUE if evidence.get(key) is not True)
    if missing:
        raise SystemExit("M45 evidence has no observed positive result for: " + ", ".join(missing))
    if evidence.get("structuredMutationBroker") is not False:
        raise SystemExit("M45 evidence must not claim M46 mutation broker implementation")
    if evidence.get("androidBuildObserved") is not False:
        raise SystemExit("M45 evidence must not claim Android build execution")
    if evidence.get("nativeWindowsTauriRuntimeObserved") is not False:
        raise SystemExit("M45 evidence must not claim native Windows/Tauri runtime proof")
    if evidence.get("evidenceStatus") != "M45_HEADLESS_READ_ONLY_INDEX_TRACE_ONLY":
        raise SystemExit("M45 evidence must remain explicitly read-only and headless")
    print("M45 AndroidCodeIntelligence evidence: PASS (headless read-only index trace)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
