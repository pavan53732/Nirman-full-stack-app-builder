# Android Interaction Validation

Scope: driving and verifying user interactions on the Nirman-managed emulator —
resolving elements by identity rather than by coordinates, executing actions
with settle waits, asserting resulting state from the observed hierarchy,
and classifying interaction and assertion failures (BS §79.7).

## Trigger
A behaviour must be exercised the way a user exercises it: tapping,
typing, scrolling, navigating, and asserting that the application reached
the state the scenario requires.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_INTERACTION_EXECUTION`
- `ANDROID_UI_OBSERVATION`

## Preconditions
- A Nirman-managed emulator session is leased and `ANDROID_EMULATOR_EXECUTION`
  resolves to AVAILABLE.
- `ANDROID_INTERACTION_EXECUTION` and `ANDROID_UI_OBSERVATION` resolve to
  AVAILABLE for the target session.
- The application is installed and launched, with install and launch evidence
  from the install and launch skill.
- Elements the scenario touches carry stable test identifiers, or an explicit
  decision is recorded to resolve them another way.

## Context requirements
- The scenario steps, each with its expected observable outcome.
- Element identities — test tags or resource identifiers — not screen
  coordinates.
- The settled state to wait for after each action.
- Revision and artifact digest, and the environment fingerprint of the
  session.

## Allowed tools
- managed_emulator
- adb
- ui_hierarchy_probe

## Procedure
1. Resolve the target element by identity through the observed hierarchy;
   never by pixel coordinate and never by visible text alone, because both
   move under localization and theming.
2. Confirm the element is actionable before acting: visible, enabled, and not
   occluded by another surface.
3. Execute the action and then wait for the settled state, not for a fixed
   duration; a fixed sleep is race, not synchronisation.
4. Re-observe the hierarchy after the action and assert the expected outcome
   from what is actually rendered.
5. Record each step with its before and after hierarchy, so a failure can be
   read without re-running the scenario.
6. On failure, capture the hierarchy at the point of failure and stop the
   scenario; a scenario that continues past a failed assertion produces
   misleading downstream results.
7. Handle interruption — a permission dialog, a system surface, an incoming
   overlay — as a classified state, not as a generic timeout.

## Evidence
- Per-step record: element identity, action executed, settle condition, and
  the resulting observable state.
- The hierarchy observed before and after each action.
- The assertion result for each expected outcome, with the observed value.
- On failure, the hierarchy at the point of failure and the step index.
- Every record bound to revision, artifact digest, and environment
  fingerprint.

## Failure classification
- BLOCKED — a required capability is UNAVAILABLE or USER_REQUIRED.
- ELEMENT_UNRESOLVED — no element matches the identity in the current
  hierarchy; reported with the hierarchy, never fuzzy-matched.
- NOT_ACTIONABLE — the element exists but is invisible, disabled, or
  occluded.
- SETTLE_TIMEOUT — the settled state was not reached within the bound.
- ASSERTION_FAILED — the action succeeded but the expected state was not
  observed.
- INTERRUPTED — a system or application surface took focus; classified, not
  swallowed.

## Recovery
- An unresolved element is retried once after re-observing the hierarchy;
  a still-unresolved element is reported, never approximated.
- A settle timeout widens the bound once for a legitimately slow transition
  and is then reported; the bound is never removed.
- An interruption is handled by the device adapter where the surface is a
  system surface, and escalated where it is the application's own.
- A failed assertion stops the scenario; remaining steps are reported as not
  executed rather than run against a diverged state.
- A blocked capability resumes when the capability record changes, with the
  resume condition named.

## Output contract
Emits `InteractionValidationResult` (§23 SkillPackage contract):

- stepsExecuted: integer, with per-step outcome
- assertions: expected value, observed value, and result for each
- failureIndex: the step at which the scenario stopped, null on success
- failureClass: the classification above, null on success
- hierarchyRefs: observed hierarchies, including the failure point
- evidenceRefs: identifiers of the captured step evidence

## Fixtures
- Tap, type, and scroll resolved by element identity
- Element absent from the hierarchy
- Element present but occluded by an overlay
- Settle timeout on a slow transition
- Assertion failure with hierarchy captured at failure
- Permission dialog interruption mid-scenario
- Capability UNAVAILABLE — blocked, no action executed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT);
every execution still passes through ToolBroker and PolicyAuthority.
