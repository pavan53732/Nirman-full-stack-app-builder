# Android Runtime Diagnostics

Scope: capturing and interpreting runtime diagnostics on the Nirman-managed
emulator — bounded logcat capture, crash and ANR classification,
correlation of structured application logs with platform logs, component
health probes, and attribution of a fault to a component with evidence (BS §79.7).

## Trigger
A runtime behaviour must be explained: a failed interaction, a crash, an
ANR, a stalled screen, or a discrepancy between expected and observed state.
Diagnosis is requested when the observation alone does not say why.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_LOGCAT_DIAGNOSTICS`
- `ANDROID_UI_OBSERVATION`

## Preconditions
- A Nirman-managed emulator session is leased and `ANDROID_EMULATOR_EXECUTION`
  resolves to AVAILABLE.
- `ANDROID_LOGCAT_DIAGNOSTICS` resolves to AVAILABLE for the target session.
- The fault is reproducible under observation, or an explicit decision is
  recorded to diagnose from an existing capture.
- The artifact, revision, and device identity of the failing run are known,
  so the capture binds to the same build.

## Context requirements
- The failing scenario: steps, expected outcome, and observed outcome.
- Revision and artifact digest of the build under diagnosis.
- Device identity and environment fingerprint of the session.
- The window of interest — a logcat capture is bounded, never a standing
  recording of the whole session.

## Allowed tools
- adb
- managed_emulator
- logcat_probe

## Procedure
1. Bound the capture: define the window by the scenario steps, clear prior
   buffers so earlier runs do not contaminate it, and start capture.
2. Reproduce the fault under observation, capturing the UI hierarchy at the
   point of failure through `ANDROID_UI_OBSERVATION`.
3. Stop capture at the window boundary and persist the logcat output as an
   artifact bound to revision, device, and scenario.
4. Classify the fault from the record: crash with a stack trace, ANR with a
   blocked main thread, a non-fatal exception, a lifecycle stall, or no
   fault found in the window.
5. Correlate the platform log with the application's structured log through
   correlation identifiers, so the platform symptom and the application
   event are one story rather than two.
6. Attribute the fault to a component only when the evidence supports it, and
   name the evidence reference that supports the attribution.
7. Distinguish an application fault from an environment fault: an
   unstartable emulator or a missing capability is an environment state, not
   an application defect.

## Evidence
- The bounded logcat capture, persisted as an artifact with its window,
  revision, and device identity.
- The crash or ANR record where one exists: stack trace, thread state, and
  the component the frames belong to.
- The UI hierarchy at the point of failure, captured through
  `ANDROID_UI_OBSERVATION`.
- The correlation identifiers linking platform log lines to structured
  application events.
- The attribution, with the evidence reference that supports it, or an
  explicit statement that the fault is unattributed.

## Failure classification
- BLOCKED — a required capability is UNAVAILABLE or USER_REQUIRED; no
  capture is attempted and the blocked state is reported.
- NO_FAULT_IN_WINDOW — the scenario completed without reproducing the fault;
  this is a finding, not a pass.
- NOT_REPRODUCIBLE — the fault did not occur under observation; the attempt
  is recorded with its conditions.
- ATTRIBUTION_UNSUPPORTED — the capture shows a symptom but no evidence
  identifies a component; a hypothesis is recorded as a hypothesis.
- CAPTURE_FAILED — the logcat stream could not be read or was truncated.

## Recovery
- A non-reproducing fault is re-attempted once under the recorded
  conditions before it is reported as not reproducible.
- A truncated capture narrows the window and is re-taken; a partial capture
  is never presented as complete.
- An environment fault is escalated to environment repair and the skill
  stops; it does not retry the application scenario against a broken
  environment.
- An unattributed hypothesis is reported with its evidence gap, so the next
  run knows what to capture.
- Nothing about the failure state is mutated while reproducing it; a
  diagnosis that changes the state it measures is invalid.

## Output contract
Emits `RuntimeDiagnosticsResult` (§23 SkillPackage contract):

- faultClass: the classification above
- attributedComponent: the component blamed, or null when unsupported
- evidenceRefs: logcat artifact, crash or ANR record, UI hierarchy capture
- correlationIds: identifiers linking platform and application logs
- window: the bounded capture interval
- environmentFault: boolean, true when the fault is the environment, not
  the application
- confidence: ATTRIBUTED, HYPOTHESIS, or UNATTRIBUTED

## Fixtures
- Crash with a readable stack trace, attributed to a component
- ANR with a blocked main thread
- Non-fatal exception with no visible symptom
- Scenario completes with no fault in the window
- Fault not reproducible under observation
- Truncated capture, re-taken with a narrower window
- Emulator unstartable — environment fault, not an application defect

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT);
every execution still passes through ToolBroker and PolicyAuthority.
