# Android Device State

Scope: observing and reconciling the state of the Nirman-managed emulator and the
application on it — device and application fingerprints, install and session
state, orientation and configuration, connectivity, and power state (BS §79.7).

## Trigger
A run must start from a known state, or a result must be explained by the
state it ran in. Unreconciled state is the most common source of evidence
that cannot be reproduced.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_UI_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `ANDROID_EMULATOR_EXECUTION` resolves to AVAILABLE and a session is leased.
- The state to reconcile toward is declared by the caller — clean, or a named
  fixture state — rather than assumed to mean clean.

## Context requirements
- The target state: reset to clean, or reproduce a named state.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the session, so every record binds to them.
- The application package identifier and the version expected to be installed.

## Allowed tools
- adb
- managed_emulator
- device_state_probe

## Procedure
1. Observe the device identity first: AVD, ABI, API level, orientation,
   density, locale, and the device state fingerprint.
2. Observe the application state: package present, installed version code,
   running processes, and the application state fingerprint.
3. Observe the environment state that changes behaviour: connectivity,
   battery and power-saving mode, and whether the device is locked.
4. Compare observed against declared target state and name each divergence
   rather than reporting a single pass or fail.
5. Reconcile only what the caller asked for: clear application data, reset
   permissions, stop the process, or change orientation — each through the
   transaction path with evidence.
6. Re-observe after reconciliation and confirm the device reached the target
   state before handing control to the next skill.

## Evidence
- Device identity record: AVD, ABI, API level, orientation, density, locale,
  and device state fingerprint.
- Application state record: package, version code, running processes, and
  application state fingerprint.
- The divergence list: each observed value that differed from the declared
  target, and each reconciliation action taken with its outcome.
- Every record bound to the environment fingerprint and the session.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- UNRECONCILABLE — the device could not be brought to the declared target
  state; the divergence is named, not suppressed.
- STATE_UNKNOWN — a probe could not read the state it needed; reported as
  unknown, never guessed.
- SESSION_LOST — the emulator session ended mid-reconciliation.

## Recovery
- A lost session is re-leased and reconciliation restarts from observation;
  a partial reconciliation is never carried forward as complete.
- An unreconciled divergence is escalated as a precondition failure for the
  dependent skill rather than absorbed.
- A state that cannot be probed is reported unknown, and the dependent run
  is marked as running against unverified state.
- One retry is permitted after a repair that materially changed the input;
  an identical reset is never re-run against unchanged evidence.

## Output contract
Emits `DeviceStateResult` (§23 SkillPackage contract):

- deviceIdentity: AVD, ABI, API level, orientation, density, locale
- applicationState: package, version code, processes, fingerprint
- divergences: observed versus declared target, per dimension
- reconciled: boolean, with the actions taken and their outcomes
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured state evidence

## Fixtures
- Clean device reconciled before a run
- Named fixture state reproduced from a clean device
- Stale application data left from a prior run
- Orientation and locale diverge from the declared target
- Connectivity or power-saving state changes behaviour
- Session lost mid-reconciliation

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
