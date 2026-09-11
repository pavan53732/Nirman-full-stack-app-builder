# Android Health Services Expert

Scope: Health and fitness data — Health Connect read and write with
granular permissions and disclosure, Health Services exercise and passive
sensor tracking on Wear OS, heart rate and location series, records and
aggregations, and durable sync of health data (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `UI Worker` consume.

## Trigger
This skill is requested when health and fitness data — Health Connect read and write with granular permissions and disclosure, Health Services exercise and passive sensor tracking on Wear OS, heart rate and location series, records and aggregations, and durable sync of health data (BS §79.7). This skill provides the domain knowledge that the `Android Data and Integration Worker` and `UI Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NATIVE_DEVICE_CAPABILITIES`

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
- health_connect_client
- health_services
- managed_emulator

## Procedure
1. Determine the data path: Health Connect as the on-device store for
   records the user owns, and Health Services for live exercise and
   passive sensor sampling while a session runs.
2. Declare granular permissions for exactly the record types used, and
   request them at the point of use rather than at startup.
3. Check availability explicitly: Health Connect may be absent or not
   updated, and the app must offer the install path instead of failing
   silently.
4. Write records with a client-generated record id and a deterministic
   time range so a retry is idempotent rather than duplicative.
5. Read with aggregations and time ranges rather than fetching entire
   histories into memory; page large reads.
6. For live tracking, run an exercise session through Health Services:
   declare the exercise type and data types up front, sample at the
   required rate, and keep the session in a foreground service with a
   visible ongoing notification.
7. Verify on the Nirman-managed emulator: grant and deny each
   permission, confirm the denial path degrades gracefully, and confirm
   written records read back with the same values and time ranges.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `HealthIntegrationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Permissions are granular and requested at the point of use, never in
  *   a startup batch.
  * Missing Health Connect is reported with an install path, never
  *   treated as empty data.
  * Writes are idempotent: a retry with the same record id does not
  *   duplicate a record.
  * A denied permission degrades the feature; it never blocks the rest of
  *   the app and never fabricates a value.
  * Health data is handled as sensitive: no health value is written to
  *   logs, memory records, or crash reports.
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
Emits `HealthIntegrationResult` from `HealthIntegrationResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Determine the data path: Health Connect as the on-device store for
- Step 2 produces its expected outcome — Declare granular permissions for exactly the record types used, and
- Step 3 produces its expected outcome — Check availability explicitly: Health Connect may be absent or not
- Step 4 produces its expected outcome — Write records with a client-generated record id and a deterministic
- Step 5 produces its expected outcome — Read with aggregations and time ranges rather than fetching entire
- Step 6 produces its expected outcome — For live tracking, run an exercise session through Health Services:
- Step 7 produces its expected outcome — Verify on the Nirman-managed emulator: grant and deny each
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
