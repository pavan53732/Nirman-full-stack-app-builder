# Windows UI Expert

Scope: WinUI 3 and Windows App SDK desktop UI — XAML markup and data
binding, MVVM structure, Fluent design and theming, navigation and the
window lifecycle, accessibility and keyboard access, and WinUI
verification (BS §79.7). This skill provides the domain
knowledge that the `UI Worker` and `Visual QA Worker` consume.

## Trigger
This skill is requested when winUI 3 and Windows App SDK desktop UI — XAML markup and data binding, MVVM structure, Fluent design and theming, navigation and the window lifecycle, accessibility and keyboard access, and WinUI verification (BS §79.7). This skill provides the domain knowledge that the `UI Worker` and `Visual QA Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`

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
- dotnet_sdk
- windows_app_sdk
- msbuild

## Procedure
1. Establish the window and navigation shell first: one window, a
   navigation surface, and a defined back behavior.
2. Structure each view as a view model behind a thin XAML view: the view
   binds, the view model holds state and commands, and neither reaches
   into the other's internals.
3. Bind data rather than assigning it in code-behind, and choose the
   binding mode deliberately — one-time for static content, one-way for
   display, and two-way only where the user edits in place.
4. Apply Fluent design: consistent spacing and typography, the platform
   accent and theme resources, and light, dark, and high-contrast
   appearance that follows the system setting.
5. Make the surface accessible: every control has an accessible name,
   focus order follows reading order, everything is operable by keyboard
   alone, and the layout respects the user's text-scaling setting.
6. Handle the window lifecycle: persist and restore window size and
   position, respond to a theme change at runtime, and keep state across
   suspend and resume.
7. Verify on the host: launch the built application, traverse every
   view, exercise keyboard-only navigation, and confirm appearance in
   light, dark, and high contrast.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `WindowsUiResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * The UI never blocks on a long-running operation; work runs off the UI
  *   thread and reports progress back to it.
  * State lives in the view model and survives view recreation; a
  *   re-navigated view never resets silently.
  * Every control is reachable and operable by keyboard, and focus is
  *   always visible.
  * Theme, contrast, and text scaling follow the system; a hard-coded
  *   color, size, or contrast pair is a defect.
  * An unverified view is reported as unverified — building is not
  *   verification, and a host build never stands in for native validation.
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

- CODE_BEHIND_BINDING — data is assigned in code-behind rather than bound, so it does not track its source.
- ACCESSIBLE_NAME_MISSING — a control has no accessible name, so it cannot be announced.
- WINDOW_STATE_LOST — window size and position are not restored, or a theme change is not honoured.

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
Emits `WindowsUiResult` from `WindowsUiResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Establish the window and navigation shell first: one window, a
- Step 2 produces its expected outcome — Structure each view as a view model behind a thin XAML view: the view
- Step 3 produces its expected outcome — Bind data rather than assigning it in code-behind, and choose the
- Step 4 produces its expected outcome — Apply Fluent design: consistent spacing and typography, the platform
- Step 5 produces its expected outcome — Make the surface accessible: every control has an accessible name,
- Step 6 produces its expected outcome — Handle the window lifecycle: persist and restore window size and
- Step 7 produces its expected outcome — Verify on the host: launch the built application, traverse every
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
