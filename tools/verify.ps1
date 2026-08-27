# Nirman local certification: the authoritative Windows developer-side validation path.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "== Nirman local certification =="
$Python = Get-Command python3 -ErrorAction SilentlyContinue
if (-not $Python) { $Python = Get-Command python -ErrorAction SilentlyContinue }
if (-not $Python) { throw "Python 3 is required for local certification" }
git diff --check
& $Python.Source tools/check_m0.py
& $Python.Source tools/verify_contract_graph.py
& $Python.Source tools/test_verify_contract_graph.py
cargo fmt --all -- --check
cargo test --workspace --exclude nirman-desktop
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo test -p nirman-control-plane --test m2_vertical_trace
cargo test -p nirman-domain --test m39_contract
cargo test -p nirman-project --test m45_acceptance
cargo test -p nirman-project --test m46_acceptance
cargo test -p nirman-android
cargo test -p nirman-control-plane --test m115_final_acceptance
cargo test -p nirman-providers --test m3_acceptance
cargo test -p nirman-providers m44_bridge -- --nocapture
cargo test -p nirman-desktop --bin nirman-desktop -- --nocapture
cargo test -p nirman-ipc --test desktop_ipc_trace
cargo test -p nirman-ipc --test m115_acceptance
& $Python.Source tools/check_m2_evidence.py
& $Python.Source tools/check_desktop_ipc_evidence.py
& $Python.Source tools/check_m3_provider_evidence.py
& $Python.Source tools/check_m39_evidence.py
& $Python.Source tools/check_m45_evidence.py
& $Python.Source tools/check_m46_evidence.py
& $Python.Source tools/check_m43_evidence.py
& $Python.Source tools/check_m44_evidence.py
Push-Location apps/desktop
pnpm install --frozen-lockfile
Pop-Location
node --experimental-strip-types tests/desktop_projection_store_trace.ts
Push-Location apps/desktop
pnpm build
Pop-Location
Write-Host "LOCAL CERTIFICATION: PASS"
