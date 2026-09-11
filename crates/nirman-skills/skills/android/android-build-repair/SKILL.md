# Android Build Repair

Scope: diagnosing and repairing a failing Android build — Gradle and plugin errors,
dependency and version-catalog resolution, resource and manifest merge conflicts,
shrinker configuration, and incremental build corruption (BS §79.7).

## Trigger
A build fails, or succeeds locally and fails elsewhere. The failure must be
classified and repaired at its cause rather than worked around.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_BUILD`
- `ANDROID_SOURCE_ENGINEERING`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `ANDROID_BUILD` and `ANDROID_SOURCE_ENGINEERING` resolve to AVAILABLE.
- The failing command, its working directory, and its environment are
  recorded as observed, not recalled.
- The build toolchain manifest identifies the exact tool versions in use.

## Context requirements
- The failing task, the failure output, and the command that produced it.
- The toolchain versions from the manifest, and the dependency graph.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the session, so every record binds to them.
- Whether the same command succeeded before, and what changed since.

## Allowed tools
- gradle
- dependency_inspector
- static_analyzer

## Procedure
1. Read the failure from the build output and identify the failing task before
   changing anything; the first error is the cause and the rest are fallout.
2. Classify the failure by layer: configuration, dependency resolution,
   compilation, resource or manifest merge, shrinker, or packaging.
3. For resolution failures, read the conflict the resolver reports and choose a
   resolution deliberately rather than forcing a version to silence it.
4. For merge conflicts, resolve at the owning source and record which manifest
   or resource won and why.
5. For shrinker failures, decide whether a keep rule is genuinely required or
   whether the code should stop depending on reflection; a blanket keep rule is
   a workaround that disables the shrinker.
6. For a build that succeeds locally and fails elsewhere, compare the toolchain
   manifest and the environment record rather than the source.
7. Suspect incremental corruption only after a clean build is compared; then
   repair the cache rather than deleting it on every failure.

## Evidence
- The failing task and the failure output it produced, verbatim.
- The layer the failure was classified into, and the evidence for that
  classification.
- The resolution chosen for a conflict, and the alternative rejected.
- A clean-build comparison where incremental corruption was suspected.
- Every record bound to revision and to the toolchain manifest.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - The failing task and its verbatim output are captured before any repair is attempted.
  - A repair changes one thing, and the build is re-run to prove that change had the effect claimed.
  - A clean build succeeding where incremental failed is recorded as incremental corruption, not as a fix.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CONFIGURATION_ERROR — the build script or plugin configuration is invalid.
- RESOLUTION_CONFLICT — dependencies could not be resolved to one version.
- MERGE_CONFLICT — manifest or resource merge failed between sources.
- SHRINKER_ERROR — shrinking removed something required, or failed outright.
- UNCLASSIFIED — the output does not establish a cause.

## Recovery
- A version force is a last resort, recorded with the conflict it silences;
  the conflict is resolved properly rather than kept suppressed.
- A blanket keep rule is replaced with a scoped one, or the reflection that
  needed it is removed.
- Incremental corruption is repaired at the cache after a clean comparison
  proves it; the cache is not deleted reflexively.
- An unclassified failure is escalated with the verbatim output rather than
  resolved by trial and error.
- One retry is permitted after a materially changed input; an identical
  build is never re-run against unchanged evidence.

## Output contract
Emits `BuildRepairResult` (§23 SkillPackage contract):

- failingTask: the Gradle task that failed
- failureLayer: configuration, resolution, merge, shrinker, or packaging
- resolutionApplied: what was changed, and the alternative rejected
- cleanBuildCompared: boolean, where corruption was suspected
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured build evidence

## Fixtures
- Configuration error caught at the failing task
- Dependency conflict resolved deliberately
- Manifest merge conflict resolved at the owning source
- Shrinker removing a required symbol
- Incremental corruption proven by clean comparison
- Capability UNAVAILABLE — blocked, nothing built

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
