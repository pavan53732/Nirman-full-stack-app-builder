# Android Compose Expert

Scope: Jetpack Compose UI building — recomposition-aware state management,
Modifier composition, Material3 theming, custom layouts, animations,
semantic testing annotations, and Compose-specific performance patterns
(BS §79.7; ADR-225 ScreenModel). This skill provides the Compose
domain knowledge that the `UI Worker` and `Visual QA Worker` consume.

## Trigger
This skill is requested when jetpack Compose UI building — recomposition-aware state management, Modifier composition, Material3 theming, custom layouts, animations, semantic testing annotations, and Compose-specific performance patterns (BS §79.7; ADR-225 ScreenModel). This skill provides the Compose domain knowledge that the `UI Worker` and `Visual QA Worker` consume.. It does not replace a worker role — it supplies the domain
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
- compose_preview
- material3_library

## Procedure
1. Analyze the design intent and map to Compose UI structure: identify
   screens, navigation destinations, reusable components, and state
   requirements.
2. Design state hoisting: determine what state lives at what level
   (remember, mutableStateOf, StateFlow, ViewModel), following unidirectional
   data flow. Avoid lifting state higher than its consumers.
3. Build the Composable tree: use Box, Row, Column, LazyColumn,
   LazyRow, ConstraintLayout as appropriate. Compose modifiers in the
   correct order — modifier order affects behavior.
4. Apply Material3 theming: use MaterialTheme.colorScheme,
   MaterialTheme.typography, MaterialTheme.spacing. Support dynamic
   color on Android 12+ with graceful fallback.
5. Handle side effects correctly: use LaunchedEffect, DisposableEffect,
   produceState, derivedStateOf, snapshotFlow — never launch
   coroutines directly in composable scope.
6. Add semantics for accessibility and testing: Modifier.semantics,
   Modifier.testTag, contentDescription. Every interactive element
   MUST have a semantic action and a test tag.
7. Validate in Preview: use `@Preview` composables and the live Preview
   surface to verify rendering before integration.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `ComposeBuildResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * State is hoisted, not duplicated — a single source of truth per piece
  *   of state. Avoid passing mutable state down the tree.
  * Modifiers are order-sensitive — `padding().background()` differs from
  *   `background().padding()`. Document the intended visual effect.
  * Side effects never run in composable scope directly — use effect handlers.
  * Every interactive Composable has a testTag for E2E verification
  *   (CAP.ANDROID.E2E_VERIFY) and a contentDescription for accessibility.
  * Recomposition is structural equality-based — use `key()` in lists and
  *   avoid unstable lambda captures.
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
Emits `ComposeBuildResult` from `ComposeBuildRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze the design intent and map to Compose UI structure: identify
- Step 2 produces its expected outcome — Design state hoisting: determine what state lives at what level
- Step 3 produces its expected outcome — Build the Composable tree: use Box, Row, Column, LazyColumn,
- Step 4 produces its expected outcome — Apply Material3 theming: use MaterialTheme.colorScheme,
- Step 5 produces its expected outcome — Handle side effects correctly: use LaunchedEffect, DisposableEffect,
- Step 6 produces its expected outcome — Add semantics for accessibility and testing: Modifier.semantics,
- Step 7 produces its expected outcome — Validate in Preview: use `@Preview` composables and the live Preview
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
