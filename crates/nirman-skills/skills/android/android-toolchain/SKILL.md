# Android Toolchain

Scope: Node, package manager, Java, Gradle, Android SDK, platform tools,
the Nirman-managed local Android emulator, native dependencies, and signing
(BS §79.7). The emulator rendered inside Nirman's embedded preview is the
only runtime surface (BS §4.4).

Gated by the Android toolchain authority (TA §49), independent of the
host toolchain capability. When ANDROID_BUILD_TOOLCHAIN — or, for the
emulator steps only, ANDROID_EMULATOR_EXECUTION — resolves to UNAVAILABLE
or USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported.

## Workflow
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

## Invariants
- Runtime evidence requires an emulator observation bound to the environment fingerprint.
  A missing or unstartable emulator is USER_REQUIRED, never a simulated
  device and never a substitute runtime.
- Versions are observed, never taken from the project's declaration; a
  declared version is a request, not a fact.
- A version mismatch is reported as USER_REQUIRED or REPAIRABLE — the
  toolchain is never silently downgraded, upgraded, or substituted to
  make a check pass.
- Emulator and system-image packages are provisioned from the SDK
  repository; Nirman MUST NOT bundle, fork, patch, rebuild, or
  redistribute the emulator engine (ADR-221).
- On a host architecture the SDK repository does not serve an emulator
  for, the emulator capability is UNAVAILABLE with that stated reason;
  build, static analysis, and export continue, and validation evidence
  honestly remains absent (BS §79.17).
- Key material is handled through the signing authority; this skill never
  holds, logs, or generates release signing credentials.
- Toolchain repair routes through the environment-repair skill and
  policy; loading this skill grants no repair authority.
