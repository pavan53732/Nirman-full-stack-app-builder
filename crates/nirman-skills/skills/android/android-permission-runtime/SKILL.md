# Android Runtime Permissions

Scope: the runtime permission model — declaring a permission versus requesting it,
permission groups, one-time and partial grants, rationale and the don't-ask-again
path, special permissions outside the normal dialog, and verifying the permission
state a flow actually produced (BS §79.7).

## Trigger
A feature needs a dangerous or special permission, or a flow must be proven to
behave correctly when the permission is denied, revoked, or granted only for this
one use.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_UI_OBSERVATION`
- `ANDROID_INTERACTION_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- A session is leased and `ANDROID_EMULATOR_EXECUTION` resolves to AVAILABLE.
- The application is installed and launched, with install and launch evidence.
- The permission set in scope is known from the manifest, not inferred from
  the code path.

## Context requirements
- The permissions in scope and which are dangerous, special, or normal.
- The feature gated by each permission and its expected behaviour when denied.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the session, so every record binds to them.
- Whether the run starts from a clean grant state or from an already-granted one.

## Allowed tools
- adb
- managed_emulator
- ui_hierarchy_probe

## Procedure
1. Read the declared permissions from the merged manifest and separate normal,
   dangerous, and special permissions, because only the last two need a runtime
   request.
2. Start from a known grant state: reset permissions so the run is not
   observing a grant left by an earlier one.
3. Trigger the feature and let the application request; do not grant by
   instrumenting around the application's own request path.
4. Exercise the grant path and record the grant scope the platform reports,
   including one-time and only-while-in-use variants.
5. Exercise the denial path, including don't-ask-again, and verify the feature
   degrades instead of failing opaque or crashing.
6. Exercise revocation from settings and mid-use, and verify the application
   re-requests rather than assuming a grant it no longer holds.
7. Verify the resulting permission state through the platform, not through the
   application's own claim about it.

## Evidence
- The declared permission set, separated by protection level.
- The observed grant state after each path: granted scope, denied, or
  don't-ask-again, read from the platform.
- The behaviour of the gated feature under each state, including the denial
  path, with the UI hierarchy observed at each step.
- Every record bound to revision, package identifier, and session.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- REQUEST_NOT_RAISED — the feature was used without the application requesting
  the permission it needs.
- CRASH_ON_DENIAL — the feature fails instead of degrading when denied.
- STALE_GRANT_ASSUMED — the application proceeds on a grant that was revoked.
- UNVERIFIABLE_STATE — the platform probe could not read the grant state.

## Recovery
- A stale grant is cleared and the flow re-run from a known state rather than
  reasoned about from the previous run.
- A crash on denial is escalated as an application defect; the skill does not
  pre-grant the permission to make the flow pass.
- Revocation is re-tested after any re-request, because a re-request that
  silently succeeds without a dialog is a defect.
- One retry is permitted after a materially changed input; an identical flow
  is never re-run against unchanged evidence.

## Output contract
Emits `PermissionFlowResult` (§23 SkillPackage contract):

- declaredPermissions: identifier and protection level for each
- observedGrants: permission, grant scope, and how it was read
- pathsExercised: grant, denial, don't-ask-again, revocation
- degradationVerified: boolean, behaviour when denied
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured permission evidence

## Fixtures
- Grant path with a one-time grant recorded
- Denial path degrading gracefully
- Don't-ask-again path
- Revocation from settings, then re-request
- Special permission routed to its settings surface
- Capability UNAVAILABLE — blocked, nothing exercised

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
