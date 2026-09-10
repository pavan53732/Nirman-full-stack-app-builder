# Android Background Expert

Scope: Android background execution — WorkManager (deferred, expedited,
periodic, chained work), foreground services (media, location, data sync),
the Android alarm service for exact alarms, Doze/App Standby awareness, broadcast
receivers, and background execution limits (API 30+) (BS §79.7). This
skill provides the background-execution domain knowledge that the
`Android Data and Integration Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides background-specific instruction.

## Workflow
1. Analyze background requirements: identify the work type (deferred,
   immediate, periodic, long-running), constraints (network, battery,
   storage), and whether the user needs to perceive the work.
2. Select the right tool: WorkManager for deferrable, guaranteed
   execution; foreground services for immediate, user-visible work;
   the Android alarm service only for exact-time alarms (clocks, reminders).
3. Define WorkManager constraints: setRequiredNetworkType,
   setRequiresBatteryNotLow, setRequiresStorageNotLow,
   setRequiresDeviceIdle. Use setExpedited for immediate work
   (subject to quota).
4. Implement the Worker class: extend CoroutineWorker for suspend
   support, Worker for synchronous work. Return `Result.success()`,
   `Result.retry()`, or `Result.failure()`.
5. Handle foreground services: declare foregroundServiceType in the
   manifest (media, location, dataSync, etc.), show a persistent
   notification, and handle the API 34 foreground service permissions.
6. Test background work: use WorkManagerTestInitHelper for unit tests,
   TestDriver for constraint satisfaction testing.

## Invariants
- WorkManager is the default for background work — it handles Doze,
   App Standbox, and OEM-specific battery optimizations automatically.
- Foreground services require a persistent notification — the user must
   be aware of ongoing background work. Never start a foreground service
   without a notification.
- Exact alarms require SCHEDULE_EXACT_ALARM permission — use
   setExactAndAllowWhileIdle sparingly, as it bypasses Doze.
- Background execution limits (API 30+) restrict background starts —
   use foreground services or WorkManager for reliable execution.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
