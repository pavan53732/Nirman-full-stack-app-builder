# Android Accessibility Expert

Scope: Android accessibility — TalkBack support, content descriptions,
touch target sizing (48dp minimum), color contrast ratios (4.5:1 for
text), accessibility scanner, semantic roles, and accessibility test
automation (BS §79.7). This skill provides the accessibility domain
knowledge that the `Visual QA Worker` and `UI Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the blocked state MUST be reported. This skill does not
replace the `Visual QA Worker` role — it provides accessibility-specific
instruction.

## Workflow
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

## Invariants
- Every non-text interactive element has a content description —
  contentDescription is required for accessibility. Empty descriptions
  are only for decorative elements.
- Touch targets are 48dp minimum — smaller targets are flagged by the
  Accessibility Scanner and MUST be enlarged.
- Color is not the sole indicator — information conveyed with color
  (errors, selection) has an additional visual indicator (icon, text).
- Dynamic content changes are announced — use `Modifier.semantics {
  liveRegion = LiveRegionMode.Polite }` for status updates.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
   execution still passes through ToolBroker and PolicyAuthority.
