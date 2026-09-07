# Windows Desktop Build

Scope: C#/.NET / WinUI 3 / Windows App SDK / XAML host build plus Rust
control-plane integration for Windows x64 — Nirman.exe packaging,
NirmanSupervisor.exe packaging, the named-pipe SupervisorConnection, native
Windows runtime integration, and installer generation (BS §79.7, ADR-108,
ADR-117). The host stack is exactly the one those ADRs lock; no web-wrapper
desktop shell is part of this skill's scope (AGENTS.md §17).

Gated by the Windows host toolchain. When `WINDOWS_HOST_TOOLCHAIN`
resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
execute and the blocked state MUST be reported — the independent work
(source, static analysis, host-native tests, artifact inspection)
continues. The host is always Windows x64 (BS §79.1); there is no
cross-compilation lane.

## Workflow
1. Consume the current EnvironmentCapabilityRecord; verify
   `WINDOWS_HOST_TOOLCHAIN` is AVAILABLE.
2. Build Nirman.exe and NirmanSupervisor.exe natively, bundle, and
   generate the installer.
3. Emit build-gate evidence bound to the environment fingerprint.

## Invariants
- Never claims runtime validation: the output field
  `runtimeValidationClaimed` is fixed to false. A successful build is an
  artifact-production result, not a runtime-validation result
  (BS §79.5, §79.10); runtime validation belongs to
  windows-runtime-validation.
- No substitute execution target is introduced or implied.
- A failed build is reported with the diagnostic reference; it is never
  represented as success.
