# Windows Diagnostics Expert

Scope: Windows runtime diagnostics — ConPTY console hosting and stream
capture, Job Object isolation and resource limits, named-pipe transport
failures, process supervision and restart behavior, and collecting
event-log and crash-dump evidence (BS §79.7). This skill provides the domain
knowledge that the `Debugging Worker` and `Release Worker` consume.

## Trigger
This skill is requested when windows runtime diagnostics — ConPTY console hosting and stream capture, Job Object isolation and resource limits, named-pipe transport failures, process supervision and restart behavior, and collecting event-log and crash-dump evidence (BS §79.7). This skill provides the domain knowledge that the `Debugging Worker` and `Release Worker` consume.. It does not replace a worker role — it supplies the domain
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
- conpty_probe
- job_object_probe
- event_log

## Procedure
1. Reproduce under observation: capture the exact command, working
   directory, environment, and account that produced the failure.
2. Separate the processes under test: the desktop application, the
   supervisor, and the workers — a symptom in one is frequently a cause
   in another.
3. Inspect the named-pipe transport first: connection establishment,
   framing, and which side closed and why.
4. Inspect ConPTY streams: confirm the console host is attached, that
   output is being drained, and that a full buffer is not the real fault.
5. Inspect Job Object state: which limits are configured, which were
   hit, which processes remain inside the job, and whether a termination
   was a limit kill.
6. Inspect process supervision: expected process count, restart count,
   and the reason recorded for each restart.
7. Collect the durable evidence: relevant event-log entries, structured
   log lines with their correlation identifiers, and a crash dump only
   where a crash is the failure.
8. State the finding with its evidence reference, and distinguish a
   diagnosed cause from a merely correlated symptom.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `DiagnosticsResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * A diagnosis is bound to evidence; a hypothesis without an evidence
  *   reference is reported as a hypothesis, not a cause.
  * Observation is read-only with respect to the failure: reproducing a
  *   fault must not mutate the state that produced it.
  * Correlation is not causation; a log line near a failure is not an
  *   explanation of it.
  * No secret, credential, or user content is copied into a diagnostic
  *   artifact, a log, or a crash dump.
  * A dump is collected only where a crash is the failure, and is
  *   recorded as an artifact with its retention rule.
  * An unverified fix is reported as unverified; a restart that hides a
  *   fault is not a repair.
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

- PROCESS_UNSEPARATED — a symptom was attributed to the wrong process among desktop application, supervisor, and workers.
- CORRELATION_BROKEN — evidence was collected without the correlation identifiers that tie it to the run.
- DIAGNOSED_VS_CORRELATED — a correlation was reported as a diagnosed cause.

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
Emits `DiagnosticsResult` from `DiagnosticsResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Reproduce under observation: capture the exact command, working
- Step 2 produces its expected outcome — Separate the processes under test: the desktop application, the
- Step 3 produces its expected outcome — Inspect the named-pipe transport first: connection establishment,
- Step 4 produces its expected outcome — Inspect ConPTY streams: confirm the console host is attached, that
- Step 5 produces its expected outcome — Inspect Job Object state: which limits are configured, which were
- Step 6 produces its expected outcome — Inspect process supervision: expected process count, restart count,
- Step 7 produces its expected outcome — Collect the durable evidence: relevant event-log entries, structured
- Step 8 produces its expected outcome — State the finding with its evidence reference, and distinguish a
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
