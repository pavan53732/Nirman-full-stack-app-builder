# Android Toolchain

Scope: Node, package manager, Java, Gradle, Android SDK, platform tools,
emulator or physical device, native dependencies, and signing (BS §79.7).

Gated by the Android toolchain authority (TA §49), independent of the
host-target build capability. When `android_build` (or the requested
device validation) resolves to UNAVAILABLE or USER_REQUIRED, the gated
steps MUST NOT execute and the blocked state MUST be reported.

## Workflow
1. Verify each toolchain component against the current environment
   record (versions observed, not assumed).
2. For device validation, require an emulator or physical device
   observation bound to the environment fingerprint.
3. Build or validate only when the required capabilities are AVAILABLE.

## Invariants
- Device-specific evidence requires a device observation; a missing
  device is USER_REQUIRED, never a simulated device.
- Toolchain state is reported from the record; repair actions route
  through the environment-repair skill and policy.
