# Android Navigation Expert

Scope: Android navigation — Navigation Component for Compose, type-safe
navigation with Serialization, deep links, nested navigation graphs,
back stack management, and multi-module navigation (BS §79.7). This
skill provides the navigation domain knowledge that the `UI Worker`
consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `UI Worker` role —
it provides navigation-specific instruction.

## Workflow
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

## Invariants
- Navigation is declarative — the navigation graph defines all valid
  routes. Programmatic navigation uses the graph, not direct
  Activity launches.
- Arguments are type-safe — use Serialization for route arguments,
  not string concatenation. Validate arguments at the destination.
- Deep links are validated — incoming deep links are parsed and
  validated before navigation. Invalid deep links route to a fallback.
- Back stack is predictable — popUpTo and launchSingleTop prevent
  duplicate destinations. Document the expected back behavior.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
