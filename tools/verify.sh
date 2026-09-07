#!/usr/bin/env bash
# tools/verify.sh — Nirman local certification entry point (Unix-like environments).
#
# Contract: ADR-204 (local certification is authoritative; hosted CI is optional)
#           development plan M0, work item "Local certification pipeline".
#
# Counterpart: tools/verify.ps1 (Windows). ADR-204 requires the two entry points
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
# Usage: tools/verify.sh [--quick]
#   --quick  skip the documentation conformance (mutation) harness, which is the
#            slowest gate. The default run always executes it.

set -uo pipefail

TOOLS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${TOOLS_DIR}/.." && pwd)"
cd "${ROOT}" || exit 1

QUICK=0
for arg in "$@"; do
  case "${arg}" in
    --quick) QUICK=1 ;;
    -h|--help)
      echo "usage: tools/verify.sh [--quick]"
      exit 0
      ;;
    *)
      echo "unknown argument: ${arg}" >&2
      echo "usage: tools/verify.sh [--quick]" >&2
      exit 2
      ;;
  esac
done

GATE_IDS=()
GATE_STATUS=()
GATE_DETAIL=()

record() {
  GATE_IDS+=("$1")
  GATE_STATUS+=("$2")
  GATE_DETAIL+=("$3")
}

have() { command -v "$1" >/dev/null 2>&1; }

LOGDIR="$(mktemp -d)"
KEEP_LOGS=0

# Run a command as a gate. Exit status 0 => PASS, anything else => FAIL.
run_gate() {
  local id="$1"
  shift
  local log="${LOGDIR}/${id}.log"
  if "$@" >"${log}" 2>&1; then
    record "${id}" "PASS" ""
  else
    KEEP_LOGS=1
    record "${id}" "FAIL" "log: ${log}"
  fi
}

PYTHON_BIN=""
for candidate in python3 python py; do
  if have "${candidate}"; then
    PYTHON_BIN="${candidate}"
    break
  fi
done

echo "Nirman local certification — ADR-204, development plan M0"
echo "root: ${ROOT}"
echo "python: ${PYTHON_BIN:-<none>}"
echo

# ---------------------------------------------------------------- documentation
if [ -z "${PYTHON_BIN}" ]; then
  record "documentation" "USER_REQUIRED" "no python interpreter found on PATH"
else
  run_gate "documentation" "${PYTHON_BIN}" tools/verify_contract_graph.py .
fi

# -------------------------------------------------- documentation conformance
if [ -z "${PYTHON_BIN}" ]; then
  record "documentation-conformance" "USER_REQUIRED" "no python interpreter found on PATH"
elif [ "${QUICK}" -eq 1 ]; then
  record "documentation-conformance" "UNAVAILABLE" "skipped by --quick"
else
  run_gate "documentation-conformance" "${PYTHON_BIN}" tools/test_verify_contract_graph.py
fi

# -------------------------------------------------------------------- foundation
# M0 foundation: the Cargo workspace must exist and parse; the module-boundary
# rule (TA §57.1 dependency directions) is enforced once the workspace exists.
if [ ! -f "Cargo.toml" ]; then
  record "foundation" "UNAVAILABLE" "Cargo workspace not created yet; no subject to evaluate"
elif ! have cargo; then
  record "foundation" "USER_REQUIRED" "Cargo.toml present but cargo is not installed"
else
  run_gate "foundation" cargo metadata --format-version 1 --no-deps
fi

# ------------------------------------------------------------------------- rust
if ! have cargo; then
  record "rust" "USER_REQUIRED" "cargo is not installed"
elif [ ! -f "Cargo.toml" ]; then
  record "rust" "UNAVAILABLE" "Rust supervisor sources not created yet"
else
  run_gate "rust" bash -c 'cargo fmt --all --check && cargo test --workspace'
fi

# ------------------------------------------------------------------------- host
# WinUI 3 / Windows App SDK / .NET host. There is no web frontend (DP M0).
HOST_SOLUTION=""
if have dotnet; then
  for candidate in $(find . -maxdepth 3 \( -name '*.sln' -o -name '*.slnx' \) \
      -not -path '*/target/*' -not -path '*/bin/*' -not -path '*/obj/*' 2>/dev/null); do
    HOST_SOLUTION="${candidate}"
    break
  done
fi

if ! have dotnet; then
  record "host" "USER_REQUIRED" "dotnet is not installed"
