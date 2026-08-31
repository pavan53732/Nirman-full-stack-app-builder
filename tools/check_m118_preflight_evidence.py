"""Validate the M118 real-host environment preflight evidence.

Companion to check_m118_evidence.py (the gate fixtures). This trace is the
`EnvironmentCapabilityPlanner` running the real `OsProbe` against the
developer host at test time: it inspects the environment only — it never
executes a build, a runtime, or a device session. The checker enforces the
honest flags and the structural invariants of the canonical record.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_FILE = ROOT / "tests/evidence/m118_environment_preflight.json"

COVERED_HOSTS = {"linux", "windows"}
FINGERPRINT_RE = re.compile(r"^[0-9a-f]{16}$")


def load(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(
            f"M118 preflight evidence is missing: {path} "
            "(run cargo test -p nirman-tools --test m118_environment_preflight)"
        )
    try:
        evidence = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"M118 preflight evidence is not valid JSON: {path}: {exc}") from exc
    if not isinstance(evidence, dict):
        raise SystemExit(f"M118 preflight evidence must be an object: {path}")
    if not str(evidence.get("schema", "")).startswith("nirman.m118."):
        raise SystemExit(f"M118 preflight evidence schema is incorrect: {path}")
    return evidence


def states(record: dict) -> dict:
    return {c["capability_id"]: c["state"] for c in record.get("capability_results", [])}


def main() -> int:
    evidence = load(EVIDENCE_FILE)

    # Honest flags: the probe inspects, it never executes.
    if evidence.get("observedByRealProbe") is not True:
        raise SystemExit("M118 preflight: observedByRealProbe must be true (this trace is a real OsProbe run)")
    if evidence.get("windowsRuntimeObserved") is not False:
        raise SystemExit("M118 preflight: windowsRuntimeObserved must be false — no Windows runtime was executed")
    if evidence.get("crossBuildExecutedOnHost") is not False:
        raise SystemExit("M118 preflight: crossBuildExecutedOnHost must be false — no build was executed")
    if not isinstance(evidence.get("androidDeviceSessionObserved"), bool):
        raise SystemExit("M118 preflight: androidDeviceSessionObserved must be a truthful boolean")

    record = evidence.get("record")
    if not isinstance(record, dict):
        raise SystemExit("M118 preflight: the canonical record is missing")
    host = record.get("host_platform")
    if host not in COVERED_HOSTS:
        raise SystemExit(f"M118 preflight: host platform {host!r} is not covered by the canonical matrix")
    if record.get("target_platform") != evidence.get("declaredTarget"):
        raise SystemExit("M118 preflight: target platform must equal the declared target")

    fingerprint = record.get("environment_fingerprint", "")
    if not FINGERPRINT_RE.match(fingerprint):
        raise SystemExit(f"M118 preflight: environment fingerprint is not canonical: {fingerprint!r}")
    if record.get("environment_id") != f"env-{host}-{fingerprint}":
        raise SystemExit("M118 preflight: environment_id must be derived from host and fingerprint")

    caps = states(record)
    if len(caps) < 17:
        raise SystemExit(f"M118 preflight: expected the host's full capability set, got {len(caps)} entries")
    for required in ("source_compilation", "windows_native_execution", "cross_build_windows"):
        if required not in caps:
            raise SystemExit(f"M118 preflight: capability {required} is missing from the record")

    # Platform facts and consistency invariants.
    if host != "windows":
        if caps["windows_native_execution"] != "unavailable":
            raise SystemExit("M118 preflight: a non-Windows host may never report native Windows execution")
        if record.get("runtime_validation_available") is not False:
            raise SystemExit("M118 preflight: a non-Windows host may never report runtime validation available")
    if record.get("cross_compilation_available"):
        if caps["cross_build_windows"] != "available":
            raise SystemExit("M118 preflight: cross_compilation_available requires an available cross-build classification")
    if caps["cross_build_windows"] == "available":
        tool_versions = record.get("tool_versions", {})
        for tool in ("rust_target_windows", "windows_linker"):
            if tool not in tool_versions:
                raise SystemExit(
                    f"M118 preflight: cross-build is available but the {tool} observation is missing from tool_versions"
                )
    if record.get("runtime_validation_available") and record.get("host_platform") == record.get("target_platform"):
        # On a matching host the native capability must be available.
        if caps["windows_native_execution"] != "available":
            raise SystemExit("M118 preflight: runtime validation available requires the native capability to be available")

    print(f"M118 environment preflight evidence: PASS (host={host}, fingerprint={fingerprint})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
