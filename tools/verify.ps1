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
cargo test --workspace
cargo test -p nirman-control-plane --test m2_vertical_trace
& $Python.Source tools/check_m2_evidence.py
Push-Location apps/desktop
pnpm install --frozen-lockfile
pnpm build
Pop-Location
Write-Host "LOCAL CERTIFICATION: PASS"
