# Android Accessibility Validation

Scope: verifying accessibility properties on the Nirman-managed emulator —
semantic labelling of controls, touch target sizing, colour contrast,
traversal order and focus behaviour, and screen reader announcement — with
every criterion reported as verified, failed, or unobserved (BS §79.7).

## Trigger
A screen or flow must meet the accessibility criteria before it is
accepted: as a release gate, or when accessibility is part of the declared
acceptance evidence.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_UI_OBSERVATION`
- `ANDROID_ACCESSIBILITY_VALIDATION`

## Preconditions
- A Nirman-managed emulator session is leased and `ANDROID_EMULATOR_EXECUTION`
  resolves to AVAILABLE.
- `ANDROID_ACCESSIBILITY_VALIDATION` resolves to AVAILABLE for the target
  session.
- The screen is reachable and in a settled state.
- The applicable criteria are known, including the minimum touch target size
  and the contrast threshold the product declares.

## Context requirements
- Screen identity and how to reach it in a settled state.
- The criteria in scope and their thresholds — touch target minimum, contrast
  ratio, and which semantics are required.
- The device configuration, since contrast and layout depend on theme,
  density, and text scaling.
- Revision and artifact digest, and the environment fingerprint.

## Allowed tools
- managed_emulator
- accessibility_probe
- ui_hierarchy_probe

## Procedure
1. Enumerate the interactive elements from the observed hierarchy through
   `ANDROID_UI_OBSERVATION`, including elements a sighted user would not
   notice are interactive.
2. Check semantic labelling: every interactive element has a content
   description or an equivalent semantic label, and a decorative element does
   not claim one.
3. Check touch target sizing against the declared minimum, measuring the
   rendered bounds rather than the declared layout size.
4. Check contrast against the declared threshold for the active theme, and
   repeat for dark and high contrast where those themes ship.
5. Check traversal: focus order follows reading order, every interactive
   element is reachable by keyboard or switch traversal, and focus is always
   visible.
6. Verify announcement through the accessibility service, and record what was
   actually announced rather than what the label contains.
7. Report each criterion as verified, failed, or unobserved; a criterion that
   the probe could not evaluate is unobserved, never assumed to pass.

## Evidence
- Per-element record: identity, semantic label, measured touch target, and
  the criteria evaluated.
- Contrast measurements per text element, with the theme they were measured
  under.
- Traversal order as observed, and the set of elements reachable by focus.
- The announcement captured from the accessibility service.
- Every record bound to revision, artifact digest, and device configuration.

## Failure classification
- BLOCKED — a required capability is UNAVAILABLE or USER_REQUIRED.
- UNLABELLED — an interactive element has no semantic label.
- TARGET_TOO_SMALL — a rendered touch target is below the declared minimum.
- CONTRAST_BELOW_THRESHOLD — measured contrast is below the declared ratio
  for the active theme.
- UNREACHABLE — an interactive element cannot be reached by traversal.
- UNOBSERVED — a criterion could not be evaluated by the probe.

## Recovery
- A criterion that could not be evaluated is reported unobserved and retried
  once after re-settling the screen; it is never defaulted to a pass.
- A contrast failure is re-measured under the intended theme before it is
  reported, because theme drift is a common false positive.
- An unreachable element is escalated as a structural defect rather than
  worked around by reordering the probe.
- A labelled-but-decorative element is reported as noise, since it degrades
  the screen reader experience as much as a missing label.
- The audit does not mutate the screen it measures; any theme or scaling
  change is reverted before the next criterion.

## Output contract
Emits `AccessibilityValidationResult` (§23 SkillPackage contract):

- criteriaEvaluated: criteria in scope, each with a result
- perElement: identity, label, measured target, and contrast where measured
- traversalOrder: the observed order, with unreachable elements named
- verdict: PASS, FAIL, or PARTIAL where some criteria are UNOBSERVED
- unobservedCriteria: criteria the probe could not evaluate
- evidenceRefs: identifiers of the captured accessibility evidence

## Fixtures
- Every interactive element labelled and reachable
- Unlabelled interactive element
- Touch target below the declared minimum
- Contrast below threshold in the active theme
- Element unreachable by traversal
- Criterion unobservable, reported UNOBSERVED
- Capability UNAVAILABLE — blocked, nothing evaluated

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT);
every execution still passes through ToolBroker and PolicyAuthority.
