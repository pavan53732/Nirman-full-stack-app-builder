# Android Offline Sync

Scope: offline-first data in a generated Android application — queueing local writes while disconnected, detecting and resolving conflicts, replaying queued operations in a correct and idempotent order, and reconciling local state after a partition (BS §79.7).

## Trigger
Offline behaviour is designed or reviewed, or a user reports lost writes, duplicated
writes, or stale data after reconnecting.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The conflict domain is known: which records can be edited concurrently, and what a
  correct resolution looks like for each.

## Context requirements
- The records in the sync domain and their conflict semantics.
- The connectivity states the app must tolerate, including a partition mid-write.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether the server or the client is authoritative for each record type.

## Allowed tools
- static_analyzer
- sync_simulator

## Procedure
1. Confirm every local write is queued durably before it is attempted, so a crash or a
   restart does not lose a pending write.
2. Verify the queue is replayed in a defined order, and that the order is preserved
   across restarts rather than being whatever the queue happens to yield.
3. Verify every replayed operation is idempotent, so a write that succeeded but whose
   acknowledgement was lost is not applied twice.
4. Detect conflicts explicitly: a version, timestamp, or vector is compared, and a
   mismatch is a conflict rather than a silent overwrite.
5. Apply the declared resolution per record type, and record that a resolution happened
   rather than resolving silently.
6. Reconcile after a partition by replaying the queue and pulling server changes, then
   confirming the local state matches the server for every record in the domain.
7. Verify the user-visible outcome: a resolved conflict is surfaced where the user needs
   to know, and is never hidden behind a successful sync status.

## Evidence
- Queue durability evidence: a pending write survives a simulated crash.
- Replay order observed, and preserved across a restart.
- Idempotency evidence: a replayed acknowledged write applied once.
- Conflict detection results, with each detected conflict recorded.
- Reconciliation result: local versus server per record after reconnect.
- User-visible outcome for each resolved conflict.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - A queued write survives a crash and is replayed exactly once.
  - A conflict is detected before either version overwrites the other.
  - After reconciliation, local and server state agree for every record in the domain.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- LOST_WRITE — a queued write did not reach the server and was not reported.
- DUPLICATE_WRITE — a replay applied an already-applied write.
- SILENT_OVERWRITE — a conflict was resolved without being detected or recorded.
- RECONCILE_MISMATCH — local and server state differ after reconciliation.

## Recovery
- A lost write is fixed in the queue's durability, not by retrying on a timer and
  hoping the window closes.
- A duplicate write is fixed with an idempotency key, not by de-duplicating on the
  server after the fact.
- A silent overwrite is fixed by comparing versions before writing, and the detected
  conflict is surfaced to the user rather than resolved invisibly.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `OfflineSyncResult` (§23 SkillPackage contract):

- queueDurability: pending writes surviving a crash
- replayOrder: observed order and restart preservation
- conflicts: detected, resolved, and surfaced
- reconciliation: local versus server per record
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured sync evidence

## Fixtures
- Write queued offline and replayed on reconnect
- Acknowledged write replayed once
- Concurrent edit detected and resolved per the declared rule
- Reconciliation leaves local and server in agreement
- Partition mid-write loses the queued write
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
