# Android Install and Launch

Scope: installing a built artifact on the Nirman-managed local emulator and
launching it under observation — artifact and target compatibility,
install verification, cold and warm launch, first-frame capture, and the
classification of install and launch failures (BS §79.7).

## Trigger
A built artifact must be installed and launched before any runtime
behaviour can be observed. Nothing downstream — interaction, visual,
accessibility, or performance validation — is admissible without a verified
install and a verified launch.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_INSTALL_LAUNCH`

## Preconditions
- The artifact exists, is complete, and its signing configuration has been
  verified (ANDROID_SIGNING_INSPECTION, BS §79.7).
- A Nirman-managed emulator session is leased and `ANDROID_EMULATOR_EXECUTION`
  resolves to AVAILABLE.
- The target device ABI and API level match the artifact's declared support.
- The package identifier and launcher activity are known from the manifest,
  not guessed from naming conventions.

## Context requirements
- Artifact path, artifact kind (single APK, split APK set, or bundle), and
  its manifest digest.
- Package identifier, version code, and version name.
- Target device identity: AVD name, ABI, API level, and the environment
  fingerprint of the emulator session.
- The evidence identifiers this run must bind to, so install and launch
  results attach to the same evidence graph as the build.

## Allowed tools
- adb
- managed_emulator
- bundle_tool

## Procedure
1. Resolve compatibility before touching the device: compare the artifact's
   ABIs and minimum SDK against the target. A mismatch is reported, never
   installed optimistically.
2. Install through the platform installer: a single APK directly, a split set
   as one session, and never a partial set that leaves the package
   incomplete.
3. Verify the install, do not assume it: confirm the package is present,
   confirm the installed version code equals the artifact's, and capture the
   install session identifier.
4. Launch as a cold start from a stopped state unless the fixture explicitly
   asks for warm: resolve the launcher activity and start it under
   observation.
5. Wait for readiness by an observable condition — process present and first
   frame rendered — with a bound; an unbounded wait is a hang, not a launch.
6. Record launch timing from the launch command to first frame, and label the
   launch cold or warm, because the two are not comparable evidence.
7. If the process dies during launch, stop and classify: capture the logcat
   window and the crash tombstone before any retry.

## Evidence
- Install record: package identifier, version code, installer session
  identifier, install duration, and the target device identity.
- Launch record: process identifier, cold or warm classification, launch
  command to first-frame duration, and the readiness condition observed.
- A logcat window bound to the session, retained only for the launch
  interval, never as a standing capture.
- Every record bound to the environment fingerprint and to the artifact
  manifest digest, so the evidence cannot be replayed against a different
  build or device.

## Failure classification
- BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  nothing is attempted and the blocked state is reported.
- INCOMPATIBLE — ABI, minimum SDK, or screen density unsupported by the
  target; the artifact is rejected, not force-installed.
- INSTALL_FAILED — conflicting package, signature mismatch, insufficient
  storage, or a rejected install session; each is recorded with the
  platform reason.
- LAUNCH_FAILED — launcher activity unresolved, or the process died before
  first frame.
- TIMEOUT — the readiness condition was not observed within the bound.

## Recovery
- An incompatible target is resolved by choosing a matching target, never
  by relaxing the artifact's declared support.
- A conflicting package is removed only through the normal transaction path
  with policy admission; this skill never silently uninstalls.
- One retry is permitted after a repair that changed the environment or the
  artifact. An identical install is never re-run against unchanged evidence.
- A crash during launch is classified as an application defect and escalated
  to diagnostics; it is never masked by a retry that happens to succeed.
- A blocked capability resumes when the capability record changes; the
  blocked node names its resume condition.

## Output contract
Emits `InstallLaunchResult` (§23 SkillPackage contract):

- installed: boolean, with package identifier and version code
- launched: boolean, with process identifier and cold or warm label
- launchDurationMs: integer, measured to the observed readiness condition
- artifactDigest: the manifest digest the evidence is bound to
- deviceIdentity: AVD, ABI, API level, and environment fingerprint
- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the captured install and launch evidence

## Fixtures
- Clean install and cold launch on a freshly leased emulator
- Split APK set installed as one session
- ABI mismatch between artifact and target
- Conflicting package already installed
- Crash before first frame, with tombstone captured
- Cold launch compared against warm launch timing
- Capability UNAVAILABLE — blocked, nothing attempted

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT);
every execution still passes through ToolBroker and PolicyAuthority.
