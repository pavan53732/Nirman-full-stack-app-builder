# Windows Process Supervision

Scope: supervision of Nirman's own processes — the expected process inventory, start and
stop ordering, restart policy and the reason each restart records, orphan detection,
and coordinated shutdown (BS §79.7).

## Trigger
The process model must be proven, or a fault involves a process that is missing,
duplicated, restarting repeatedly, or outliving its owner.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The expected process inventory for the configuration under test is known: which
  executables run, how many of each, and who owns them.

## Context requirements
- The expected inventory and the owner of each process.
- The restart policy in force, including its attempt bound.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The shutdown sequence the supervisor is expected to run.

## Allowed tools
- process_probe
- event_log

## Procedure
1. Enumerate the running processes and compare against the expected inventory,
   naming each extra and each missing process rather than counting alone.
2. Verify ownership: each process was started by the supervisor and is known to it,
   so nothing is running unmanaged.
3. Exercise the start ordering and confirm dependencies come up before their
   consumers.
4. Kill a supervised process and observe the restart: it restarts, the attempt is
   counted, and the reason is recorded durably.
5. Verify the attempt bound: repeated failures escalate rather than looping forever,
   and the escalation is visible rather than silent.
6. Run a coordinated shutdown and confirm every process exits, in order, with none
   orphaned and none killed without a chance to flush.

## Evidence
- The observed process inventory against the expected one, with each discrepancy
  named.
- Ownership verification per process.
- Restart observations: attempt count, recorded reason, and outcome.
- Shutdown trace: order, exit codes, and any process that failed to exit.
- Every record bound to revision and host environment fingerprint.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - The running inventory matches the expected inventory, or each discrepancy is named.
  - Every process is owned by the supervisor; none runs unmanaged.
  - A coordinated shutdown leaves no orphan and kills nothing denied a bounded chance to flush.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- INVENTORY_MISMATCH — a process is missing, extra, or duplicated.
- UNMANAGED_PROCESS — a process is running that no owner supervises.
- RESTART_LOOP — restarts repeat past the attempt bound without escalating.
- ORPHAN_ON_SHUTDOWN — a process outlived its owner or was killed without
  flushing.

## Recovery
- A restart loop is escalated rather than retried again; the bound is never raised
  to let the loop continue.
- An orphan is fixed in the shutdown sequence so the process is given a bounded
  chance to flush, then terminated deliberately.
- An unmanaged process is brought under supervision or removed; it is not left
  running because nothing claims it.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `SupervisionResult` (§23 SkillPackage contract):

- expectedInventory and observedInventory, with discrepancies named
- restarts: attempts, recorded reasons, and outcomes
- shutdownTrace: order, exit codes, orphans
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured supervision evidence

## Fixtures
- Inventory matches under normal operation
- Supervised process killed and restarted with reason recorded
- Attempt bound reached and escalated
- Coordinated shutdown with no orphans
- Unmanaged process detected
- Capability UNAVAILABLE — blocked, nothing supervised

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
