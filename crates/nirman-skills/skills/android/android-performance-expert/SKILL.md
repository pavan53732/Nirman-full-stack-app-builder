# Android Performance Expert

Scope: Android performance optimization — Baseline Profiles, Macrobenchmark,
startup optimization, memory profiling (LeakCanary, Android Studio Profiler),
layout performance, RecyclerView/Compose lazy list optimization, and
network performance (BS §79.7). This skill provides the performance
domain knowledge that the `Performance Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Performance Worker`
role — it provides performance-specific instruction.

## Workflow
1. Establish performance baselines: measure startup time (cold, warm,
   hot), frame rendering (16ms budget), memory footprint, and network
   latency before optimization.
2. Optimize startup: use Baseline Profiles to pre-compile hot paths,
   lazy-initialize dependencies with `App Startup` library, reduce
   Application.onCreate work, and defer non-critical initialization.
3. Optimize UI rendering: use Compose derivedStateOf to avoid
   unnecessary recomposition, `key()` in lazy lists, remember for
   expensive calculations, and avoid layout nesting depth.
4. Optimize memory: detect leaks with LeakCanary, use WeakReference
   for listeners, avoid static references to Activities/Contexts, and
   profile allocations with Android Studio Profiler.
5. Optimize network: use HTTP/2, connection pooling, response caching,
   and pagination for large datasets. Use Coil for image loading with
   memory and disk caching.
6. Measure with Macrobenchmark: write MacrobenchmarkTest for startup,
   scroll jank, and frame timing. Run on real devices for accurate
   results.
7. Monitor in production: use Firebase Performance Monitoring for
   real-user metrics, custom traces for critical user journeys.

## Invariants
- Baseline Profiles are generated, not hand-written — use the
  `benchmark-macro` library to capture user journeys and generate
  profiles.
- Memory leaks are failures — LeakCanary detections route through
  RecoveryAuthority for repair, not silent acceptance.
- Frame budget is 16ms — any operation exceeding this causes jank.
  Profile with Systrace/Android Studio Profiler to identify offenders.
- Network calls are paginated — never load unbounded datasets in a
  single request. Use Paging 3 for Compose integration.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
