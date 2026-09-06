# Windows Desktop Build

Scope: C#/.NET / WinUI 3 / Windows App SDK / XAML + Rust control-plane
integration for Windows x64; Nirman.exe packaging, NirmanSupervisor.exe
packaging, named-pipe SupervisorConnection, native Windows runtime
integration, and MSIX installer generation (BS §79.7, §51.1; ADR-108,
ADR-111, ADR-117).

Gated by the cross-compilation capability (or a native Windows host).
When `cross_build_windows` or `windows_installer_generation` resolve to
UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT execute and the
blocked state MUST be reported — the independent work (source, static
analysis, host-native tests, artifact inspection) continues.

## Workflow
1. Consume the current EnvironmentCapabilityRecord; verify the required
   capabilities are AVAILABLE for this host→target pair.
2. Build the target artifacts (cross-build or native): the WinUI 3
   Nirman.exe host and the Rust NirmanSupervisor.exe; package both
   together and generate the MSIX installer when the installer
   capability is available (BS §51.2; ADR-111).
3. Emit build-gate evidence bound to the environment fingerprint.

## Invariants
- Never claims runtime validation: the output field
  `runtimeValidationClaimed` is fixed to false. A successful cross-build
  is an artifact-production result, not a runtime-validation result
  (BS §79.5, §79.10).
- No substitute execution target is introduced or implied.
- A failed build is reported with the diagnostic reference; it is never
  represented as success.
