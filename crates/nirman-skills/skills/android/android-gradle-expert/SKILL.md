# Android Gradle Expert

Scope: Android Gradle build system — version catalogs (libs.versions.toml),
convention plugins, build variants (debug/release/staging), signing
config, ProGuard/R8 rules, dependency resolution, and build optimization
(BS §79.7). This skill provides the build-system domain knowledge that
the `Release Worker` and ToolchainAuthority consume.

## Trigger
This skill is requested when android Gradle build system — version catalogs (libs.versions.toml), convention plugins, build variants (debug/release/staging), signing config, ProGuard/R8 rules, dependency resolution, and build optimization (BS §79.7). This skill provides the build-system domain knowledge that the `Release Worker` and ToolchainAuthority consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_BUILD`
- `ANDROID_PACKAGING`

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
- gradle_build_system
- android_gradle_plugin
- proguard_r8

## Procedure
1. Analyze build requirements: identify build variants (debug, release,
   staging), dependency groups, and build optimization needs.
2. Set up version catalogs: define libs.versions.toml with versions,
   libraries, and bundles. Use libs.android.gradle.plugin syntax for
   plugins, libs.bundles.compose for grouped dependencies.
3. Create convention plugins: use `build-logic` module with
   convention.gradle.kts files for Android library, Android app,
   and Compose configuration. Avoid duplicating build logic across modules.
4. Configure build variants: define debug, release, and optional
   staging variants with different application IDs, signing configs,
   and build config fields.
5. Configure ProGuard/R8: define `proguard-rules.pro` for release builds,
   keep rules for reflection-based libraries, and test with
   `minifyEnabled = true` on debug for early detection.
6. Optimize build: enable build cache, configuration cache, parallel
   execution, and non-transitive R classes. Use Gradle build scans
   for bottleneck identification.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `BuildSystemConfigurationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Version catalogs are the single source of truth — dependencies are
  *    referenced via `libs.*`, never hardcoded as `group:artifact:version`.
  * Convention plugins are used for shared build logic — each module
  *    applies a convention plugin, not raw `android {}` blocks.
  * Signing config is separate from build logic — the `Release Worker`
  *    manages signing identity, not the Gradle script.
  * ProGuard/R8 rules are tested — release builds are tested on the
  *    emulator to catch reflection/serialization issues.
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
Emits `BuildSystemConfigurationResult` from `BuildSystemConfigurationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze build requirements: identify build variants (debug, release,
- Step 2 produces its expected outcome — Set up version catalogs: define libs.versions.toml with versions,
- Step 3 produces its expected outcome — Create convention plugins: use `build-logic` module with
- Step 4 produces its expected outcome — Configure build variants: define debug, release, and optional
- Step 5 produces its expected outcome — Configure ProGuard/R8: define `proguard-rules.pro` for release builds,
- Step 6 produces its expected outcome — Optimize build: enable build cache, configuration cache, parallel
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
