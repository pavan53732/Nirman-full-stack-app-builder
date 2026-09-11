# Windows Desktop Build

Scope: C#/.NET / WinUI 3 / Windows App SDK / XAML host build plus Rust
control-plane integration for Windows x64 — Nirman.exe packaging,
NirmanSupervisor.exe packaging, the named-pipe SupervisorConnection, native
Windows runtime integration, and installer generation (BS §79.7, ADR-108,
ADR-117). The host stack is exactly the one those ADRs lock; no web-wrapper
desktop shell is part of this skill's scope (AGENTS.md §17).

## Trigger
This skill is requested when c#/.NET / WinUI 3 / Windows App SDK / XAML host build plus Rust control-plane integration for Windows x64 — Nirman.exe packaging, NirmanSupervisor.exe packaging, the named-pipe SupervisorConnection, native Windows runtime integration, and installer generation (BS §79.7, ADR-108, ADR-117). The host stack is exactly the one those ADRs lock; no web-wrapper desktop shell is part of this skill's scope (AGENTS.md §17).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

- Gated by the Windows host toolchain. When `WINDOWS_HOST_TOOLCHAIN`
resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
execute and the blocked state MUST be reported — the independent work
(source, static analysis, host-native tests, artifact inspection)
continues rather than being blocked with it.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- dotnet
- msbuild
- cargo

## Procedure
1. Consume the current EnvironmentCapabilityRecord; verify
   WINDOWS_HOST_TOOLCHAIN is AVAILABLE.
2. Build Nirman.exe and NirmanSupervisor.exe natively, bundle, and
   generate the installer.
3. Emit build-gate evidence bound to the environment fingerprint.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `BuildGateRecord` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Never claims runtime validation: the output field
  *   runtimeValidationClaimed is fixed to false. A successful build is an
  *   artifact-production result, not a runtime-validation result
  *   (BS §79.5, §79.10); runtime validation belongs to
  *   windows-runtime-validation.
  * No substitute execution target is introduced or implied.
  * A failed build is reported with the diagnostic reference; it is never
  *   represented as success.
- Every claim reduced to an observable: what was seen, on which device or
  host, at which revision — never a statement of intent.

## Failure classification
- BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- PRECONDITION_UNMET — a precondition below was not satisfied; the skill
  does not proceed past it.
- ACTION_FAILED — a procedure step was attempted and did not produce its
  expected outcome.
- INVARIANT_VIOLATED — the work completed but one of the invariant claims
  this skill must leave observable does not hold.
- TIMEOUT — a wait exceeded its bound; an unbounded wait is a hang, not a
  slow step.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `BuildGateRecord` from `WindowsDesktopBuildRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Consume the current EnvironmentCapabilityRecord; verify
- Step 2 produces its expected outcome — Build Nirman.exe and NirmanSupervisor.exe natively, bundle, and
- Step 3 produces its expected outcome — Emit build-gate evidence bound to the environment fingerprint.
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
