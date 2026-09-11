# Windows UI Expert

Scope: WinUI 3 and Windows App SDK desktop UI — XAML markup and data
binding, MVVM structure, Fluent design and theming, navigation and the
window lifecycle, accessibility and keyboard access, and WinUI
verification (BS §79.7). This skill provides the domain
knowledge that the `UI Worker` and `Visual QA Worker` consume.

Gated by WINDOWS_HOST_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked
state MUST be reported. This skill does not replace a worker role
— it provides domain-specific instruction that the worker executes
within its scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- The UI never blocks on a long-running operation; work runs off the UI
  thread and reports progress back to it.
- State lives in the view model and survives view recreation; a
  re-navigated view never resets silently.
- Every control is reachable and operable by keyboard, and focus is
  always visible.
- Theme, contrast, and text scaling follow the system; a hard-coded
  color, size, or contrast pair is a defect.
- An unverified view is reported as unverified — building is not
  verification, and a host build never stands in for native validation.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
