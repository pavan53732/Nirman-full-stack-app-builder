#!/usr/bin/env bash
# Nirman local certification: the authoritative developer-side validation path.
set -euo pipefail

if [[ -f "$HOME/.cargo/env" ]]; then
  # Prefer rustup-managed toolchains when present; Tauri 2.11 requires Rust >= 1.77.2.
  source "$HOME/.cargo/env"
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

printf '%s\n' '== Nirman local certification =='
git diff --check
python3 tools/check_m0.py
python3 tools/verify_contract_graph.py
python3 tools/test_verify_contract_graph.py
cargo fmt --all -- --check
cargo test --workspace --exclude nirman-desktop
if command -v rustup >/dev/null 2>&1 && rustup target list --installed | grep -qx 'x86_64-pc-windows-gnu'; then
  cargo check --target x86_64-pc-windows-gnu --manifest-path apps/desktop/src-tauri/Cargo.toml
else
  printf '%s\n' 'Tauri Windows target check: SKIPPED (x86_64-pc-windows-gnu target is not installed)'
fi
cargo test -p nirman-control-plane --test m2_vertical_trace
cargo test -p nirman-domain --test m39_contract
cargo test -p nirman-project --test m45_acceptance
cargo test -p nirman-project --test m46_acceptance
cargo test -p nirman-android
cargo test -p nirman-android --test m47_acceptance
cargo test -p nirman-preview
cargo test -p nirman-preview --test m48_acceptance
cargo test -p nirman-agents --test m49_acceptance
cargo test -p nirman-control-plane --test m115_final_acceptance
cargo test -p nirman-providers --test m3_acceptance
cargo test -p nirman-providers m44_bridge -- --nocapture
cargo test -p nirman-desktop --bin nirman-desktop -- --nocapture
cargo test -p nirman-ipc --test desktop_ipc_trace
cargo test -p nirman-ipc --test m115_acceptance
python3 tools/check_m2_evidence.py
python3 tools/check_desktop_ipc_evidence.py
python3 tools/check_m3_provider_evidence.py
python3 tools/check_m39_evidence.py
python3 tools/check_m45_evidence.py
python3 tools/check_m46_evidence.py
python3 tools/check_m43_evidence.py
python3 tools/check_m44_evidence.py
python3 tools/check_m47_evidence.py
python3 tools/check_m48_evidence.py
python3 tools/check_m49_evidence.py
(
  cd apps/desktop
  pnpm install --frozen-lockfile
  cd ../..
  node --experimental-strip-types tests/desktop_projection_store_trace.ts
  cd apps/desktop
  pnpm build
)
printf '%s\n' 'LOCAL CERTIFICATION: PASS'
