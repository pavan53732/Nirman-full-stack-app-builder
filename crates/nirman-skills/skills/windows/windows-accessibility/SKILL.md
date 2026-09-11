# Windows Accessibility

Scope: accessibility of Nirman's desktop surface — keyboard reachability and focus order,
screen reader equivalents for every control and every status, contrast and visual
dependency, and support for high contrast and scaling (BS §79.7).

## Trigger
A surface is built or changed and must be reachable without a mouse, readable by a
screen reader, and usable under high contrast and scaling — or a user cannot complete
a task with assistive technology.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The surface under review is built and reachable on the host.

## Context requirements
- The surface under review and the controls it contains.
- The statuses it conveys, including those conveyed by colour or animation alone.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The display configurations to cover: scale factors and high contrast.

## Allowed tools
- accessibility_probe
- ui_inspector

## Procedure
1. Walk the surface with the keyboard alone and confirm every control is reachable,
   including those inside menus, dialogs, and scrollable regions.
2. Verify focus order follows reading order and that focus is visible at every stop.
3. Check for traps: a control that can be entered but not left by keyboard.
4. Verify every control has a name, role, and value that a screen reader can announce.
5. Verify every status has a non-visual equivalent — a build state, a progress value,
   or an error must be announced, not only coloured or animated.
6. Check contrast against the required ratio, and confirm no information is carried by
   colour alone.
7. Repeat under high contrast and at a larger scale factor, and confirm nothing is
   clipped, overlapped, or unreachable.

## Evidence
- Keyboard reachability per control, with any unreachable one named.
- Focus order observed, and focus visibility at each stop.
- Name, role, and value per control as a screen reader reads them.
- Non-visual equivalent per status, with any missing one named.
- Contrast measurements, and the results under high contrast and scaling.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CONTROL_UNREACHABLE — a control cannot be reached by keyboard.
- KEYBOARD_TRAP — focus can enter a control but not leave it.
- MISSING_EQUIVALENT — a status is conveyed only visually.
- CONTRAST_BELOW_THRESHOLD — measured contrast is under the required ratio.

## Recovery
- An unreachable control is fixed in the focus path, not by adding a shortcut that
  bypasses the ordering problem.
- A missing equivalent is added to the accessible tree, not by adding a tooltip that
  only appears on hover.
- A contrast failure is fixed in the colour values, not by enlarging the text until
  the measured ratio passes.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `AccessibilityResult` (§23 SkillPackage contract):

- keyboardReachability: per control, with unreachable ones named
- focusOrder: observed order and visibility
- screenReaderEquivalents: name, role, and value per control, and per status
- contrast: measured ratios, and results under high contrast and scaling
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured accessibility evidence

## Fixtures
- Every control reachable by keyboard
- Focus visible at every stop
- Screen reader announces every control and status
- Keyboard trap in a dialog
- Contrast measured below the required ratio

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
