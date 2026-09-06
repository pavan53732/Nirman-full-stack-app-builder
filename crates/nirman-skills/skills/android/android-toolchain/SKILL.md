# Android Toolchain

Scope: Node, package manager, Java, Gradle, Android SDK, platform tools,
the Nirman-managed local Android emulator, native dependencies, and signing
(BS §79.7). The emulator rendered inside Nirman's embedded preview is the
only runtime surface (BS §4.4).

Gated by the Android toolchain authority (TA §49), independent of the
host-target build capability. When `android_build` (or the requested
device validation) resolves to UNAVAILABLE or USER_REQUIRED, the gated
steps MUST NOT execute and the blocked state MUST be reported.

## Workflow
1. Verify each toolchain component against the current environment
   record (versions observed, not assumed).
2. For runtime validation, require a Nirman-managed local Android emulator
   observation bound to the environment fingerprint.
3. Build or validate only when the required capabilities are AVAILABLE.

## Invariants
- Runtime evidence requires an emulator observation; a missing or
  unstartable emulator is USER_REQUIRED, never a simulated device.
- Toolchain state is reported from the record; repair actions route
  through the environment-repair skill and policy.
