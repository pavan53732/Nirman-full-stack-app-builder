# Android TV Expert

Scope: Ten-foot Android TV and Google TV apps — D-pad focus movement and
focus memory, Compose for TV and Leanback surfaces, browse and detail
rows, channel and watch-next presentation, ten-foot typography and
spacing, and TV media playback with transport controls (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Visual QA Worker` consume.

## Trigger
This skill is requested when ten-foot Android TV and Google TV apps — D-pad focus movement and focus memory, Compose for TV and Leanback surfaces, browse and detail rows, channel and watch-next presentation, ten-foot typography and spacing, and TV media playback with transport controls (BS §79.7). This skill provides the domain knowledge that the `UI Worker` and `Visual QA Worker` consume.. It does not replace a worker role — it supplies the domain
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
- compose_for_tv
- leanback
- managed_emulator

## Procedure
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

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `TvAppResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every destination is reachable and operable by D-pad alone.
  * Exactly one element holds focus at a time, and it is always visible.
  * Focus and scroll position survive navigation and process recreation.
  * Overscan is respected: no actionable content sits in the unsafe
  *   border region.
  * Playback responds to media transport keys in the background as well
  *   as in the foreground.
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

- DPAD_UNREACHABLE — a destination cannot be reached by directional movement alone.
- FOCUS_AMBIGUOUS — no element, or more than one element, holds focus at some point in traversal.
- FOCUS_MEMORY_LOST — returning to a browse row does not restore the row and item that were last focused.

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
Emits `TvAppResult` from `TvAppResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Design for the D-pad first: every destination is reachable by
- Step 2 produces its expected outcome — Make focus unambiguous: exactly one element holds focus, the focused
- Step 3 produces its expected outcome — Preserve focus memory: returning to a browse row restores the row and
- Step 4 produces its expected outcome — Lay out for ten feet: generous margins, large type, high contrast, and
- Step 5 produces its expected outcome — Build the browse and detail surfaces with Compose for TV components,
- Step 6 produces its expected outcome — Publish content to the home screen through channels and watch next,
- Step 7 produces its expected outcome — Verify with the remote on a TV emulator image: traverse every screen
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
