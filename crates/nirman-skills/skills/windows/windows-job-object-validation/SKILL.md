# Windows Job Object Validation

Scope: Job Object isolation for Nirman's worker processes — the limits configured, what
happens when a limit is hit, whether processes stay contained, the accounting
counters, and the rules for nested job assignment (BS §79.7).

## Trigger
Worker isolation must be proven: that limits are actually applied, that a limit
violation terminates the job rather than the host, and that no process escapes the
job it was assigned to.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The limits the configuration declares are known: memory, CPU, process count, and
  any UI or clipboard restrictions.

## Context requirements
- The declared limits and where they are configured.
- Which processes must be inside the job.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether nested job assignment is expected in this configuration.

## Allowed tools
- job_object_probe
- process_probe

## Procedure
1. Read the job's configured limits from the object itself rather than from the
   configuration that set them.
2. Confirm every process that must be inside the job is inside it, and that none was
   silently dropped at assignment.
3. Drive a process toward the memory limit and observe the outcome at the boundary:
   it is terminated, and the termination is recorded as a limit kill.
4. Drive CPU and process-count limits the same way and record each outcome.
5. Verify containment: a child spawned by a job member stays in the job, and no
   process is found outside it that should be inside.
6. Read the accounting counters and confirm they agree with the observed behaviour
   rather than being read as truth without corroboration.
7. Where nesting is expected, confirm the assignment follows the rule; where it is
   not, confirm a nested assignment is refused.

## Evidence
- The limits read from the job object, each with its source.
- Containment verification: processes inside, and any that escaped.
- Per-limit outcome at the boundary, with the termination reason recorded.
- Accounting counters, corroborated against observed behaviour.
- Every record bound to revision and host environment fingerprint.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - Every declared limit is present on the job object and read from it, not from the configuration that set it.
  - No process that belongs inside the job is found outside it.
  - A termination caused by a limit is recorded as a limit kill, never as a plain exit.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- LIMIT_NOT_APPLIED — a declared limit is absent from the job object.
- CONTAINMENT_BREACH — a process that must be inside the job was found outside it.
- LIMIT_KILL_UNRECORDED — a limit termination happened without being recorded as
  such.
- NESTING_VIOLATION — a job was assigned where the configuration forbids it.

## Recovery
- A containment breach is fixed at the assignment path, not by widening the job to
  include the escape.
- An unrecorded limit kill is fixed in the termination path so the reason is
  durable; the kill is never reported as a plain process exit.
- A nesting violation is resolved by following the assignment rule, not by allowing
  the assignment and ignoring the rule.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `JobObjectResult` (§23 SkillPackage contract):

- limitsConfigured: limit, value, and source, read from the job object
- containment: processes inside, and any outside that belong inside
- limitOutcomes: per-limit behaviour at the boundary
- accounting: counters read, corroborated
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured job-object evidence

## Fixtures
- All declared limits present on the job object
- Memory limit reached and job terminated with reason recorded
- Child process stays inside the job
- Process found outside the job that belongs inside
- Nested assignment refused where forbidden
- Capability UNAVAILABLE — blocked, nothing validated

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
