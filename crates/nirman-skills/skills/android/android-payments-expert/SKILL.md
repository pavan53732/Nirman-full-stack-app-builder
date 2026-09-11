# Android Payments Expert

Scope: Google Play Billing — in-app purchases (consumable, non-consumable,
subscriptions), subscription management (base plans, offers, upgrade/downgrade),
purchase flow, purchase verification, and subscription status (BS §79.7).
This skill provides the payments domain knowledge that the `Android Data
and Integration Worker` consumes.

## Trigger
This skill is requested when google Play Billing — in-app purchases (consumable, non-consumable, subscriptions), subscription management (base plans, offers, upgrade/downgrade), purchase flow, purchase verification, and subscription status (BS §79.7). This skill provides the payments domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NETWORK_INTEGRATION`

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
- billing_client
- google_play_billing
- purchase_verification

## Procedure
1. Analyze payment requirements: identify product types (consumable,
   non-consumable, subscription), pricing tiers, subscription periods,
   and upgrade/downgrade rules.
2. Configure Google Play Console: create products in the Play Console,
   define pricing, set up subscription base plans and offers, and
   configure tax rates.
3. Implement the billing client: use BillingClient with PurchasesUpdatedListener
   to handle purchase flows. Connect to the billing service with
   startConnection.
4. Implement the purchase flow: use launchBillingFlow to initiate
   purchases, handle BillingResponseCode, and acknowledge purchases
   with acknowledgePurchase.
5. Implement purchase verification: verify purchases on the backend using
   the Google Play Developer API, validate purchase tokens, and grant
   entitlements.
6. Handle subscription status: use queryPurchasesAsync to check active
   subscriptions, handle subscription lifecycle (active, canceled,
   in grace period, on hold, expired).
7. Test billing features: use Google Play's test environment with
   test card numbers, test subscription scenarios (cancel, resume,
   refund), and verify purchase acknowledgment.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `PaymentsIntegrationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Purchases are verified server-side — never trust client-side purchase
  *    data alone. Verify with the Google Play Developer API.
  * Purchases are acknowledged within 3 days — unacknowledged purchases
  *    are automatically refunded. Acknowledge after granting the entitlement.
  * Subscription status is checked on app start — always query active
  *    purchases on launch to sync entitlement state.
  * Test purchases use test card numbers — use the Google Play test
  *    environment for development. Never use real cards in testing.
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

- UNACKNOWLEDGED_PURCHASE — a purchase was not acknowledged within the window, so the platform refunds it.
- PURCHASE_UNVERIFIED — a purchase was granted without server-side verification.
- BILLING_UNAVAILABLE — the billing service is unavailable and purchase attempts were not blocked.
- SUBSCRIPTION_STATE_UNKNOWN — active subscription state could not be queried, and entitlement defaulted to granted.

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
Emits `PaymentsIntegrationResult` from `PaymentsIntegrationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze payment requirements: identify product types (consumable,
- Step 2 produces its expected outcome — Configure Google Play Console: create products in the Play Console,
- Step 3 produces its expected outcome — Implement the billing client: use BillingClient with PurchasesUpdatedListener
- Step 4 produces its expected outcome — Implement the purchase flow: use launchBillingFlow to initiate
- Step 5 produces its expected outcome — Implement purchase verification: verify purchases on the backend using
- Step 6 produces its expected outcome — Handle subscription status: use queryPurchasesAsync to check active
- Step 7 produces its expected outcome — Test billing features: use Google Play's test environment with
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
