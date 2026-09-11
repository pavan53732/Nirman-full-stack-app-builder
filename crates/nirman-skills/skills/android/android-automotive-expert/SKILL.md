# Android Automotive Expert

Scope: Building for the car — the Car App Library, the fixed set of
automotive templates, driver-distraction and step constraints, navigation
and parked-mode surfaces, media and messaging templates, and the
automotive quality gates that a car-hosted app must satisfy (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Android Data and Integration Worker` consume.

## Trigger
This skill is requested when building for the car — the Car App Library, the fixed set of automotive templates, driver-distraction and step constraints, navigation and parked-mode surfaces, media and messaging templates, and the automotive quality gates that a car-hosted app must satisfy (BS §79.7). This skill provides the domain knowledge that the `UI Worker` and `Android Data and Integration Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_UI_OBSERVATION`

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
- car_app_library
- managed_emulator

## Procedure
1. Choose the app category the platform permits for the car: navigation,
   parked apps (point of interest, charging, parking), or media. A
   category outside these is not distributable to a car host.
2. Model the UI as a stack of templates, not as free-form screens: pick
   from the list, grid, message, pane, and navigation templates and
   respect the item and action limits of each.
3. Honour the distraction constraints: the number of items that may be
   shown while driving, the step depth allowed in a driving task, and the
   requirement that a task be completable within the permitted steps.
4. Gate parked-only content on the driving state; a parked app's richer
   surfaces appear only when the car is stationary.
5. For navigation apps, provide the navigation template with live
   routing, lane guidance, and turn-by-turn updates through the
   navigation manager rather than a bespoke renderer.
6. Implement media browsing and playback through the media template, so
   the car host can control playback from its own hardware controls.
7. Verify on an automotive emulator image: exercise every template, the
   parked and driving states, and the day and night color constraints of
   the car host.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `AutomotiveAppResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Only platform templates are rendered; a bespoke automotive screen is
  *   rejected by the car host and is a defect.
  * Item counts, action counts, and step depth stay within the
  *   distraction limits for the current driving state.
  * Parked-only surfaces are unreachable while the vehicle is in motion.
  * Text contrast and touch target sizing meet the automotive night and
  *   day requirements in both color modes.
  * An unverified driving state or template is reported as unverified,
  *   never assumed from the phone form factor.
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

- UNSUPPORTED_APP_CATEGORY — the app requests a category the automotive platform does not permit.
- DISTRACTION_LIMIT_EXCEEDED — more items are shown while driving than the distraction constraints allow.
- PARKED_CONTENT_WHILE_DRIVING — content gated on the parked state is reachable while driving.
- TEMPLATE_NOT_AVAILABLE — a required template is absent from the host, and no fallback surface is defined.

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
Emits `AutomotiveAppResult` from `AutomotiveAppResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Choose the app category the platform permits for the car: navigation,
- Step 2 produces its expected outcome — Model the UI as a stack of templates, not as free-form screens: pick
- Step 3 produces its expected outcome — Honour the distraction constraints: the number of items that may be
- Step 4 produces its expected outcome — Gate parked-only content on the driving state; a parked app's richer
- Step 5 produces its expected outcome — For navigation apps, provide the navigation template with live
- Step 6 produces its expected outcome — Implement media browsing and playback through the media template, so
- Step 7 produces its expected outcome — Verify on an automotive emulator image: exercise every template, the
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
