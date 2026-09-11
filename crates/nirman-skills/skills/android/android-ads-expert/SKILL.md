# Android Ads and Monetization Expert

Scope: Earning from an app with ads — the Google Mobile Ads SDK, banner,
interstitial, rewarded, and native formats, mediation and ad source
configuration, privacy and consent signalling, ad disclosure and
labeling, and test-ad verification before release (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `Release Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN and ANDROID_NETWORK_INTEGRATION. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- A failed or withheld ad never blocks, hides, or degrades app
  functionality.
- Consent is collected before personalized ads and is honoured in every
  ad request; consent state is durable evidence.
- Rewarded ads grant the reward only on a verified completion callback,
  never on impression alone.
- Only test ad units are used in verification; live inventory is never
  exercised by an automated run.
- Ads are labeled; a sponsored placement is never presented as organic
  content.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
