# Rust Control Plane Expert

Scope: Rust control-plane engineering — crate layout and workspace
boundaries, async runtime discipline, typed error handling, named-pipe
interprocess communication, structured logging and diagnostics, and Rust
test and static-analysis practice (BS §79.7). This skill provides the domain
knowledge that the `Architecture Worker` and `Debugging Worker` consume.

## Trigger
This skill is requested when rust control-plane engineering — crate layout and workspace boundaries, async runtime discipline, typed error handling, named-pipe interprocess communication, structured logging and diagnostics, and Rust test and static-analysis practice (BS §79.7). This skill provides the domain knowledge that the `Architecture Worker` and `Debugging Worker` consume.. It does not replace a worker role — it supplies the domain
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

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- cargo
- rustc
- clippy

## Procedure
1. Keep crate boundaries honest: one crate per responsibility,
   dependencies pointing one way, and no cycle between crates.
2. Put every long-running or concurrent path on an explicit async runtime
   and name it; nothing spawns a detached task that nobody owns.
3. Cancel and bound deliberately: every spawned task is joinable, every
   wait has a bound, and shutdown is a coordinated sequence rather than a
   process kill.
4. Model failure as typed errors with context, not as strings, so a
   caller can distinguish retryable from terminal without parsing a
   message.
5. Build interprocess communication on named pipes with framed messages,
   explicit versioning, and defined behavior for peer disconnection.
6. Log structurally: stable event names, typed fields, and correlation
   identifiers — never secrets, tokens, or user content.
7. Test at the boundary that matters: unit tests for pure logic,
   integration tests at the pipe and process boundaries, and static
   analysis in the build rather than in review.
8. Verify on the host: build with static analysis enabled, run the suite,
   and confirm the binaries start, connect, and shut down cleanly.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `ControlPlaneResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * No unwrap or panic on a reachable error path in control-plane code; a
  *   panic in a long-lived process is a defect, not an error-handling
  *   strategy.
  * Every task is owned and joinable; no task outlives its owner silently.
  * Every wait is bounded; an unbounded wait is a hang waiting to happen.
  * Named-pipe framing is versioned; a peer speaking an older version is
  *   rejected explicitly, never partially parsed.
  * No secret, credential, or user content is written to a log, a memory
  *   record, or a crash report.
  * An unverified path is reported as unverified; a successful compile is
  *   not evidence of correct concurrent behavior.
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

- CRATE_BOUNDARY_VIOLATION — a dependency points the wrong way, or two crates share one responsibility.
- UNBOUNDED_TASK — a spawned task is not joinable, or a wait has no bound.
- FRAMING_ERROR — an IPC message was truncated, oversized, or mis-framed.
- UNTYPED_ERROR — a failure surfaced as a string, so a caller cannot distinguish retryable from terminal.

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
Emits `ControlPlaneResult` from `ControlPlaneResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Keep crate boundaries honest: one crate per responsibility,
- Step 2 produces its expected outcome — Put every long-running or concurrent path on an explicit async runtime
- Step 3 produces its expected outcome — Cancel and bound deliberately: every spawned task is joinable, every
- Step 4 produces its expected outcome — Model failure as typed errors with context, not as strings, so a
- Step 5 produces its expected outcome — Build interprocess communication on named pipes with framed messages,
- Step 6 produces its expected outcome — Log structurally: stable event names, typed fields, and correlation
- Step 7 produces its expected outcome — Test at the boundary that matters: unit tests for pure logic,
- Step 8 produces its expected outcome — Verify on the host: build with static analysis enabled, run the suite,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
