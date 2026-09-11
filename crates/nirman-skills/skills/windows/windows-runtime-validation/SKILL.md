# Windows Runtime Validation

Scope: native Windows runtime validation — startup, IPC, ConPTY, process
supervision, Job Objects, isolation, restart/recovery, credential
storage, installer/uninstaller behavior (BS §79.7).

## Trigger
This skill is requested when native Windows runtime validation — startup, IPC, ConPTY, process supervision, Job Objects, isolation, restart/recovery, credential storage, installer/uninstaller behavior (BS §79.7).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- dotnet
- process-observer

## Procedure
1. Verify a durable ValidationEnvironment lease exists for a native
   Windows target (BS §79.8). No lease, no validation claim.
2. Launch the built executable on the native host and capture the
   observation set (process identity, runtime output, IPC, recovery).
3. Bind the observations to evidence and update the validation gate.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `BuildGateRecord` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * The output field simulated is fixed to false; a skill that cannot
  *   observe a real native process must not emit a pass.
  * Build evidence is never reinterpreted as runtime evidence
  *   (BS §79.10).
  * A missing environment is a truthful USER_REQUIRED/UNAVAILABLE node,
  *   not a substitute target.
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

- NO_VALIDATION_LEASE — a native run was attempted without a durable ValidationEnvironment lease.
- OBSERVATION_SET_INCOMPLETE — launch succeeded but the observation set was not captured.
- GATE_UPDATED_WITHOUT_EVIDENCE — the validation gate was updated with no evidence bound to it.

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
Emits `BuildGateRecord` from `WindowsRuntimeValidationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Verify a durable ValidationEnvironment lease exists for a native
- Step 2 produces its expected outcome — Launch the built executable on the native host and capture the
- Step 3 produces its expected outcome — Bind the observations to evidence and update the validation gate.
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
