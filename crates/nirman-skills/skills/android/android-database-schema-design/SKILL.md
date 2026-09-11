# Android Database Schema Design

Scope: on-device database schema in a generated Android application — entity modelling, key stability, relationships and referential behaviour, constraints, indexing against real query patterns, and migration safety (BS §79.7).

## Trigger
A schema is created, changed, or reviewed, or a query is slow and the cause is
suspected to be structural rather than in the query text.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The queries the workload actually runs are known, so indexing decisions follow real
  access patterns rather than assumed ones.

## Context requirements
- The schema under review and its current version.
- The query patterns and their relative frequency.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether a destructive change is permitted in this change.

## Allowed tools
- static_analyzer
- query_analyzer

## Procedure
1. Model each table around one entity, and confirm every column depends on the key,
   the whole key, and nothing but the key.
2. Choose primary keys and confirm each is stable: a key is never derived from data a
   user can change later.
3. Model relationships explicitly with foreign keys, and set the behaviour on delete and
   update rather than leaving it implicit.
4. Set constraints and defaults where the database can enforce them, so application code
   does not have to re-check what the schema already guarantees.
5. Index against the observed query patterns: filter columns, join columns, and sort
   columns — and drop any index no query uses.
6. Check each change for migration safety: additive and backward-compatible by default,
   with a destructive change split into expand, migrate, and contract phases.
7. Analyse the access plan for the slowest queries and confirm the chosen index is
   actually used rather than merely present.

## Evidence
- Table and column model, with any dependency violation named.
- Key stability determination per table.
- Relationship definitions with their referential behaviour.
- Index inventory mapped to the query each serves, and unused indexes named.
- Migration-safety determination per change.
- Access plans for the slowest queries.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- UNSTABLE_KEY — a primary key derives from mutable data.
- MISSING_CONSTRAINT — a rule the database could enforce is only checked in
  application code.
- UNUSED_INDEX — an index serves no observed query.
- DESTRUCTIVE_CHANGE — a change drops or alters data without an expand-migrate-contract
  split.

## Recovery
- A destructive change is split into expand, migrate, and contract phases rather than
  applied in one step with a backup as the only safety net.
- A missing constraint is added at the schema level, not by adding a validation that
  duplicates it in every caller.
- An unused index is dropped after confirming no query uses it, since it costs writes
  while serving nothing.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `SchemaDesignResult` (§23 SkillPackage contract):

- model: tables, columns, and dependency findings
- keys: stability determination per table
- indexes: mapped to the query each serves, with unused ones named
- migrationSafety: per-change determination
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured schema evidence

## Fixtures
- Additive migration applied and backward compatible
- Destructive change split into three phases
- Index used by the slowest query's access plan
- Unused index identified and dropped
- Primary key derived from mutable data
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
