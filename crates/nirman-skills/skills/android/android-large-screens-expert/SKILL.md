# Android Large Screens and Foldables Expert

Scope: Adaptive UI for large screens — WindowSizeClass-driven layout switching,
foldable postures and hinge handling, tablet two-pane and list-detail
compositions, activity embedding, multi-window and multi-resume behavior,
drag and drop, and the large-screen quality gates of the Play store (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Visual QA Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN, ANDROID_UI_OBSERVATION, and ANDROID_INTERACTION_EXECUTION. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
1. Read the device posture and window metrics from the WindowManager
   APIs: current WindowSizeClass (COMPACT, MEDIUM, EXPANDED), the window
   width and height DPs, and for foldables the FoldingFeature state
   (FLAT or HALF_OPENED), its orientation, and its occlusion type.
2. Choose the canonical layout for the size class: one pane with bottom
   navigation in COMPACT, one pane with a navigation rail in MEDIUM, and a
   two-pane list-detail with a navigation drawer in EXPANDED.
3. Hoist the size class into composition as a single derived value and
   branch on it. Never measure the screen directly to pick a layout —
   the size class is the contract.
4. For foldables, treat HALF_OPENED as a distinct state: avoid placing
   interactive controls across a hinge whose occlusion type is FULL, and
   reflow table-like content into separated panes when the fold is vertical.
5. Support multi-window and multi-resume: the app may be visible but not
   focused, so pause camera, video, and location on loss of focus rather
   than on onPause of the visible lifecycle alone.
6. Implement drag and drop between panes and from outside the app where
   the layout invites it, using the platform drag framework with a
   meaningful clip description and a visible drop target.
7. Verify at every size class on the Nirman-managed local emulator:
   resize the window, fold and unfold the device, rotate it, and assert
   that state is preserved and no content is occluded.

## Invariants
- State survives configuration change — rotation, resizing, and folding
  must never reset a screen or lose scroll position.
- Every layout branch is reachable and verified; an unverified size
  class is reported as unverified, never assumed to mirror another.
- No interactive control is placed across a FULL occlusion hinge.
- Media and sensor resources follow focus, not mere visibility.
- The app does not lock orientation or aspect ratio to escape the
  large-screen gates (BS §79.7 large-screen quality).
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
