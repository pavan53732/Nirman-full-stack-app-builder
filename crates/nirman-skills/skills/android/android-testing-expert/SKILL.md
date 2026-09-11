# Android Testing Expert

Scope: Android testing — JUnit 5, Compose UI Test, Espresso, Paparazzi
screenshot testing, Roborazzi, MockK, Turbine (Flow testing), and
test fixture management (BS §79.7). This skill provides the testing
domain knowledge that the `Test and QA Worker` and `Visual QA Worker`
consume.

## Trigger
This skill is requested when android testing — JUnit 5, Compose UI Test, Espresso, Paparazzi screenshot testing, Roborazzi, MockK, Turbine (Flow testing), and test fixture management (BS §79.7). This skill provides the testing domain knowledge that the `Test and QA Worker` and `Visual QA Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
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
- junit5
- compose_ui_test
- paparazzi
- mockk

## Procedure
1. Analyze testing requirements: identify unit test targets (ViewModels,
   UseCases, Repositories), integration test targets (DAOs, API clients),
   and UI test targets (Composables, navigation flows).
2. Write unit tests: use JUnit 5 with runTest for coroutine tests,
   MockK for mocking, Turbine for Flow assertions. Test ViewModels
   by collecting state flows and asserting emitted values.
3. Write Compose UI tests: use createAndroidComposeRule for
   ComposeTestRule, onNodeWithTag for finding nodes (requires
   testTag), performClick, assertIsDisplayed. Test recomposition
   by performing actions and asserting state changes.
4. Write screenshot tests: use Paparazzi for pixel-perfect verification
   without a device, Roborazzi for screenshot comparison with tolerance
   thresholds. Store golden images in version control.
5. Write integration tests: use Room.inMemoryDatabaseBuilder for DAO
   tests, MockWebServer for API tests, AndroidJUnit4 for
   instrumented tests on the emulator.
6. Manage test fixtures: use TestFixture pattern for reusable test
   data, TestDispatcher for controlling coroutine execution in tests.
7. Run tests in CI: configure Gradle to run unit tests on every build,
   instrumented tests on the emulator, screenshot tests on every PR.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `TestingStrategyResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every Composable has a testTag — Compose UI tests locate nodes by
  *   tag, not by text or position. Tags are stable across recomposition.
  * Screenshot tests have golden images — Paparazzi/Roborazzi compare
  *   against stored golden images. Changes require explicit golden update.
  * Unit tests are deterministic — use TestDispatcher and runTest
  *   to eliminate timing flakiness. Never use Thread.sleep in tests.
  * Integration tests use in-memory databases — never test against a
  *   production database. Room.inMemoryDatabaseBuilder provides isolation.
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

- FLAKY_TEST — a test passes and fails on identical input, so its result cannot be cited as evidence.
- TEST_ISOLATION_BROKEN — a test depends on state another test leaves behind, or on the order of execution.
- COVERAGE_CLAIM_UNSUPPORTED — a coverage figure is reported without the run, the scope, or the excluded surfaces.

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
Emits `TestingStrategyResult` from `TestingStrategyRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze testing requirements: identify unit test targets (ViewModels,
- Step 2 produces its expected outcome — Write unit tests: use JUnit 5 with runTest for coroutine tests,
- Step 3 produces its expected outcome — Write Compose UI tests: use createAndroidComposeRule for
- Step 4 produces its expected outcome — Write screenshot tests: use Paparazzi for pixel-perfect verification
- Step 5 produces its expected outcome — Write integration tests: use Room.inMemoryDatabaseBuilder for DAO
- Step 6 produces its expected outcome — Manage test fixtures: use TestFixture pattern for reusable test
- Step 7 produces its expected outcome — Run tests in CI: configure Gradle to run unit tests on every build,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
