# Android Application State Management

Scope: state management in a generated Android application — owned versus derived state, mutation discipline around a single source of truth, scope and lifetime across configuration changes, and stale reads and update races (BS §79.7).

## Trigger
A state model is designed or reviewed, or the UI shows stale, duplicated, or lost
state and the cause is in how state is held rather than in how it is rendered.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The state in scope is identified, including where each piece is created and where it
  is read.

## Context requirements
- The state model under review and the boundaries between its parts.
- The lifecycle of the screen or session that owns the state.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which parts of the state must survive a configuration change.

## Allowed tools
- static_analyzer
- state_tracer

## Procedure
1. Classify every piece of state as owned or derived, and confirm nothing derived is
   stored — a value computed from other state is computed, not duplicated.
2. Confirm a single source of truth per piece of owned state, naming any place the same
   fact is held twice.
3. Review mutation discipline: every mutation goes through one path, is explicit about
   what it changes, and is not performed from a render path.
4. Determine the scope and lifetime of each store, and confirm nothing outlives the
   screen or session that needs it.
5. Verify what survives a configuration change: state that must survive is owned at a
   surviving scope, and transient state is deliberately not preserved.
6. Trace a stale read to its cause: a missing invalidation, a cached value with no
   freshness rule, or two copies that were never kept in step.
7. Verify concurrent updates are ordered or merged by an explicit rule rather than
   racing to whichever finishes last.

## Evidence
- The owned-versus-derived classification, with any derived value found stored.
- Source-of-truth determination per owned state, with duplicates named.
- Mutation paths, and any mutation found on a render path.
- Lifetime determination per store.
- Configuration-change survival results per state.
- For a stale read, the traced cause.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- DERIVED_STATE_STORED — a computed value is stored as if it were owned state.
- DUPLICATE_SOURCE_OF_TRUTH — the same fact is held in two places.
- STALE_READ — a value was read without a freshness or invalidation rule.
- UPDATE_RACE — concurrent updates reached an order-dependent result.

## Recovery
- Duplicated state is collapsed to one owner, with the other readers deriving from it,
  rather than adding a synchronisation step between the copies.
- A stale read is fixed with an explicit invalidation rule, not by shortening a cache
  duration until the symptom stops appearing.
- An update race is resolved with an explicit ordering or merge rule, not by relying on
  the order the calls happen to return in.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `StateManagementResult` (§23 SkillPackage contract):

- classification of state as owned or derived
- sourcesOfTruth: per owned state, with duplicates named
- mutationPaths: reviewed, with render-path mutations named
- lifetime: per store and configuration-change survival
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured state evidence

## Fixtures
- Derived value computed, not stored
- Single source of truth per owned state
- State survives a configuration change as intended
- Stale read traced to a missing invalidation
- Two copies of the same fact held
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
