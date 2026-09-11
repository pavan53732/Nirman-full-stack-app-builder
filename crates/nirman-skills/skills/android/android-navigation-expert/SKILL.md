# Android Navigation Expert

Scope: Android navigation — Navigation Component for Compose, type-safe
navigation with Serialization, deep links, nested navigation graphs,
back stack management, and multi-module navigation (BS §79.7). This
skill provides the navigation domain knowledge that the `UI Worker`
consumes.

## Trigger
This skill is requested when android navigation — Navigation Component for Compose, type-safe navigation with Serialization, deep links, nested navigation graphs, back stack management, and multi-module navigation (BS §79.7). This skill provides the navigation domain knowledge that the `UI Worker` consumes.. It does not replace a worker role — it supplies the domain
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
- navigation_compose
- kotlinx_serialization
- deep_link_handler

## Procedure
1. Analyze navigation requirements: identify screens, navigation paths,
   deep link targets, authentication-gated routes, and bottom navigation
   structure.
2. Design the navigation graph: define destinations (Composables), actions
   (transitions), arguments (path/query parameters), and nested graphs
   for feature modules.
3. Implement type-safe navigation: use the Kotlin serialization library for
   route definitions, NavType for argument serialization, and
   NavHostController for programmatic navigation.
4. Handle deep links: define deepLink patterns in the navigation graph,
   handle incoming intents in the Activity, and validate deep link
   arguments.
5. Manage the back stack: use popUpTo, launchSingleTop,
   restoreState for back stack control. Handle system back with
   BackHandler in Compose.
6. Implement multi-module navigation: use NavGraphBuilder.navigation
   for feature modules, `global actions` for cross-module navigation,
   and `deep links` for module entry points.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `NavigationDesignResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Navigation is declarative — the navigation graph defines all valid
  *   routes. Programmatic navigation uses the graph, not direct
  *   Activity launches.
  * Arguments are type-safe — use Serialization for route arguments,
  *   not string concatenation. Validate arguments at the destination.
  * Deep links are validated — incoming deep links are parsed and
  *   validated before navigation. Invalid deep links route to a fallback.
  * Back stack is predictable — popUpTo and launchSingleTop prevent
  *   duplicate destinations. Document the expected back behavior.
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
Emits `NavigationDesignResult` from `NavigationDesignRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze navigation requirements: identify screens, navigation paths,
- Step 2 produces its expected outcome — Design the navigation graph: define destinations (Composables), actions
- Step 3 produces its expected outcome — Implement type-safe navigation: use the Kotlin serialization library for
- Step 4 produces its expected outcome — Handle deep links: define deepLink patterns in the navigation graph,
- Step 5 produces its expected outcome — Manage the back stack: use popUpTo, launchSingleTop,
- Step 6 produces its expected outcome — Implement multi-module navigation: use NavGraphBuilder.navigation
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
