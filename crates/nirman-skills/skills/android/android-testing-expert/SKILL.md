# Android Testing Expert

Scope: Android testing — JUnit 5, Compose UI Test, Espresso, Paparazzi
screenshot testing, Roborazzi, MockK, Turbine (Flow testing), and
test fixture management (BS §79.7). This skill provides the testing
domain knowledge that the `Test and QA Worker` and `Visual QA Worker`
consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Test and QA Worker`
role — it provides testing-specific instruction.

## Workflow
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

## Invariants
- Every Composable has a testTag — Compose UI tests locate nodes by
  tag, not by text or position. Tags are stable across recomposition.
- Screenshot tests have golden images — Paparazzi/Roborazzi compare
  against stored golden images. Changes require explicit golden update.
- Unit tests are deterministic — use TestDispatcher and runTest
  to eliminate timing flakiness. Never use Thread.sleep in tests.
- Integration tests use in-memory databases — never test against a
  production database. Room.inMemoryDatabaseBuilder provides isolation.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
