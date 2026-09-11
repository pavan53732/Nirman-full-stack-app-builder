# Android Dynamic Delivery Expert

Scope: Android dynamic delivery — dynamic feature modules (on-demand
delivery, conditional delivery), Play Feature Delivery (install-time,
on-demand, conditional), Play Asset Delivery (install-time, fast-follow,
on-demand), and app bundles (BS §79.7). This skill provides the dynamic
delivery domain knowledge that the `Release Worker` consumes.

## Trigger
This skill is requested when android dynamic delivery — dynamic feature modules (on-demand delivery, conditional delivery), Play Feature Delivery (install-time, on-demand, conditional), Play Asset Delivery (install-time, fast-follow, on-demand), and app bundles (BS §79.7). This skill provides the dynamic delivery domain knowledge that the `Release Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_PACKAGING`
- `ANDROID_ARTIFACT_INSPECTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- split_install_manager
- asset_pack_manager
- dynamic_feature_plugin

## Procedure
1. Analyze delivery requirements: identify which features can be deferred
   (on-demand), which are needed immediately (install-time), and which
   are device-conditional.
2. Create dynamic feature modules: use `com.android.dynamic-feature`
   plugin, define `dist:module` metadata, and configure delivery options
   in the module manifest.
3. Configure Play Feature Delivery: use `<dist:module dist:title="...">`
   with `dist:on-demand` or `dist:instant` attributes. Define conditions
   (`dist:device-feature`, `dist:min-sdk`, `dist:user-countries`).
4. Configure Play Asset Delivery: use `<dist:install-time>`,
   `<dist:fast-follow>`, or `<dist:on-demand>` for asset packs. Define
   asset pack metadata in build.gradle.kts.
5. Request on-demand modules: use SplitInstallManager to request
   module installation, handle SplitInstallRequest, monitor
   SplitInstallSessionStatus, and handle errors.
6. Manage asset packs: use AssetPackManager to fetch asset packs,
   handle AssetPackStatus, and access downloaded assets with
   AssetPackLocation.
7. Test dynamic delivery: use internal app sharing for testing, test
   on-demand module requests, test asset pack delivery, and verify
   module uninstall behavior.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `DynamicDeliveryResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Dynamic feature modules are optional — the app MUST function without
  *   on-demand modules. Gracefully handle module unavailability.
  * On-demand modules require Play Store — dynamic delivery only works
  *   through the Play Store. Use internal app sharing for testing.
  * Asset packs have size limits — install-time asset packs are limited
  *   to 1 GB. Use fast-follow for larger assets.
  * Module requests are monitored — SplitInstallManager provides
  *   real-time status updates. Handle all status values (pending,
  *   downloading, installed, failed, canceled).
- Every claim reduced to an observable: what was seen, on which device or
  host, at which revision — never a statement of intent.

## Failure classification
- BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- PRECONDITION_UNMET — a precondition below was not satisfied; the skill
  does not proceed past it.
- ACTION_FAILED — a procedure step was attempted and did not produce its
  expected outcome.
- INVARIANT_VIOLATED — the work completed but one of the invariant claims
  this skill must leave observable does not hold.
- TIMEOUT — a wait exceeded its bound; an unbounded wait is a hang, not a
  slow step.

- MODULE_REQUEST_FAILED — an on-demand module was requested and the install did not complete, with the split-install state recorded.
- ASSET_PACK_UNAVAILABLE — an asset pack never reached a usable state, and no fallback content is defined.
- DELIVERY_TYPE_MISMATCH — content that must be present at install time was configured as on-demand or fast-follow.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `DynamicDeliveryResult` from `DynamicDeliveryRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze delivery requirements: identify which features can be deferred
- Step 2 produces its expected outcome — Create dynamic feature modules: use `com.android.dynamic-feature`
- Step 3 produces its expected outcome — Configure Play Feature Delivery: use `<dist:module dist:title="...">`
- Step 4 produces its expected outcome — Configure Play Asset Delivery: use `<dist:install-time>`,
- Step 5 produces its expected outcome — Request on-demand modules: use SplitInstallManager to request
- Step 6 produces its expected outcome — Manage asset packs: use AssetPackManager to fetch asset packs,
- Step 7 produces its expected outcome — Test dynamic delivery: use internal app sharing for testing, test
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
