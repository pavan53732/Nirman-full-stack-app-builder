#!/usr/bin/env bash
# Nirman local certification: the authoritative developer-side validation path.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

printf '%s\n' '== Nirman local certification =='
git diff --check
python3 tools/check_m0.py
python3 tools/verify_contract_graph.py
python3 tools/test_verify_contract_graph.py
cargo fmt --all -- --check
cargo test --workspace
(
  cd apps/desktop
  pnpm install --frozen-lockfile
  pnpm build
)
printf '%s\n' 'LOCAL CERTIFICATION: PASS'
