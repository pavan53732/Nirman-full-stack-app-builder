# Android Background Expert

Scope: Android background execution — WorkManager (deferred, expedited,
periodic, chained work), foreground services (media, location, data sync),
the Android alarm service for exact alarms, Doze/App Standby awareness, broadcast
receivers, and background execution limits (API 30+) (BS §79.7). This
skill provides the background-execution domain knowledge that the
`Android Data and Integration Worker` consumes.

## Trigger
This skill is requested when android background execution — WorkManager (deferred, expedited, periodic, chained work), foreground services (media, location, data sync), the Android alarm service for exact alarms, Doze/App Standby awareness, broadcast receivers, and background execution limits (API 30+) (BS §79.7). This skill provides the background-execution domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_BACKGROUND_EXECUTION`

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
- workmanager_library
- alarm_manager
- broadcast_receiver

## Procedure
1. Analyze background requirements: identify the work type (deferred,
   immediate, periodic, long-running), constraints (network, battery,
   storage), and whether the user needs to perceive the work.
2. Select the right tool: WorkManager for deferrable, guaranteed
   execution; foreground services for immediate, user-visible work;
   the Android alarm service only for exact-time alarms (clocks, reminders).
3. Define WorkManager constraints: setRequiredNetworkType,
   setRequiresBatteryNotLow, setRequiresStorageNotLow,
   setRequiresDeviceIdle. Use setExpedited for immediate work
   (subject to quota).
4. Implement the Worker class: extend CoroutineWorker for suspend
   support, Worker for synchronous work. Return `Result.success()`,
   `Result.retry()`, or `Result.failure()`.
5. Handle foreground services: declare foregroundServiceType in the
   manifest (media, location, dataSync, etc.), show a persistent
   notification, and handle the API 34 foreground service permissions.
6. Test background work: use WorkManagerTestInitHelper for unit tests,
   TestDriver for constraint satisfaction testing.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `BackgroundWorkResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * WorkManager is the default for background work — it handles Doze,
  *    App Standbox, and OEM-specific battery optimizations automatically.
  * Foreground services require a persistent notification — the user must
  *    be aware of ongoing background work. Never start a foreground service
  *    without a notification.
  * Exact alarms require SCHEDULE_EXACT_ALARM permission — use
  *    setExactAndAllowWhileIdle sparingly, as it bypasses Doze.
  * Background execution limits (API 30+) restrict background starts —
  *    use foreground services or WorkManager for reliable execution.
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

- WORK_TOOL_MISMATCH — the work type does not match the tool chosen; deferrable work on a foreground service, or long-running work on a one-shot request.
- CONSTRAINT_NEVER_SATISFIED — the declared constraints cannot all hold, so the work never runs.
- FOREGROUND_SERVICE_TYPE_UNDECLARED — a foreground service runs without its type declared in the manifest.
- WORK_LOST — work was enqueued and did not survive a process death or a restart.

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
Emits `BackgroundWorkResult` from `BackgroundWorkRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze background requirements: identify the work type (deferred,
- Step 2 produces its expected outcome — Select the right tool: WorkManager for deferrable, guaranteed
- Step 3 produces its expected outcome — Define WorkManager constraints: setRequiredNetworkType,
- Step 4 produces its expected outcome — Implement the Worker class: extend CoroutineWorker for suspend
- Step 5 produces its expected outcome — Handle foreground services: declare foregroundServiceType in the
- Step 6 produces its expected outcome — Test background work: use WorkManagerTestInitHelper for unit tests,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
