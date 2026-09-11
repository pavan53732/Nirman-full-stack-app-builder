# Android Data Migration

Scope: schema and data migrations for a generated Android application's on-device database — chain ordering and versioning, backward compatibility, reversible versus destructive steps, and verifying integrity after a migration (BS §79.7).

## Trigger
A migration is written or reviewed, or data is suspected lost or altered after one
ran.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The starting version on the devices or databases in scope is known, since a
  migration runs from whatever version is present and not from the latest one.

## Context requirements
- The migration chain and the versions in the field.
- Which steps are reversible and which are destructive.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The integrity checks that must hold before and after.

## Allowed tools
- static_analyzer
- migration_runner

## Procedure
1. Version every migration and confirm the chain applies from any version present in
   the field, not only from the immediately preceding one.
2. Order the migrations and confirm each is applied exactly once, with the applied set
   recorded durably rather than inferred.
3. Split every destructive change into expand, migrate, and contract phases so the old
   and new shapes coexist until the data has moved.
4. Keep the intermediate state backward compatible, so an app on the previous version
   still works against a half-migrated store.
5. Record reversibility per step, and for any destructive step record the restore path
   rather than relying on a backup existing.
6. Run the migration on a copy that reflects the field's oldest version, then verify
   integrity: row counts, constraints, and checks on the values that were transformed.
7. Verify the post-migration state is readable by both the old and the new version where
   the contract requires it.

## Evidence
- The migration chain, with the versions it was proven to apply from.
- Applied-set record, showing each migration applied exactly once.
- Phase split per destructive change.
- Reversibility and restore path per step.
- Integrity results from the copy-based run: counts, constraints, and value checks.
- Compatibility result for both versions where required.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - The chain applies from every version present in the field, not only from the immediately preceding one.
  - Each migration is applied exactly once, and the applied set is recorded durably.
  - After migration, row counts, constraints, and transformed values all verify.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CHAIN_GAP — no path exists from a version present in the field.
- APPLIED_TWICE — a migration ran more than once.
- INTEGRITY_LOST — row counts, constraints, or transformed values failed after the
  migration.
- IRREVERSIBLE_UNRECORDED — a destructive step has no recorded restore path.

## Recovery
- A chain gap is fixed by adding the missing step, not by requiring devices to
  reinstall from a current version.
- Lost integrity is fixed in the transformation and re-run on a fresh copy; the
  migration is never declared successful on a spot check of a few rows.
- An irreversible step without a restore path is blocked until a verified restore path
  exists, rather than proceeding because a backup is assumed.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `MigrationResult` (§23 SkillPackage contract):

- chain: versions covered and any gap
- appliedSet: each migration applied exactly once
- phases: expand-migrate-contract split per destructive change
- integrity: counts, constraints, and value checks after the run
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured migration evidence

## Fixtures
- Migration applies from the oldest version in the field
- Destructive change split into three phases
- Integrity verified on a copy before release
- Migration applied twice detected
- Capability UNAVAILABLE — blocked, nothing migrated

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
