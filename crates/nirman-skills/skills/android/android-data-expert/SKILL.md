# Android Data Expert

Scope: Android data persistence and synchronization — Room database
(entities, DAOs, migrations, relationships), DataStore (Preferences
and Proto), offline-first patterns, repository pattern, data sync
strategies, and background data operations (BS §79.7). This skill
provides the data-layer domain knowledge that the `Android Data and
Integration Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides data-specific instruction.

## Workflow
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

## Invariants
- Room queries returning Flow are reactive — the UI automatically
  updates when data changes. Use this for real-time UI.
- All database operations run on Dispatchers.IO — Room enforces this
  for suspend functions and Flow queries.
- Migrations are additive by default — avoid destructive migrations in
  production. Document every migration with the schema change.
- DataStore is the only persistence mechanism for new simple data —
  SharedPreferences is legacy and MUST NOT be used for new code.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
