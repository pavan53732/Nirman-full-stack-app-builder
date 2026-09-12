# Android App Shortcuts Expert

Scope: Launcher and assistant entry points — static, dynamic, and pinned
shortcuts through ShortcutManager, capability-based assistant entry, deep
link targets with back-stack correctness, shortcut limits and ranking, and
keeping shortcuts consistent with app state (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Android Data and Integration Worker` consume.

## Trigger
This skill is requested when launcher and assistant entry points — static, dynamic, and pinned shortcuts through ShortcutManager, capability-based assistant entry, deep link targets with back-stack correctness, shortcut limits and ranking, and keeping shortcuts consistent with app state (BS §79.7). This skill provides the domain knowledge that the `UI Worker` and `Android Data and Integration Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_SOURCE_ENGINEERING`
- `ANDROID_EMULATOR_EXECUTION`

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
- shortcut_manager
- app_actions
- managed_emulator

## Procedure
1. Choose the shortcut kind: static for entry points that never change,
   dynamic for context-dependent destinations, and pinned only where the
   user explicitly asks to keep one.
2. Give every shortcut a stable id, a short and long label, and an icon
   that is a recognizable adaptive icon rather than a cropped launcher
   glyph.
3. Bind each shortcut to a deep link into a real destination, and make
   that destination stand alone: it must be reachable directly, with the
   correct back stack synthesized, not only by in-app navigation.
4. Keep dynamic shortcuts current: update or remove them as the
   underlying data changes, so a shortcut never points at something that
   no longer exists.
5. Respect the published limits on how many dynamic and pinned shortcuts
   the launcher will show, and rank the most likely destinations first,
   because the surplus is dropped.
6. Declare capabilities for assistant entry where a voice or assistant
   invocation is a natural way to reach the same destination, and map
   them to the same deep-link targets.
7. Verify on the Nirman-managed emulator: long-press to reveal
   shortcuts, launch each one cold, confirm the destination and the back
   stack, and confirm a stale shortcut is removed or updated.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `ShortcutResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every shortcut launches a destination that stands alone with a
  *   correct synthesized back stack.
  * Shortcut ids are stable; a rebuilt id is a new shortcut and loses its
  *   rank and pin.
  * Dynamic shortcuts track app state; a shortcut pointing at deleted
  *   data is removed, never left dangling.
  * Published count limits are respected and the most likely destinations
  *   are ranked first.
  * Shortcuts are verified by cold launch, not merely declared.
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

- SHORTCUT_UNSTABLE_ID — a shortcut's identifier changes, so it cannot be updated or removed reliably.
- SHORTCUT_STALE — a dynamic shortcut points at data that no longer exists.
- SHORTCUT_LIMIT_EXCEEDED — more shortcuts are published than the launcher will show.
- SHORTCUT_DESTINATION_MISSING — a shortcut's deep link does not resolve to a real destination.

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
Emits `ShortcutResult` from `ShortcutResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Choose the shortcut kind: static for entry points that never change,
- Step 2 produces its expected outcome — Give every shortcut a stable id, a short and long label, and an icon
- Step 3 produces its expected outcome — Bind each shortcut to a deep link into a real destination, and make
- Step 4 produces its expected outcome — Keep dynamic shortcuts current: update or remove them as the
- Step 5 produces its expected outcome — Respect the published limits on how many dynamic and pinned shortcuts
- Step 6 produces its expected outcome — Declare capabilities for assistant entry where a voice or assistant
- Step 7 produces its expected outcome — Verify on the Nirman-managed emulator: long-press to reveal
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
