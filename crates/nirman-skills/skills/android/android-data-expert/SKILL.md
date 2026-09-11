# Android Data Expert

Scope: Android data persistence and synchronization — Room database
(entities, DAOs, migrations, relationships), DataStore (Preferences
and Proto), offline-first patterns, repository pattern, data sync
strategies, and background data operations (BS §79.7). This skill
provides the data-layer domain knowledge that the `Android Data and
Integration Worker` consumes.

## Trigger
This skill is requested when android data persistence and synchronization — Room database (entities, DAOs, migrations, relationships), DataStore (Preferences and Proto), offline-first patterns, repository pattern, data sync strategies, and background data operations (BS §79.7). This skill provides the data-layer domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_SOURCE_ENGINEERING`

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
- room_compiler
- datastore_library
- migration_tester

## Procedure
1. Analyze data requirements: identify entities, relationships, query
   patterns, offline needs, sync frequency, and data volume.
2. Design the Room schema: define entities (with primary keys, indices,
   foreign keys), DAOs (with suspend functions, Flow-returning queries,
   transactions), and the RoomDatabase class.
3. Handle migrations: use Migration classes for schema changes,
   fallbackToDestructiveMigration only for development. Every migration
   MUST be tested with a migration test.
4. Implement DataStore: use Proto DataStore for typed data with schema
   evolution, Preferences DataStore for simple key-value pairs. Never
   use SharedPreferences for new code.
5. Design the repository layer: coordinate Room (local) and API (remote)
   data sources. Implement `offline-first` by emitting local data first,
   then fetching remote and updating local.
6. Handle background data operations: use WorkManager for periodic sync,
   expedited work for immediate sync, with proper constraints
   (network type, battery).
7. Test the data layer: use Room.inMemoryDatabaseBuilder for DAO tests,
   AndroidJUnit4 for instrumented tests with real database.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `DataLayerResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Room queries returning Flow are reactive — the UI automatically
  *   updates when data changes. Use this for real-time UI.
  * All database operations run on Dispatchers.IO — Room enforces this
  *   for suspend functions and Flow queries.
  * Migrations are additive by default — avoid destructive migrations in
  *   production. Document every migration with the schema change.
  * DataStore is the only persistence mechanism for new simple data —
  *   SharedPreferences is legacy and MUST NOT be used for new code.
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

- MIGRATION_MISSING — a schema change was made without a Migration, so existing data cannot be opened.
- DESTRUCTIVE_MIGRATION_ENABLED — a destructive fallback is active on a path that must preserve user data.
- SINGLE_SOURCE_VIOLATION — an entity has more than one source of truth, so local and remote can disagree.

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
Emits `DataLayerResult` from `DataLayerRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze data requirements: identify entities, relationships, query
- Step 2 produces its expected outcome — Design the Room schema: define entities (with primary keys, indices,
- Step 3 produces its expected outcome — Handle migrations: use Migration classes for schema changes,
- Step 4 produces its expected outcome — Implement DataStore: use Proto DataStore for typed data with schema
- Step 5 produces its expected outcome — Design the repository layer: coordinate Room (local) and API (remote)
- Step 6 produces its expected outcome — Handle background data operations: use WorkManager for periodic sync,
- Step 7 produces its expected outcome — Test the data layer: use Room.inMemoryDatabaseBuilder for DAO tests,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
