# Windows Installer Validation

Scope: validating the installer end to end — clean and in-place install paths, repair,
how cleanly uninstall removes the product, per-user install without elevation, and
upgrading across versions (BS §79.7).

## Trigger
A release package must be proven installable: on a clean host, over an existing
install, repaired, upgraded, and removed — with each path verified rather than assumed.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The package under test is built and signed.
- A clean host state is available, so a clean install is genuinely clean.

## Context requirements
- The package under test and its version.
- The install location and whether elevation is expected.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The prior version to upgrade from, where the upgrade path is in scope.

## Allowed tools
- installer_probe
- filesystem_probe

## Procedure
1. Install on a clean host and confirm the product launches and reports its version
   correctly, rather than stopping at install completion.
2. Confirm the install did not require elevation where a per-user install is declared,
   and that it wrote only to per-user locations.
3. Install in place over an existing install and confirm existing user data and
   settings survive.
4. Run repair and confirm it restores a working install from a damaged state, and that
   it does not discard user data.
5. Upgrade from the declared prior version and confirm settings carry forward and the
   old version is fully replaced.
6. Uninstall and then verify cleanliness: directories, files, registry entries, and
   shortcuts removed, and user data retained or removed as the contract states.
7. Verify the uninstaller leaves no orphaned processes or locked files that block
   reinstallation.

## Evidence
- Clean install outcome, with a launch and version check.
- Elevation requirement observed, and the locations written to.
- In-place and upgrade results, with settings and data comparison.
- Repair outcome from a deliberately damaged state.
- Post-uninstall residue: directories, entries, shortcuts, and user data, each listed.
- Reinstall feasibility after uninstall.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - A declared per-user install completes without requiring elevation.
  - An upgrade carries user settings forward.
  - Uninstall leaves no residue, and a subsequent reinstall succeeds.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- ELEVATION_REQUIRED — a declared per-user install demanded elevation.
- INSTALL_FAILED — installation did not complete or the product did not launch.
- UPGRADE_LOST_SETTINGS — user settings did not carry forward across an upgrade.
- UNINSTALL_RESIDUE — files, entries, or shortcuts remained after uninstall.

## Recovery
- Installer residue is fixed in the uninstaller, not by shipping a cleanup script the
  user must run.
- An elevation requirement is fixed by removing the machine-wide write, not by
  accepting elevation for a per-user install.
- Lost settings across an upgrade are fixed in the migration step, and the upgrade is
  re-proven from the declared prior version.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `InstallerValidationResult` (§23 SkillPackage contract):

- cleanInstall: completed, launched, version reported
- elevation: required or not, and locations written
- inPlaceAndUpgrade: settings and data comparison
- repair: outcome from a damaged state
- residue: directories, entries, shortcuts, and user data after uninstall
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured installer evidence

## Fixtures
- Clean install launches and reports its version
- Per-user install without elevation
- In-place install preserves settings
- Repair restores a damaged install
- Uninstall leaves no residue and reinstall works
- Capability UNAVAILABLE — blocked, nothing validated

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
