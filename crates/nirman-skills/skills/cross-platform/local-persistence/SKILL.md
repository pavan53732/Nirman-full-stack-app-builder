# Local Persistence

Scope: on-device persistence for Nirman apps — choosing the right store, writing
transactionally, versioning the schema and checking integrity, bounding a cache and
evidencing eviction, and keeping sensitive values in secure storage (BS §79.7).

## Trigger
A persistence layer is designed or reviewed, or data is lost, corrupted, or grows
without bound on device.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The data shapes and their access patterns are known, so the store selection follows
  them rather than convenience.

## Context requirements
- The data shapes to persist and how each is read and written.
- What must survive an app update, and what is disposable cache.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which values are sensitive enough to require secure storage.

## Allowed tools
- storage_probe
- static_analyzer

## Procedure
1. Select the store per data shape from the access pattern: structured and relational,
   key-value and small, or bulk file storage — and record the reason for each choice.
2. Confirm every multi-record write is transactional, so a partial write is never left
   readable as if it were complete.
3. Version the schema and verify the on-device version is read and migrated rather than
   assumed current.
4. Check integrity on open, and define what happens when corruption is detected rather
   than letting a corrupt store surface as a crash.
5. Bound every cache by size or age, and verify eviction actually occurs at the bound
   rather than the cache growing until storage runs out.
6. Place sensitive values in the platform's secure storage, and confirm none of them is
   present in a plain store, a backup, or a log.
7. Verify the survival rule: what persists across an update and what is cleared, and
   that the observed behaviour matches the declared one.

## Evidence
- Store selection per data shape, with the reason recorded.
- Transaction boundaries, and the partial-write test outcome.
- Schema version read from the device and the migration outcome.
- Integrity check results, including the corrupted-store case.
- Cache growth against the declared bound, with eviction observed at the bound.
- Sensitive-value placement, and the scan of plain stores, backups, and logs.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- NON_TRANSACTIONAL_WRITE — a multi-record write can leave a partial state.
- UNMIGRATED_SCHEMA — the on-device version was not read or migrated.
- CACHE_UNBOUNDED — a cache grows without evicting at its declared bound.
- SENSITIVE_IN_PLAIN_STORE — a value requiring secure storage was found elsewhere.

## Recovery
- A partial write is fixed with a transaction, not by validating the records after
  reading them back.
- Unbounded cache growth is fixed with eviction at the declared bound, not by clearing
  the whole cache when storage runs low.
- A sensitive value found in a plain store is moved and the plain copy is removed; the
  finding is not closed by deleting only the live row.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `PersistenceResult` (§23 SkillPackage contract):

- storeSelection: per data shape, with reasons
- transactions: boundaries and partial-write outcome
- schemaVersion: read from device and migrated
- cacheBounds: growth against the bound, eviction observed
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured persistence evidence

## Fixtures
- Multi-record write is transactional
- On-device schema migrated on open
- Cache evicts at its declared bound
- Corrupted store detected and handled rather than crashing
- Sensitive value found outside secure storage
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
