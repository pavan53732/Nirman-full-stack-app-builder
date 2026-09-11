# Windows Crash Recovery

Scope: crash handling and recovery on the Windows host — detecting and classifying a crash,
capturing a dump under a retention rule, reconciling state after an unclean exit, and
restarting without repeating or losing work (BS §79.7).

## Trigger
A Nirman process crashed, or crash handling must be proven: that a crash is detected,
classified, and recovered from rather than leaving the system in an unknown state.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The dump retention rule and where dumps may be written are known.

## Context requirements
- The process that crashed and what it was doing.
- The durable state that may have been left half-written.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The dump retention rule and the allowed dump location.

## Allowed tools
- crash_probe
- event_log
- dump_analyzer

## Procedure
1. Detect the crash from the process exit state and the recorded reason, rather than
   inferring a crash from a missing result.
2. Classify it: unhandled exception, stack exhaustion, access violation, or an explicit
   abort — because each has a different cause.
3. Capture a dump only where a crash is the failure, write it to the allowed location,
   and record it as an artifact with its retention rule.
4. Confirm no credential or user content is captured in the dump, since a dump is a
   copy of process memory.
5. Reconcile durable state: identify transactions that were open at the crash and
   determine which committed and which did not, from the record rather than by
   inspection.
6. Restart and confirm the work resumes at a correct boundary — not repeating committed
   work and not losing uncommitted work that was durably recorded.

## Evidence
- The crash record: process, exit state, recorded reason, and classification.
- The dump artifact reference, its location, and its retention rule.
- The dump content check for credentials and user content.
- The open-transaction reconciliation, with the committed and uncommitted sets.
- The restart outcome: resumed boundary, and whether any work was repeated.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CRASH_UNCLASSIFIED — the exit state does not establish a cause.
- DUMP_UNAVAILABLE — a crash occurred and no dump was captured where one was required.
- STATE_UNRECONCILED — open transactions could not be resolved from the record.
- WORK_REPEATED — committed work ran again after restart.

## Recovery
- An unclassified crash is escalated with the exit state and the log window rather
  than attributed to a likely component.
- Unreconciled state is resolved from the durable record; where the record cannot
  decide, the transaction is marked unknown rather than assumed committed.
- Repeated work is fixed at the resume boundary, and the boundary is proven by a
  crash-during-work fixture rather than by inspection.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `CrashRecoveryResult` (§23 SkillPackage contract):

- crashRecord: process, exit state, reason, classification
- dumpArtifact: reference, location, retention rule
- reconciliation: committed and uncommitted transaction sets
- restartOutcome: resumed boundary and any repeated work
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured crash evidence

## Fixtures
- Unhandled exception captured and classified
- Dump written with its retention rule recorded
- Open transactions reconciled after a crash
- Restart resumes at a correct boundary
- Crash with no dump captured — reported
- Capability UNAVAILABLE — blocked, nothing recovered

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
