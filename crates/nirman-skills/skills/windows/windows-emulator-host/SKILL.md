# Windows Emulator Host

Scope: the host side of the Nirman-managed Android emulator — hypervisor availability and
acceleration, the host architecture constraint, provisioning and lifecycle, the
resource budget the host can spare, and the surface the preview renders into (BS §79.7).

## Trigger
The emulator must be brought up, or it fails to start, runs unaccelerated, or
starves the host — or the host must be assessed before a device-dependent run.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The host architecture and the hypervisor platforms available on it are known from
  observation.

## Context requirements
- Host CPU architecture and the hypervisor platforms present.
- The emulator package and system image the configuration selects.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The resource budget the host can spare while the emulator runs.

## Allowed tools
- hypervisor_probe
- emulator_probe
- resource_probe

## Procedure
1. Determine the host architecture and whether the SDK repository publishes an
   emulator for it; where it does not, the emulator capability is UNAVAILABLE with
   that stated reason and is never substituted.
2. Probe hypervisor platforms and confirm acceleration is actually usable, not merely
   installed.
3. Confirm the selected system image matches the host architecture and the declared
   target ABI; a mismatched image is never provisioned.
4. Provision from the SDK repository and verify integrity; the emulator engine is never
   bundled, forked, patched, or rebuilt.
5. Start the emulator and verify it reaches a usable state within a bound, recording
   boot completion rather than assuming it.
6. Measure the resource the emulator actually consumes against the host budget,
   including under a concurrent build.
7. Verify the preview surface receives frames and that a lost stream is reported as a
   gap rather than shown as a frozen live view.

## Evidence
- Host architecture and hypervisor availability, each from observation.
- Image-to-host match verification and the provisioning integrity result.
- Boot outcome with the completion signal observed and the time taken.
- Resource measurements against the declared host budget.
- Preview stream state, including any gap and how it was reported.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- HOST_UNSUPPORTED — no emulator is published for this host architecture; reported
  with that reason and never substituted.
- NO_ACCELERATION — the hypervisor is absent or unusable; the emulator is not started
  unaccelerated without saying so.
- IMAGE_MISMATCH — the selected image does not match the host architecture.
- BOOT_TIMEOUT — the emulator did not reach a usable state within the bound.

## Recovery
- An unsupported host is reported with its reason and build, static analysis, and
  export continue under the split rule; no substitute runtime is presented.
- Missing acceleration is repaired at the hypervisor configuration, and until then the
  run is blocked rather than run slowly and silently.
- A boot timeout is escalated with the last observed boot signal rather than retried
  indefinitely.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `EmulatorHostResult` (§23 SkillPackage contract):

- hostArchitecture and hypervisor availability
- imageMatch: selected image versus host architecture
- bootOutcome: completion signal observed and time taken
- resourceUsage: measured against the host budget
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured host evidence

## Fixtures
- Emulator boots accelerated on a supported host
- ARM64 host reported UNAVAILABLE with its reason
- Hypervisor absent — blocked, not started unaccelerated
- Mismatched system image refused
- Boot timeout with the last signal recorded
- Preview stream gap reported as a gap

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
