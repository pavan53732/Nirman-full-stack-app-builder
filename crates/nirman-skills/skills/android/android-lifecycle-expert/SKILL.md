# Android Lifecycle Expert

Scope: component and process lifecycle — configuration change and recreation,
state save and restore, process death and restoration, lifecycle-aware
collection, foreground and background transitions, and the regression cases a
lifecycle must survive (BS §79.7).

## Trigger
A screen must survive rotation, resize, folding, backgrounding, and process
death without losing state — or a bug is suspected to be a lifecycle bug:
lost state, duplicated work, or work running after its scope ended.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_SOURCE_ENGINEERING`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `ANDROID_SOURCE_ENGINEERING` resolves to AVAILABLE for static analysis.
- The component under review is identified, with the state it owns.

## Context requirements
- The components in scope and the state each one owns.
- Which state must survive recreation and which is deliberately transient.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the session, so every record binds to them.
- The regression cases the component must survive: rotation, resize,
  backgrounding, and process death.

## Allowed tools
- adb
- managed_emulator
- static_analyzer

## Procedure
1. Classify the state each component owns: state that must survive recreation,
   state that is deliberately transient, and state that belongs elsewhere.
2. Move state that must survive into a holder that outlives the component, and
   keep transient state out of it so it is not resurrected stale.
3. Handle configuration change by surviving it, not by suppressing it; a
   locked orientation or a suppressed recreation is a workaround, not a fix.
4. Make collection lifecycle-aware so work stops when its scope ends and does
   not restart a duplicate on recreation.
5. Save and restore the small state the platform can deliver, and persist the
   rest durably rather than trusting a large parcel.
6. Test process death explicitly: background the app, kill the process, and
   restore — then verify the restored state matches what the user left.
7. Statically check for work launched in a scope it can outlive, and for
   collection started without a bound to its lifecycle.

## Evidence
- The state classification: what survives recreation, what is transient, and
  where each piece now lives.
- Per-case restoration results: rotation, resize, backgrounding, and process
  death, each with the state before and after.
- Static analysis findings for unbounded collection and for work launched in
  a scope it can outlive.
- Every record bound to revision and artifact digest.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - State the contract requires to survive recreation is restored identically after rotation, resize, backgrounding, and process death.
  - No work outlives the scope that owns it.
  - A restored screen never shows a value that was transient at the moment of death.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- STATE_LOST — state that must survive recreation did not.
- DUPLICATE_WORK — collection or work restarted on recreation, producing two
  live copies.
- ORPHANED_WORK — work outlived its scope and kept running after it.
- SUPPRESSED_RECREATION — the component suppresses configuration change
  instead of surviving it.

## Recovery
- Lost state is restored by moving it to a holder that outlives the component,
  not by disabling the configuration change that exposed it.
- Duplicate work is fixed by making collection lifecycle-aware and by owning
  each job exactly once across recreation.
- A suppressed recreation is removed and the component made to survive it;
  the check is never relaxed to keep the workaround.
- One retry is permitted after a repair that materially changed the input;
  an identical case is never re-run against unchanged evidence.

## Output contract
Emits `LifecycleResult` (§23 SkillPackage contract):

- stateClassification: survives, transient, or relocated per piece of state
- casesExercised: rotation, resize, backgrounding, process death
- restorationResults: state before and after, per case
- staticFindings: unbounded collection and orphaned work
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured lifecycle evidence

## Fixtures
- Rotation preserves state and resumes collection once
- Resize and fold preserve state
- Process death and restoration return the user's state
- Duplicate collection caught on recreation
- Work orphaned after its scope ended
- Capability UNAVAILABLE — blocked, nothing analysed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
