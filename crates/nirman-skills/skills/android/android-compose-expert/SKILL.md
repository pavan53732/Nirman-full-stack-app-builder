# Android Compose Expert

Scope: Jetpack Compose UI building — recomposition-aware state management,
Modifier composition, Material3 theming, custom layouts, animations,
semantic testing annotations, and Compose-specific performance patterns
(BS §79.7; ADR-225 ScreenModel). This skill provides the Compose
domain knowledge that the `UI Worker` and `Visual QA Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `UI Worker` role — it
provides Compose-specific instruction that the worker executes within
its scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- State is hoisted, not duplicated — a single source of truth per piece
  of state. Avoid passing mutable state down the tree.
- Modifiers are order-sensitive — `padding().background()` differs from
  `background().padding()`. Document the intended visual effect.
- Side effects never run in composable scope directly — use effect handlers.
- Every interactive Composable has a testTag for E2E verification
  (CAP.ANDROID.E2E_VERIFY) and a contentDescription for accessibility.
- Recomposition is structural equality-based — use `key()` in lists and
  avoid unstable lambda captures.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
