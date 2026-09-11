# Windows Update Recovery

Scope: recovering from a failed Nirman update — staging and validation before an update is
applied, rolling back to the last known-good version, preserving user data and
settings, and proving the rollback restores a working install (BS §79.7).

## Trigger
An update failed, or an update path must be proven: that a bad update can be rolled
back, that data survives, and that the rollback itself works.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- A known-good version exists and its package is retained.
- The user data location is known and is separate from the install location.

## Context requirements
- The version being updated from and the version being updated to.
- What data and settings must survive the update.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether the failure was mid-download, mid-install, or after first launch.

## Allowed tools
- installer_probe
- filesystem_probe

## Procedure
1. Establish the last known-good version and confirm its package is still available
   before anything is changed.
2. Classify where the update failed: download, staging, install, or first launch after
   install — each has a different recovery path.
3. Stage and validate the new package before applying it, so a package that cannot be
   validated is never applied.
4. Back up or confirm separation of user data and settings, so rollback never takes
   user data with it.
5. Roll back to the known-good version and then prove it: launch it and confirm it
   works, rather than assuming the rollback succeeded because it completed.
6. Verify data and settings survived both directions, and name anything that did not
   rather than reporting a general success.

## Evidence
- The known-good version identified and its package confirmed available.
- The failure stage, with the evidence that placed it there.
- Staging and validation results for the new package.
- Rollback result, with a post-rollback launch proving the install works.
- Data and settings comparison before and after, per item.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- NO_KNOWN_GOOD — no retained package to roll back to; escalated, never improvised.
- ROLLBACK_FAILED — the rollback did not restore a working install.
- DATA_LOST — user data or settings did not survive the update or rollback.
- VALIDATION_SKIPPED — the new package was applied without being validated.

## Recovery
- A rollback is proven by launching the restored version; a rollback that completes
  but does not launch is reported as failed.
- Lost data is restored from the retained copy where one exists, and the update path
  is fixed to separate data from install permanently.
- A skipped validation is corrected in the staging path, not by trusting the package
  because it came from the usual source.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `UpdateRecoveryResult` (§23 SkillPackage contract):

- knownGoodVersion: version and package availability
- failureStage: download, staging, install, or first launch
- rollbackResult: completed, and whether the restored version launched
- dataPreserved: per item, both directions
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured update evidence

## Fixtures
- Update applies and the new version launches
- Rollback restores known-good and launches
- Update fails at staging and is rolled back
- Data and settings survive both directions
- No known-good package retained — escalated
- Capability UNAVAILABLE — blocked, nothing recovered

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
