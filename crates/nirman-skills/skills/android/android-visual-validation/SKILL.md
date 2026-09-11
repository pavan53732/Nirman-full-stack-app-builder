# Android Visual Validation

Scope: capturing rendered screens on the Nirman-managed emulator and comparing
them against the design specification — screenshot capture bound to
revision and device, golden image comparison under a defined difference
policy, design token fidelity, and truthful reporting of visual criteria
that could not be observed (BS §79.7).

## Trigger
A screen's appearance must be checked against its design: after a UI
change, as a regression gate, or when a visual criterion is part of the
acceptance evidence.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_UI_OBSERVATION`
- `ANDROID_VISUAL_VALIDATION`

## Preconditions
- A Nirman-managed emulator session is leased and `ANDROID_EMULATOR_EXECUTION`
  resolves to AVAILABLE.
- `ANDROID_VISUAL_VALIDATION` resolves to AVAILABLE for the target session.
- A golden image or a design reference exists for the screen at the same
  device configuration; comparison without a reference is not validation.
- The screen is reachable and in a settled state — animations complete and
  no transient surface is on top.

## Context requirements
- Screen identity and the route or interaction that reaches it.
- Golden image or design reference, with the device configuration it was
  produced at: API level, resolution, density, theme, and locale.
- Revision and artifact digest under test.
- The difference policy: tolerance, and which regions are exempt and why.

## Allowed tools
- managed_emulator
- screenshot_capture
- image_diff

## Procedure
1. Reach the screen deterministically through `ANDROID_UI_OBSERVATION`, then
   wait for a settled state: no running animation, no transient overlay.
2. Capture the screenshot through the platform capture path, bound to
   revision, artifact digest, and device identity.
3. Compare against the golden image only at the same configuration; a golden
   from a different density or theme is not comparable and is reported as
   such rather than diffed anyway.
4. Apply the difference policy: compute the diff, apply the declared
   tolerance, and evaluate exempt regions deliberately rather than by
   blanket exclusion.
5. Classify each difference as a design deviation, a platform rendering
   difference, or an anti-aliasing artefact, because they have different
   owners.
6. Report unobservable criteria honestly: a visual criterion that could not
   be captured is NOT_OBSERVED, never inferred and never defaulted to a
   pass.
7. Record the comparison as evidence bound to the golden image identity, so
   the result cannot be replayed against a different reference.

## Evidence
- The captured screenshot as an artifact, bound to revision, artifact
  digest, and device identity.
- The golden image identity and the device configuration it was produced at.
- The difference result: measured difference, applied tolerance, and the
  classification of each differing region.
- The declared exempt regions with the reason each is exempt.
- An explicit NOT_OBSERVED entry for every visual criterion that could not
  be captured.

## Failure classification
- BLOCKED — a required capability is UNAVAILABLE or USER_REQUIRED.
- NO_REFERENCE — no golden image or design reference exists at a matching
  configuration; reported, never diffed against a mismatched one.
- UNSETTLED — the screen did not reach a settled state within the bound, so
  any diff would be unstable.
- DEVIATION — the difference exceeds the declared tolerance and is
  attributable to the implementation.
- NOT_OBSERVED — a criterion could not be captured; reported as unobserved,
  never as a pass.

## Recovery
- An unsettled screen is re-captured after extending the settle wait once;
  a repeatedly unsettled screen is reported rather than diffed.
- A missing reference at the target configuration is escalated to capture a
  new golden, with policy admission; a golden is never captured from an
  unverified screen.
- A platform rendering difference is recorded as platform behaviour with
  its evidence, not silently absorbed into the tolerance.
- A deviation is reported with its region and measurement; it is never
  resolved by widening the tolerance to make it disappear.
- Re-capture is bound to the same device configuration; a comparison across
  configurations is void.

## Output contract
Emits `VisualValidationResult` (§23 SkillPackage contract):

- captured: boolean, with screenshot artifact reference
- compared: boolean, with golden image identity and configuration
- difference: measured value, applied tolerance, and per-region
  classification
- exemptRegions: regions excluded, each with its reason
- verdict: MATCH, DEVIATION, NO_REFERENCE, UNSETTLED, or NOT_OBSERVED
- unobservedCriteria: criteria that could not be captured
- evidenceRefs: capture and comparison evidence identifiers

## Fixtures
- Screen matches its golden image at the same configuration
- Deviation exceeding the declared tolerance
- Golden missing at the target density
- Screen never reaches a settled state
- Platform rendering difference versus implementation deviation
- Visual criterion reported NOT_OBSERVED
- Capability UNAVAILABLE — blocked, nothing captured

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT);
every execution still passes through ToolBroker and PolicyAuthority.
