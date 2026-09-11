# Android Large Screens and Foldables Expert

Scope: Adaptive UI for large screens — WindowSizeClass-driven layout switching,
foldable postures and hinge handling, tablet two-pane and list-detail
compositions, activity embedding, multi-window and multi-resume behavior,
drag and drop, and the large-screen quality gates of the Play store (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Visual QA Worker` consume.

## Trigger
This skill is requested when adaptive UI for large screens — WindowSizeClass-driven layout switching, foldable postures and hinge handling, tablet two-pane and list-detail compositions, activity embedding, multi-window and multi-resume behavior, drag and drop, and the large-screen quality gates of the Play store (BS §79.7). This skill provides the domain knowledge that the `UI Worker` and `Visual QA Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_UI_OBSERVATION`
- `ANDROID_INTERACTION_EXECUTION`

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
- compose_compiler
- window_manager
- managed_emulator

## Procedure
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

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `LargeScreenLayoutResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * State survives configuration change — rotation, resizing, and folding
  *   must never reset a screen or lose scroll position.
  * Every layout branch is reachable and verified; an unverified size
  *   class is reported as unverified, never assumed to mirror another.
  * No interactive control is placed across a FULL occlusion hinge.
  * Media and sensor resources follow focus, not mere visibility.
  * The app does not lock orientation or aspect ratio to escape the
  *   large-screen gates (BS §79.7 large-screen quality).
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

- SIZE_CLASS_IGNORED — the layout does not branch on the window size class, so it stretches instead of reflowing.
- HINGE_CONTROL_PLACED — an interactive control spans the fold hinge while the device is half-opened.
- MULTI_WINDOW_UNHANDLED — the app assumes focus while merely visible, so it does not pause what must pause.

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
Emits `LargeScreenLayoutResult` from `LargeScreenLayoutResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Read the device posture and window metrics from the WindowManager
- Step 2 produces its expected outcome — Choose the canonical layout for the size class: one pane with bottom
- Step 3 produces its expected outcome — Hoist the size class into composition as a single derived value and
- Step 4 produces its expected outcome — For foldables, treat HALF_OPENED as a distinct state: avoid placing
- Step 5 produces its expected outcome — Support multi-window and multi-resume: the app may be visible but not
- Step 6 produces its expected outcome — Implement drag and drop between panes and from outside the app where
- Step 7 produces its expected outcome — Verify at every size class on the Nirman-managed local emulator:
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
