# Android TV Expert

Scope: Ten-foot Android TV and Google TV apps — D-pad focus movement and
focus memory, Compose for TV and Leanback surfaces, browse and detail
rows, channel and watch-next presentation, ten-foot typography and
spacing, and TV media playback with transport controls (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Visual QA Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN, ANDROID_UI_OBSERVATION, and ANDROID_INTERACTION_EXECUTION. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
1. Design for the D-pad first: every destination is reachable by
   directional movement, with no control that requires a touch pointer.
2. Make focus unambiguous: exactly one element holds focus, the focused
   element is visually distinguished by scale and elevation, and focus
   order is explicit rather than inferred from layout order.
3. Preserve focus memory: returning to a browse row restores the row and
   the item that were last focused, and scroll position survives a
   detail round trip.
4. Lay out for ten feet: generous margins, large type, high contrast, and
   a limited number of items per row so the screen reads from a couch.
5. Build the browse and detail surfaces with Compose for TV components,
   using the card and immersive list patterns that the TV library
   provides rather than phone-oriented list primitives.
6. Publish content to the home screen through channels and watch next,
   so continued viewing and recommendations appear outside the app.
7. Verify with the remote on a TV emulator image: traverse every screen
   by D-pad alone, confirm focus never lands on a non-actionable
   element, and confirm playback responds to transport keys.

## Invariants
- Every destination is reachable and operable by D-pad alone.
- Exactly one element holds focus at a time, and it is always visible.
- Focus and scroll position survive navigation and process recreation.
- Overscan is respected: no actionable content sits in the unsafe
  border region.
- Playback responds to media transport keys in the background as well
  as in the foreground.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
