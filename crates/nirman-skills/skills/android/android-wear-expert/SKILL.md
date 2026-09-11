# Android Wear OS Expert

Scope: Wear OS development — Wear OS UI (Compose for Wear, BoxInsetLayout,
CurvedLayout, SwipeDismissFrameLayout), watch faces (CanvasWatchFaceService),
complications (data providers for watch faces), tiles (quick actions),
and health services (Heart Rate, Step Count, Location) (BS §79.7).
This skill provides the Wear OS domain knowledge that the `UI Worker`
consumes.

## Trigger
This skill is requested when wear OS development — Wear OS UI (Compose for Wear, BoxInsetLayout, CurvedLayout, SwipeDismissFrameLayout), watch faces (CanvasWatchFaceService), complications (data providers for watch faces), tiles (quick actions), and health services (Heart Rate, Step Count, Location) (BS §79.7). This skill provides the Wear OS domain knowledge that the `UI Worker` consumes.. It does not replace a worker role — it supplies the domain
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
- wear_os_sdk
- compose_for_wear
- health_services

## Procedure
1. Analyze Wear OS requirements: identify the app type (standalone,
   companion), screen size (round, square), and health data needs.
2. Set up the Wear OS module: create a Wear OS module in the project,
   add the Wear OS dependencies, and configure the manifest with the
   Wear OS feature declaration.
3. Design the Wear OS UI: use Compose for Wear with Scaffold,
   TimeText, ScalingLazyColumn, SwipeToDismissBox. Handle round
   screen insets with BoxInsetLayout.
4. Implement complications: use ComplicationProviderService to provide
   data to watch faces. Define complication types (short text, long text,
   small image, ranged value) and update on data change.
5. Implement tiles: use TileService to provide quick-access information.
   Define tile layout with TileLayout, handle tile requests with
   onTileRequest.
6. Access health services: use HealthServicesClient for heart rate,
   step count, location. Request health permissions, handle sensor
   availability.
7. Test Wear OS features: use the Wear OS emulator (round, square),
   test complications on watch faces, test tiles, and verify health
   sensor access.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `WearOSResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Wear OS apps are standalone — they run independently on the watch.
  *   Companion apps are optional, not required.
  * Round screen insets are mandatory — use BoxInsetLayout or Compose
  *   contentPadding to avoid content being cut off on round screens.
  * Complications have data limits — complication data is limited in size.
  *   Keep data concise and update only when necessary.
  * Tiles are limited to one per app — each app can have only one tile.
  *   Design the tile to show the most relevant information.
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
Emits `WearOSResult` from `WearOSRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze Wear OS requirements: identify the app type (standalone,
- Step 2 produces its expected outcome — Set up the Wear OS module: create a Wear OS module in the project,
- Step 3 produces its expected outcome — Design the Wear OS UI: use Compose for Wear with Scaffold,
- Step 4 produces its expected outcome — Implement complications: use ComplicationProviderService to provide
- Step 5 produces its expected outcome — Implement tiles: use TileService to provide quick-access information.
- Step 6 produces its expected outcome — Access health services: use HealthServicesClient for heart rate,
- Step 7 produces its expected outcome — Test Wear OS features: use the Wear OS emulator (round, square),
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
