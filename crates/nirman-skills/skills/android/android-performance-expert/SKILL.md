# Android Performance Expert

Scope: Android performance optimization — Baseline Profiles, Macrobenchmark,
startup optimization, memory profiling (LeakCanary, Android Studio Profiler),
layout performance, RecyclerView/Compose lazy list optimization, and
network performance (BS §79.7). This skill provides the performance
domain knowledge that the `Performance Worker` consumes.

## Trigger
This skill is requested when android performance optimization — Baseline Profiles, Macrobenchmark, startup optimization, memory profiling (LeakCanary, Android Studio Profiler), layout performance, RecyclerView/Compose lazy list optimization, and network performance (BS §79.7). This skill provides the performance domain knowledge that the `Performance Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_PERFORMANCE_VALIDATION`

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
- baseline_profile_generator
- macrobenchmark
- leakcanary
- android_profiler

## Procedure
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

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `PerformanceOptimizationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Baseline Profiles are generated, not hand-written — use the
  *   `benchmark-macro` library to capture user journeys and generate
  *   profiles.
  * Memory leaks are failures — LeakCanary detections route through
  *   RecoveryAuthority for repair, not silent acceptance.
  * Frame budget is 16ms — any operation exceeding this causes jank.
  *   Profile with Systrace/Android Studio Profiler to identify offenders.
  * Network calls are paginated — never load unbounded datasets in a
  *   single request. Use Paging 3 for Compose integration.
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

- STARTUP_REGRESSION — startup time regressed against the recorded baseline for its cold, warm, or hot case.
- JANK_OVER_BUDGET — frame times exceed the 16ms budget over the measured scroll.
- UNATTRIBUTED_LEAK — a retained reference was found and no owning scope was named for it.

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
Emits `PerformanceOptimizationResult` from `PerformanceOptimizationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Establish performance baselines: measure startup time (cold, warm,
- Step 2 produces its expected outcome — Optimize startup: use Baseline Profiles to pre-compile hot paths,
- Step 3 produces its expected outcome — Optimize UI rendering: use Compose derivedStateOf to avoid
- Step 4 produces its expected outcome — Optimize memory: detect leaks with LeakCanary, use WeakReference
- Step 5 produces its expected outcome — Optimize network: use HTTP/2, connection pooling, response caching,
- Step 6 produces its expected outcome — Measure with Macrobenchmark: write MacrobenchmarkTest for startup,
- Step 7 produces its expected outcome — Monitor in production: use Firebase Performance Monitoring for
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