elif [ -z "${HOST_SOLUTION}" ]; then
  record "host" "UNAVAILABLE" "no .NET solution found; host sources not created yet"
else
  run_gate "host" bash -c "dotnet build '${HOST_SOLUTION}' --nologo && dotnet test '${HOST_SOLUTION}' --nologo"
fi

# ---------------------------------------------------------------------- fixture
# M0 requires at least three representative Android projects under fixtures/.
FIXTURE_COUNT=0
if [ -d fixtures ]; then
  for candidate in fixtures/*; do
    [ -d "${candidate}" ] || continue
    if [ -f "${candidate}/settings.gradle" ] || [ -f "${candidate}/settings.gradle.kts" ] \
       || [ -f "${candidate}/build.gradle" ] || [ -f "${candidate}/build.gradle.kts" ]; then
      FIXTURE_COUNT=$((FIXTURE_COUNT + 1))
    fi
  done
  if [ "${FIXTURE_COUNT}" -lt 3 ]; then
    record "fixture" "FAIL" "M0 requires at least 3 Android fixtures, found ${FIXTURE_COUNT}"
  else
    record "fixture" "PASS" "${FIXTURE_COUNT} Android fixtures present"
  fi
else
  record "fixture" "UNAVAILABLE" "fixtures/ not created yet; no subject to evaluate"
fi

# ----------------------------------------------------------------------- static
# Security baseline (DP M0): secret material must never be tracked. This gate is
# evaluated on every run because its subject is the tracked file set itself.
if ! have git; then
  record "static" "USER_REQUIRED" "git is not installed"
else
  SECRETS=""
  while IFS= read -r tracked; do
    base="$(basename "${tracked}")"
    case "${base}" in
      .env|.env.*|*.pem|*.key|*.p12|*.pfx|*.keystore|*.jks|*.mobileprovision|\
*.publishsettings|secrets.json|id_rsa|id_dsa|id_ecdsa|id_ed25519)
        SECRETS="${SECRETS}    ${tracked}"$'\n'
        ;;
    esac
  done < <(git ls-files)
  if [ -n "${SECRETS}" ]; then
    record "static" "FAIL" "tracked secret material detected:"$'\n'"${SECRETS}"
  else
    record "static" "PASS" "no tracked secret material"
  fi
fi

# ------------------------------------------------------------------- reporting
echo "gate results"
max_id_len=0
for id in "${GATE_IDS[@]}"; do
  [ "${#id}" -gt "${max_id_len}" ] && max_id_len="${#id}"
done

failed=0
unevaluated=0
for i in "${!GATE_IDS[@]}"; do
  printf "  %-14s %-${max_id_len}s" "${GATE_STATUS[$i]}" "${GATE_IDS[$i]}"
  if [ -n "${GATE_DETAIL[$i]}" ] && [ "${GATE_STATUS[$i]}" != "PASS" ]; then
    printf "  %s" "${GATE_DETAIL[$i]}"
  fi
  printf "\n"
  case "${GATE_STATUS[$i]}" in
    FAIL) failed=$((failed + 1)) ;;
    UNAVAILABLE|USER_REQUIRED) unevaluated=$((unevaluated + 1)) ;;
  esac
done

echo
if [ "${KEEP_LOGS}" -eq 1 ]; then
  echo "gate logs retained at: ${LOGDIR}"
else
  rm -rf "${LOGDIR}" 2>/dev/null || true
fi

if [ "${failed}" -gt 0 ]; then
  echo "CERTIFICATION: LOCAL_CERTIFICATION_FAIL"
  exit 1
fi

if [ "${unevaluated}" -gt 0 ]; then
  echo "UNEVALUATED GATES (${unevaluated}) — required subject or toolchain absent:"
  for i in "${!GATE_IDS[@]}"; do
    case "${GATE_STATUS[$i]}" in
      UNAVAILABLE|USER_REQUIRED)
        echo "  [${GATE_IDS[$i]}] ${GATE_STATUS[$i]}: ${GATE_DETAIL[$i]}"
        ;;
    esac
  done
  echo
  echo "exit code 0 reflects zero defects among the evaluated gates, not complete"
  echo "evaluation, and is not RUNTIME_CERTIFIED."
  echo "CERTIFICATION: LOCAL_CERTIFICATION_INCOMPLETE"
  exit 0
fi

echo "CERTIFICATION: LOCAL_CERTIFICATION_PASS"
exit 0
