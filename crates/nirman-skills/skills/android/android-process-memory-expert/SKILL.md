# Android Process and Memory Expert

Scope: process and memory health on the Nirman-managed emulator — heap growth and
leak detection, out-of-memory classification, process importance and the
low-memory killer, background execution limits, and memory regression measurement (BS §79.7).

## Trigger
Memory must be measured rather than assumed: a suspected leak, an
out-of-memory failure to be classified, a process that dies unexpectedly, or a
memory budget a screen must stay within.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_PERFORMANCE_VALIDATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- A session is leased and `ANDROID_EMULATOR_EXECUTION` resolves to AVAILABLE.
- `ANDROID_PERFORMANCE_VALIDATION` resolves to AVAILABLE for measurement.
- The app is installed and launched, and the scenario to measure is repeatable.

## Context requirements
- The scenario to measure and how many cycles it runs for.
- The memory budget or threshold in scope, and where it comes from.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the session, so every record binds to them.
- The process model of the app: how many processes it runs and why.

## Allowed tools
- managed_emulator
- memory_profiler
- adb

## Procedure
1. Establish a baseline: measure heap after a settled idle state, not at
   startup, because startup allocation is not a leak.
2. Run the scenario for enough cycles that a real leak separates from noise,
   forcing collection at cycle boundaries so garbage is not counted as growth.
3. Measure retained heap across cycles and classify growth as a leak, as a
   cache that is behaving within its bound, or as noise.
4. Where growth is real, attribute it to a retaining path rather than to a
   component by reputation; name the reference chain that holds it.
5. Classify any out-of-memory: allocation failure, a large single allocation,
   or exhaustion under a low-memory condition.
6. Check process importance and survival: what the platform kills first under
   memory pressure, and whether the app survives the transition it must.
7. Confirm background behaviour obeys the platform's execution limits rather
   than assuming background work continues indefinitely.

## Evidence
- Baseline and per-cycle heap measurements, with the cycle count and the
  collection points.
- The growth classification, with the retaining path where growth is real.
- Out-of-memory classification where one occurred, with the allocation that
  failed and the memory state at the time.
- Process importance observed under memory pressure, and what survived.
- Every record bound to revision, artifact digest, and device identity.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- LEAK_CONFIRMED — retained heap grows across cycles with a named retaining
  path.
- OOM_UNCLASSIFIED — an out-of-memory occurred whose cause the evidence does
  not establish.
- BACKGROUND_KILLED — the process was killed under memory pressure in a
  transition it was required to survive.
- MEASUREMENT_INVALID — the measurement was taken without a settled baseline
  or without collection at cycle boundaries.

## Recovery
- An invalid measurement is retaken with a settled baseline and collection at
  cycle boundaries; a number taken without them is discarded, not averaged in.
- A confirmed leak is fixed at the retaining reference, and re-measured over
  the same cycle count to confirm the growth is gone.
- An unclassified out-of-memory is escalated to runtime diagnostics with the
  allocation evidence rather than guessed at.
- One retry is permitted after a materially changed input; an identical
  measurement is never re-run against unchanged evidence.

## Output contract
Emits `MemoryHealthResult` (§23 SkillPackage contract):

- baselineHeapKb and perCycleHeapKb: the measured series
- growthClass: LEAK, CACHE_WITHIN_BOUND, or NOISE
- retainingPath: the reference chain, where growth is real
- oomClass: allocation failure, large allocation, or pressure exhaustion
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured memory evidence

## Fixtures
- Stable heap across repeated scenario cycles
- Confirmed leak with a retaining path named
- Cache growth within its declared bound
- Out-of-memory under memory pressure
- Process killed in a transition it must survive
- Capability UNAVAILABLE — blocked, nothing measured

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
