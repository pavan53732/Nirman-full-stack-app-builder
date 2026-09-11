# Android Accessibility Expert

Scope: Android accessibility — TalkBack support, content descriptions,
touch target sizing (48dp minimum), color contrast ratios (4.5:1 for
text), accessibility scanner, semantic roles, and accessibility test
automation (BS §79.7). This skill provides the accessibility domain
knowledge that the `Visual QA Worker` and `UI Worker` consume.

## Trigger
This skill is requested when android accessibility — TalkBack support, content descriptions, touch target sizing (48dp minimum), color contrast ratios (4.5:1 for text), accessibility scanner, semantic roles, and accessibility test automation (BS §79.7). This skill provides the accessibility domain knowledge that the `Visual QA Worker` and `UI Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_UI_OBSERVATION`
- `ANDROID_ACCESSIBILITY_VALIDATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- accessibility_scanner
- talkback
- color_contrast_analyzer

## Procedure
1. Analyze accessibility requirements: identify user journeys that need
   TalkBack support, interactive elements, and dynamic content.
2. Add content descriptions: every non-text element (images, icons,
   buttons) has a contentDescription. Decorative elements are marked
   with `Modifier.clearAndSetSemantics {}`.
3. Ensure touch target sizing: every interactive element is at least
   48dp × 48dp. Use `Modifier.minimumInteractiveComponentSize()` or
   `Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)`.
4. Verify color contrast: text-to-background contrast ratio is 4.5:1
   for normal text, 3:1 for large text. Use the Accessibility Scanner
   to verify.
5. Add semantic roles: use `Modifier.semantics { role = Role.Button }`
   for custom interactive elements. Group related elements with
   `Modifier.semantics(mergeDescendants = true)`.
6. Test with TalkBack: enable TalkBack and verify every screen is
   navigable, every action is announced, and every state change is
   communicated. Use AccessibilityScanner for automated checks.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `AccessibilityImplementationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every non-text interactive element has a content description —
  *   contentDescription is required for accessibility. Empty descriptions
  *   are only for decorative elements.
  * Touch targets are 48dp minimum — smaller targets are flagged by the
  *   Accessibility Scanner and MUST be enlarged.
  * Color is not the sole indicator — information conveyed with color
  *   (errors, selection) has an additional visual indicator (icon, text).
  * Dynamic content changes are announced — use `Modifier.semantics {
  *   liveRegion = LiveRegionMode.Polite }` for status updates.
- Every claim reduced to an observable: what was seen, on which device or
  host, at which revision — never a statement of intent.

## Failure classification
- BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- PRECONDITION_UNMET — a precondition below was not satisfied; the skill
  does not proceed past it.
- ACTION_FAILED — a procedure step was attempted and did not produce its
  expected outcome.
- INVARIANT_VIOLATED — the work completed but one of the invariant claims
  this skill must leave observable does not hold.
- TIMEOUT — a wait exceeded its bound; an unbounded wait is a hang, not a
  slow step.

- MISSING_CONTENT_DESCRIPTION — a non-text element carries no description, so a screen reader announces nothing for it.
- TOUCH_TARGET_TOO_SMALL — an interactive element is under 48dp in either dimension.
- CONTRAST_BELOW_THRESHOLD — measured contrast is under 4.5:1 for normal text or 3:1 for large text.
- TALKBACK_UNREACHABLE — a screen or action cannot be reached or announced with TalkBack enabled.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `AccessibilityImplementationResult` from `AccessibilityImplementationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze accessibility requirements: identify user journeys that need
- Step 2 produces its expected outcome — Add content descriptions: every non-text element (images, icons,
- Step 3 produces its expected outcome — Ensure touch target sizing: every interactive element is at least
- Step 4 produces its expected outcome — Verify color contrast: text-to-background contrast ratio is 4.5:1
- Step 5 produces its expected outcome — Add semantic roles: use `Modifier.semantics { role = Role.Button }`
- Step 6 produces its expected outcome — Test with TalkBack: enable TalkBack and verify every screen is
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
