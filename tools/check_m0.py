#!/usr/bin/env python3
"""Validate the repository/runtime foundation required by M0."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

REQUIRED_CRATES = {
    "nirman-control-plane",
    "nirman-domain",
    "nirman-storage",
    "nirman-ipc",
    "nirman-supervisor",
    "nirman-agents",
    "nirman-workers",
    "nirman-policy",
    "nirman-tools",
    "nirman-providers",
    "nirman-project",
    "nirman-android",
    "nirman-preview",
    "nirman-evidence",
    "nirman-recovery",
    "nirman-artifacts",
}

FIXTURES = [
    ROOT / "fixtures/android/minimal/fixture.json",
    ROOT / "fixtures/android/compose/fixture.json",
    ROOT / "fixtures/android/react-native/fixture.json",
]


def main() -> int:
    workspace = ROOT / "Cargo.toml"
    desktop_manifest = ROOT / "apps/desktop/package.json"
    runtime_config = ROOT / "config/runtime.example.json"

    required_files = [workspace, desktop_manifest, runtime_config, ROOT / "AGENTS.md"]
    missing = [str(path.relative_to(ROOT)) for path in required_files if not path.exists()]
    if missing:
        raise SystemExit(f"M0 missing required files: {', '.join(missing)}")

    workspace_text = workspace.read_text(encoding="utf-8")
    for crate in REQUIRED_CRATES:
        crate_dir = ROOT / "crates" / crate
        manifest = crate_dir / "Cargo.toml"
        source = crate_dir / "src" / "lib.rs"
        if f'"crates/{crate}"' not in workspace_text:
            raise SystemExit(f"M0 workspace is missing crate declaration: {crate}")
        if not crate_dir.is_dir() or not manifest.is_file() or not source.is_file():
            raise SystemExit(f"M0 declared crate is missing its directory, manifest, or source: {crate}")
        if f'name = "{crate}"' not in manifest.read_text(encoding="utf-8"):
            raise SystemExit(f"M0 crate manifest name does not match workspace identity: {crate}")

    fixture_ids: set[str] = set()
    for fixture_path in FIXTURES:
        fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        fixture_ids.add(fixture["fixtureId"])
        if fixture["targetPlatforms"] != ["android"]:
            raise SystemExit(f"M0 fixture is not Android-only: {fixture_path}")
        if not fixture.get("technology") or not fixture.get("purpose") or not fixture.get("requiredChecks"):
            raise SystemExit(f"M0 fixture profile is incomplete: {fixture_path}")
        if fixture.get("synthetic") is True:
            raise SystemExit(f"M0 Android fixture profile must not be marked synthetic: {fixture_path}")

    if len(fixture_ids) != len(FIXTURES):
        raise SystemExit("M0 fixture IDs must be unique")

    config = json.loads(runtime_config.read_text(encoding="utf-8"))
    forbidden = [key for key in config if any(secret in key.lower() for secret in ("key", "token", "password", "secret"))]
    if forbidden:
        raise SystemExit(f"M0 example configuration contains secret-bearing fields: {forbidden}")
    if config.get("allowRemoteExecution") is not False or config.get("allowExternalDeployment") is not False:
        raise SystemExit("M0 local-only execution and deployment boundaries are not closed")

    package = json.loads(desktop_manifest.read_text(encoding="utf-8"))
    if package.get("private") is not True or package.get("scripts", {}).get("build") is None:
        raise SystemExit("M0 desktop package is missing private/build safeguards")

    print(f"M0 foundation: PASS ({len(REQUIRED_CRATES)} Rust crates, {len(FIXTURES)} Android fixture profiles and manifests)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
