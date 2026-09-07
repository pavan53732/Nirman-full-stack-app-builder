#Requires -Version 5.1
# tools/verify.ps1 -- Nirman local certification entry point (Windows PowerShell).
#
# Contract: ADR-204 (local certification is authoritative; hosted CI is optional)
#           development plan M0, work item "Local certification pipeline".
#
# Counterpart: tools/verify.sh (Unix-like). ADR-204 requires the two entry points
# to stay aligned: same gates, same order, same status vocabulary. Any change to
# this file requires the equivalent change to the other.
#
# Status vocabulary
#   PASS           the gate was evaluated and passed
#   FAIL           the gate was evaluated and failed
#   UNAVAILABLE    the subject this gate evaluates does not exist yet
#   USER_REQUIRED  the subject exists but the toolchain needed to evaluate it is
#                  not installed on this host
#
# Exit codes
#   0  no gate failed. If any gate is UNAVAILABLE or USER_REQUIRED the run is
#      incomplete and the terminal status says so. Exit 0 therefore means zero
#      defects among the gates that could be evaluated, NOT complete evaluation.
#   1  at least one gate failed
#   2  usage error
#
# Usage: tools\verify.ps1 [-Quick]
#   -Quick  skip the documentation conformance (mutation) harness, which is the
#           slowest gate. The default run always executes it.

[CmdletBinding()]
param(
    [switch]$Quick
)

$ErrorActionPreference = 'Continue'

$ToolsDir = $PSScriptRoot
$Root = Split-Path -Parent $ToolsDir
Set-Location $Root

$Gates = New-Object System.Collections.Generic.List[object]
$KeepLogs = $false

$LogDir = Join-Path ([System.IO.Path]::GetTempPath()) ("nirman-verify-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $LogDir -Force | Out-Null

function Record {
    param([string]$Id, [string]$Status, [string]$Detail)
    $Gates.Add([PSCustomObject]@{ Id = $Id; Status = $Status; Detail = $Detail })
}

function Get-Tool {
    param([string[]]$Names)
    foreach ($name in $Names) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    return $null
}

function Invoke-GateSequence {
    param([string]$Id, [object[]]$Steps)
    $log = Join-Path $LogDir ($Id + '.log')
    $all = @()
    $code = 0
    foreach ($step in $Steps) {
        $out = & $step.Exe @($step.Arguments) 2>&1
        $all += $out
        if ($LASTEXITCODE -ne 0) { $code = $LASTEXITCODE; break }
    }
    $all | Out-File -FilePath $log -Encoding utf8
    if ($code -eq 0) {
        Record $Id 'PASS' ''
    } else {
        $script:KeepLogs = $true
        Record $Id 'FAIL' "log: $log"
    }
}

$Python = Get-Tool @('python3', 'python', 'py')

Write-Output "Nirman local certification -- ADR-204, development plan M0"
Write-Output "root: $Root"
Write-Output ("python: " + $(if ($Python) { $Python } else { '<none>' }))
Write-Output ""

# ---------------------------------------------------------------- documentation
if (-not $Python) {
    Record 'documentation' 'USER_REQUIRED' 'no python interpreter found on PATH'
} else {
    Invoke-GateSequence 'documentation' @(
        @{ Exe = $Python; Arguments = @('tools/verify_contract_graph.py', '.') }
    )
}

# --------------------------------------------------- documentation conformance
if (-not $Python) {
    Record 'documentation-conformance' 'USER_REQUIRED' 'no python interpreter found on PATH'
} elseif ($Quick) {
    Record 'documentation-conformance' 'UNAVAILABLE' 'skipped by -Quick'
} else {
    Invoke-GateSequence 'documentation-conformance' @(
        @{ Exe = $Python; Arguments = @('tools/test_verify_contract_graph.py') }
    )
}

# -------------------------------------------------------------------- foundation
# M0 foundation: the Cargo workspace must exist and parse; the module-boundary
# rule (TA §57.1 dependency directions) is enforced once the workspace exists.
$Cargo = Get-Tool @('cargo')
if (-not (Test-Path 'Cargo.toml')) {
    Record 'foundation' 'UNAVAILABLE' 'Cargo workspace not created yet; no subject to evaluate'
} elseif (-not $Cargo) {
    Record 'foundation' 'USER_REQUIRED' 'Cargo.toml present but cargo is not installed'
} else {
    Invoke-GateSequence 'foundation' @(
        @{ Exe = $Cargo; Arguments = @('metadata', '--format-version', '1', '--no-deps') }
    )
}

# ------------------------------------------------------------------------- rust
if (-not $Cargo) {
    Record 'rust' 'USER_REQUIRED' 'cargo is not installed'
} elseif (-not (Test-Path 'Cargo.toml')) {
    Record 'rust' 'UNAVAILABLE' 'Rust supervisor sources not created yet'
} else {
    Invoke-GateSequence 'rust' @(
        @{ Exe = $Cargo; Arguments = @('fmt', '--all', '--check') },
        @{ Exe = $Cargo; Arguments = @('test', '--workspace') }
    )
}

# ------------------------------------------------------------------------- host
# WinUI 3 / Windows App SDK / .NET host. There is no web frontend (DP M0).
$Dotnet = Get-Tool @('dotnet')
$Solution = $null
if ($Dotnet) {
    $found = Get-ChildItem -Path . -Recurse -Include '*.sln', '*.slnx' -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch '[\\/](target|bin|obj)[\\/]' } |
        Select-Object -First 1
    if ($found) { $Solution = $found.FullName }
}

