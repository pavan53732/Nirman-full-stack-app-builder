# Android Widgets Expert

Scope: Android app widgets — home screen widgets (AppWidgetProvider,
RemoteViews), widget layouts (ListView, GridView, StackView), widget
configuration activities, widget update strategies (AlarmManager,
WorkManager), and widget sizing (responsive layouts, categories)
(BS §79.7). This skill provides the widgets domain knowledge that the
`UI Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `UI Worker` role —
it provides widgets-specific instruction.

## Workflow
1. Analyze widget requirements: identify widget size (small, medium,
   large), update frequency, data source, and user interaction needs.
2. Define the widget provider: create AppWidgetProvider subclass,
   declare it in the manifest with android.appwidget.action.APPWIDGET_UPDATE
   intent filter, and define the widget metadata XML.
3. Design the widget layout: use RemoteViews with supported views
   (TextView, ImageView, Button, ListView, GridView). Define responsive
   layouts for different sizes using `layout-w<N>dp` qualifiers.
4. Implement the widget service: use RemoteViewsService for collection
   widgets (ListView, GridView), RemoteViewsFactory for data binding.
5. Handle widget updates: use AppWidgetManager.updateAppWidget to
   refresh the widget. Schedule periodic updates with WorkManager or
   AlarmManager.
6. Add widget configuration: use AppWidgetManager.ACTION_APPWIDGET_CONFIGURE
   intent for configuration activity. Store configuration per widget ID.
7. Test widget features: use the widget host emulator, test different
   sizes, test update behavior, and verify configuration flow.

## Invariants
- RemoteViews supports a limited view set — only TextView, ImageView,
   Button, ProgressBar, ListView, GridView, StackView, AdapterViewFlipper,
   FrameLayout, LinearLayout, RelativeLayout, and AnalogClock are supported.
- Widget updates are batched — the system throttles widget updates.
   Use WorkManager for reliable periodic updates.
- Widget configuration is per-widget — each widget instance has its own
   configuration. Store configuration with the widget ID as key.
- Widgets are not interactive — use PendingIntent for user interactions.
   Only one PendingIntent per view is supported.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
