# Android Wear OS Expert

Scope: Wear OS development — Wear OS UI (Compose for Wear, BoxInsetLayout,
CurvedLayout, SwipeDismissFrameLayout), watch faces (CanvasWatchFaceService),
complications (data providers for watch faces), tiles (quick actions),
and health services (Heart Rate, Step Count, Location) (BS §79.7).
This skill provides the Wear OS domain knowledge that the `UI Worker`
consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `UI Worker` role —
it provides Wear OS-specific instruction.

## Workflow
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

## Invariants
- Wear OS apps are standalone — they run independently on the watch.
  Companion apps are optional, not required.
- Round screen insets are mandatory — use BoxInsetLayout or Compose
  contentPadding to avoid content being cut off on round screens.
- Complications have data limits — complication data is limited in size.
  Keep data concise and update only when necessary.
- Tiles are limited to one per app — each app can have only one tile.
  Design the tile to show the most relevant information.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
