# Nirman local certification: the authoritative Windows developer-side validation path.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "== Nirman local certification =="
git diff --check
python3 tools/check_m0.py
python3 tools/verify_contract_graph.py
python3 tools/test_verify_contract_graph.py
cargo fmt --all -- --check
cargo test --workspace
Push-Location apps/desktop
pnpm install --frozen-lockfile
pnpm build
Pop-Location
Write-Host "LOCAL CERTIFICATION: PASS"
