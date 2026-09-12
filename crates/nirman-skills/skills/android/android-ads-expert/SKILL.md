# Android Ads and Monetization Expert

Scope: Earning from an app with ads — the Google Mobile Ads SDK, banner,
interstitial, rewarded, and native formats, mediation and ad source
configuration, privacy and consent signalling, ad disclosure and
labeling, and test-ad verification before release (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `Release Worker` consume.

## Trigger
This skill is requested when earning from an app with ads — the Google Mobile Ads SDK, banner, interstitial, rewarded, and native formats, mediation and ad source configuration, privacy and consent signalling, ad disclosure and labeling, and test-ad verification before release (BS §79.7). This skill provides the domain knowledge that the `Android Data and Integration Worker` and `Release Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NETWORK_INTEGRATION`
- `ANDROID_EMULATOR_EXECUTION`

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
- google_mobile_ads
- ump_consent_sdk
- managed_emulator

## Procedure
1. Declare the app id in the manifest and initialize the SDK once, early,
   and off the critical startup path so a slow ad network cannot delay
   first frame.
2. Pick the format that fits the surface: banner for persistent low-value
   placement, interstitial at natural task boundaries only, rewarded
   where the user opts in for a defined benefit, and native where the ad
   must adopt the surrounding design.
3. Load ahead of the moment of display and handle the full lifecycle:
   onAdLoaded, onAdFailedToLoad, onAdImpression, and onAdClicked are each
   handled, and a failed load simply leaves the slot empty.
4. Never gate a functional outcome on an ad: a failed load, an
   unavailable network, or a user who declines consent leaves the app
   fully usable.
5. Wire consent and privacy: collect consent where required before
   personalized ads, honour the consent state in the ad request, and
   respect the advertising identifier when it is unavailable.
6. Label paid or sponsored content visibly, and keep ad controls clear of
   the app's own controls so a tap is never ambiguous.
7. Verify with test ad units on the Nirman-managed emulator: exercise
   load success, load failure, impression, click, and rewarded grant,
   and confirm the app behaves correctly when ads are unavailable.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `AdsIntegrationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * A failed or withheld ad never blocks, hides, or degrades app
  *   functionality.
  * Consent is collected before personalized ads and is honoured in every
  *   ad request; consent state is durable evidence.
  * Rewarded ads grant the reward only on a verified completion callback,
  *   never on impression alone.
  * Only test ad units are used in verification; live inventory is never
  *   exercised by an automated run.
  * Ads are labeled; a sponsored placement is never presented as organic
  *   content.
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

- AD_LOAD_FAILED — the SDK reported a failed load and the surface has no defined behaviour for it.
- CONSENT_UNRESOLVED — personalized ads were requested before consent was collected, or consent was ignored.
- FUNCTIONAL_OUTCOME_GATED_ON_AD — a functional outcome depends on an ad loading, being watched, or earning revenue.
- TEST_UNIT_IN_RELEASE — a test ad unit identifier is present in a release build.

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
Emits `AdsIntegrationResult` from `AdsIntegrationResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Declare the app id in the manifest and initialize the SDK once, early,
- Step 2 produces its expected outcome — Pick the format that fits the surface: banner for persistent low-value
- Step 3 produces its expected outcome — Load ahead of the moment of display and handle the full lifecycle:
- Step 4 produces its expected outcome — Never gate a functional outcome on an ad: a failed load, an
- Step 5 produces its expected outcome — Wire consent and privacy: collect consent where required before
- Step 6 produces its expected outcome — Label paid or sponsored content visibly, and keep ad controls clear of
- Step 7 produces its expected outcome — Verify with test ad units on the Nirman-managed emulator: exercise
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