if (-not $Dotnet) {
    Record 'host' 'USER_REQUIRED' 'dotnet is not installed'
} elseif (-not $Solution) {
    Record 'host' 'UNAVAILABLE' 'no .NET solution found; host sources not created yet'
} else {
    Invoke-GateSequence 'host' @(
        @{ Exe = $Dotnet; Arguments = @('build', $Solution, '--nologo') },
        @{ Exe = $Dotnet; Arguments = @('test', $Solution, '--nologo') }
    )
}

# ---------------------------------------------------------------------- fixture
# M0 requires at least three representative Android projects under fixtures/.
if (Test-Path 'fixtures') {
    $count = 0
    foreach ($dir in Get-ChildItem -Path 'fixtures' -Directory -ErrorAction SilentlyContinue) {
        $markers = @('settings.gradle', 'settings.gradle.kts', 'build.gradle', 'build.gradle.kts')
        foreach ($marker in $markers) {
            if (Test-Path (Join-Path $dir.FullName $marker)) { $count++; break }
        }
    }
    if ($count -lt 3) {
        Record 'fixture' 'FAIL' "M0 requires at least 3 Android fixtures, found $count"
    } else {
        Record 'fixture' 'PASS' "$count Android fixtures present"
    }
} else {
    Record 'fixture' 'UNAVAILABLE' 'fixtures/ not created yet; no subject to evaluate'
}

# ----------------------------------------------------------------------- static
# Security baseline (DP M0): secret material must never be tracked. This gate is
# evaluated on every run because its subject is the tracked file set itself.
$Git = Get-Tool @('git')
if (-not $Git) {
    Record 'static' 'USER_REQUIRED' 'git is not installed'
} else {
    $patterns = @('.env', '.env.*', '*.pem', '*.key', '*.p12', '*.pfx', '*.keystore',
        '*.jks', '*.mobileprovision', '*.publishsettings', 'secrets.json',
        'id_rsa', 'id_dsa', 'id_ecdsa', 'id_ed25519')
    $tracked = & $Git ls-files 2>&1
    $hits = @()
    foreach ($file in $tracked) {
        $base = Split-Path -Leaf $file
        foreach ($pattern in $patterns) {
            if ($base -like $pattern) { $hits += "    $file"; break }
        }
    }
    if ($hits.Count -gt 0) {
        Record 'static' 'FAIL' ("tracked secret material detected: " + [string]::Join(' ', $hits))
    } else {
        Record 'static' 'PASS' 'no tracked secret material'
    }
}

# ------------------------------------------------------------------- reporting
Write-Output "gate results"
$maxIdLen = ($Gates | ForEach-Object { $_.Id.Length } | Measure-Object -Maximum).Maximum

$failed = 0
$unevaluated = 0
foreach ($gate in $Gates) {
    $line = "  " + $gate.Status.PadRight(14) + $gate.Id.PadRight($maxIdLen)
    if ($gate.Detail -and $gate.Status -ne 'PASS') { $line += "  " + $gate.Detail }
    Write-Output $line
    switch ($gate.Status) {
        'FAIL' { $failed++ }
        'UNAVAILABLE' { $unevaluated++ }
        'USER_REQUIRED' { $unevaluated++ }
    }
}

Write-Output ""
if ($KeepLogs) {
    Write-Output "gate logs retained at: $LogDir"
} else {
    Remove-Item -Recurse -Force $LogDir -ErrorAction SilentlyContinue
}

if ($failed -gt 0) {
    Write-Output "CERTIFICATION: LOCAL_CERTIFICATION_FAIL"
    exit 1
}

if ($unevaluated -gt 0) {
    Write-Output "UNEVALUATED GATES ($unevaluated) -- required subject or toolchain absent:"
    foreach ($gate in $Gates) {
        if ($gate.Status -in @('UNAVAILABLE', 'USER_REQUIRED')) {
            Write-Output ("  [" + $gate.Id + "] " + $gate.Status + ": " + $gate.Detail)
        }
    }
    Write-Output ""
    Write-Output "exit code 0 reflects zero defects among the evaluated gates, not complete"
    Write-Output "evaluation, and is not RUNTIME_CERTIFIED."
    Write-Output "CERTIFICATION: LOCAL_CERTIFICATION_INCOMPLETE"
    exit 0
}

Write-Output "CERTIFICATION: LOCAL_CERTIFICATION_PASS"
exit 0
