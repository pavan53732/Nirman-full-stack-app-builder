# Windows Desktop Build

Scope: Tauri 2 / React / TypeScript / Vite / Rust build for Windows x64,
bundling, and installer generation (BS §79.7).

Gated by the cross-compilation capability (or a native Windows host).
When `cross_build_windows` or `windows_installer_generation` resolve to
UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT execute and the
blocked state MUST be reported — the independent work (source, static
analysis, host-native tests, artifact inspection) continues.

## Workflow
1. Consume the current EnvironmentCapabilityRecord; verify the required
   capabilities are AVAILABLE for this host→target pair.
2. Build the target artifact (cross-build or native), bundle, and
   generate the installer when the installer capability is available.
3. Emit build-gate evidence bound to the environment fingerprint.

## Invariants
- Never claims runtime validation: the output field
  `runtimeValidationClaimed` is fixed to false. A successful cross-build
  is an artifact-production result, not a runtime-validation result
  (BS §79.5, §79.10).
- No substitute execution target is introduced or implied.
- A failed build is reported with the diagnostic reference; it is never
  represented as success.
