# Android Automotive Expert

Scope: Building for the car — the Car App Library, the fixed set of
automotive templates, driver-distraction and step constraints, navigation
and parked-mode surfaces, media and messaging templates, and the
automotive quality gates that a car-hosted app must satisfy (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Android Data and Integration Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN and ANDROID_UI_OBSERVATION. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- Only platform templates are rendered; a bespoke automotive screen is
  rejected by the car host and is a defect.
- Item counts, action counts, and step depth stay within the
  distraction limits for the current driving state.
- Parked-only surfaces are unreachable while the vehicle is in motion.
- Text contrast and touch target sizing meet the automotive night and
  day requirements in both color modes.
- An unverified driving state or template is reported as unverified,
  never assumed from the phone form factor.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
