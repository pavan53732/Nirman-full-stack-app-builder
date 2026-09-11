# Android Toolchain

Scope: Node, package manager, Java, Gradle, Android SDK, platform tools,
the Nirman-managed local Android emulator, native dependencies, and signing
(BS §79.7). The emulator rendered inside Nirman's embedded preview is the
only runtime surface (BS §4.4).

## Trigger
This skill is requested when node, package manager, Java, Gradle, Android SDK, platform tools, the Nirman-managed local Android emulator, native dependencies, and signing (BS §79.7). The emulator rendered inside Nirman's embedded preview is the only runtime surface (BS §4.4).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

- Gated by the Android toolchain authority (TA §49), independent of the
host toolchain capability. When `ANDROID_BUILD_TOOLCHAIN` (or, for the
emulator steps, `ANDROID_EMULATOR_EXECUTION`) resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- java
- gradle
- sdkmanager
- adb
- emulator

## Procedure
1. Enumerate every component with an observed version: JDK, Gradle, the
   Android SDK and its installed platforms and build tools, platform
   tools, the NDK where native code is built, and Node with its package
   manager where the project uses them.
2. Resolve the SDK root explicitly from the environment record. A tool
   found only because of shell search order is a misconfiguration, not a
   working toolchain.
3. Verify SDK license acceptance state; an unaccepted license makes the
   component unusable and is reported, not clicked through.
4. Confirm the emulator package matches the declared target: the system
   image ABI and API level correspond to the target platform, an AVD
   exists, and acceleration is available through a usable hypervisor.
5. Verify signing material: a debug keystore for development builds, and
   for release only the configured signing identity, never a credential
   the skill creates or stores.
6. Lock the observation into the AndroidToolchainManifest — observed
   versions, resolved paths, and fingerprints — so every later build and
   evidence record binds to a single toolchain identity.
7. Build or validate only when the required capabilities are AVAILABLE,
   and bind the result to the manifest and the environment fingerprint.
8. Re-enumerate after any repair, because a repaired toolchain is a new
   identity and the previous manifest no longer describes it.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `AndroidToolchainManifest` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Runtime evidence requires an emulator observation bound to the environment fingerprint.
  *   A missing or unstartable emulator is USER_REQUIRED, never a simulated
  *   device and never a substitute runtime.
  * Versions are observed, never taken from the project's declaration; a
  *   declared version is a request, not a fact.
  * A version mismatch is reported as USER_REQUIRED or REPAIRABLE — the
  *   toolchain is never silently downgraded, upgraded, or substituted to
  *   make a check pass.
  * Emulator and system-image packages are provisioned from the SDK
  *   repository; Nirman MUST NOT bundle, fork, patch, rebuild, or
  *   redistribute the emulator engine (ADR-221).
  * On a host architecture the SDK repository does not serve an emulator
  *   for, the emulator capability is UNAVAILABLE with that stated reason;
  *   build, static analysis, and export continue, and validation evidence
  *   honestly remains absent (BS §79.17).
  * Key material is handled through the signing authority; this skill never
  *   holds, logs, or generates release signing credentials.
  * Toolchain repair routes through the environment-repair skill and
  *   policy; loading this skill grants no repair authority.
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
Emits `AndroidToolchainManifest` from `AndroidToolchainRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Enumerate every component with an observed version: JDK, Gradle, the
- Step 2 produces its expected outcome — Resolve the SDK root explicitly from the environment record. A tool
- Step 3 produces its expected outcome — Verify SDK license acceptance state; an unaccepted license makes the
- Step 4 produces its expected outcome — Confirm the emulator package matches the declared target: the system
- Step 5 produces its expected outcome — Verify signing material: a debug keystore for development builds, and
- Step 6 produces its expected outcome — Lock the observation into the AndroidToolchainManifest — observed
- Step 7 produces its expected outcome — Build or validate only when the required capabilities are AVAILABLE,
- Step 8 produces its expected outcome — Re-enumerate after any repair, because a repaired toolchain is a new
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
