# Android Widgets Expert

Scope: Android app widgets — home screen widgets (AppWidgetProvider,
RemoteViews), widget layouts (ListView, GridView, StackView), widget
configuration activities, widget update strategies (AlarmManager,
WorkManager), and widget sizing (responsive layouts, categories)
(BS §79.7). This skill provides the widgets domain knowledge that the
`UI Worker` consumes.

## Trigger
This skill is requested when android app widgets — home screen widgets (AppWidgetProvider, RemoteViews), widget layouts (ListView, GridView, StackView), widget configuration activities, widget update strategies (AlarmManager, WorkManager), and widget sizing (responsive layouts, categories) (BS §79.7). This skill provides the widgets domain knowledge that the `UI Worker` consumes.. It does not replace a worker role — it supplies the domain
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
- app_widget_provider
- remote_views
- widget_service

## Procedure
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

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `WidgetsResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * RemoteViews supports a limited view set — only TextView, ImageView,
  *    Button, ProgressBar, ListView, GridView, StackView, AdapterViewFlipper,
  *    FrameLayout, LinearLayout, RelativeLayout, and AnalogClock are supported.
  * Widget updates are batched — the system throttles widget updates.
  *    Use WorkManager for reliable periodic updates.
  * Widget configuration is per-widget — each widget instance has its own
  *    configuration. Store configuration with the widget ID as key.
  * Widgets are not interactive — use PendingIntent for user interactions.
  *    Only one PendingIntent per view is supported.
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
Emits `WidgetsResult` from `WidgetsRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze widget requirements: identify widget size (small, medium,
- Step 2 produces its expected outcome — Define the widget provider: create AppWidgetProvider subclass,
- Step 3 produces its expected outcome — Design the widget layout: use RemoteViews with supported views
- Step 4 produces its expected outcome — Implement the widget service: use RemoteViewsService for collection
- Step 5 produces its expected outcome — Handle widget updates: use AppWidgetManager.updateAppWidget to
- Step 6 produces its expected outcome — Add widget configuration: use AppWidgetManager.ACTION_APPWIDGET_CONFIGURE
- Step 7 produces its expected outcome — Test widget features: use the widget host emulator, test different
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
