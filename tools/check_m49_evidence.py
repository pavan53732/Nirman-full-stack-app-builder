import json
from pathlib import Path

path = Path(__file__).resolve().parents[1] / "tests/evidence/m49_acceptance.json"
data = json.loads(path.read_text(encoding="utf-8"))
required = {"decisionTraceObserved", "resourceDecisionObserved", "safetyGatesPreservedObserved"}
if not all(data.get(key) is True for key in required):
    raise SystemExit("M49 evidence is missing executable positive observations")
for key in ("nativeWindowsTauriRuntimeObserved", "androidRuntimeObserved"):
    if data.get(key) is not False:
        raise SystemExit(f"M49 runtime claim must remain false: {key}")
if data.get("m49Status") != "M49_HEADLESS_AUTHORITY_TRACE_ONLY":
    raise SystemExit("M49 evidence must remain explicitly headless")
print("M49 evidence: PASS (headless decision and resource authority trace)")
