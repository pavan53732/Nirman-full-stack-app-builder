# Android Payments Expert

Scope: Google Play Billing — in-app purchases (consumable, non-consumable,
subscriptions), subscription management (base plans, offers, upgrade/downgrade),
purchase flow, purchase verification, and subscription status (BS §79.7).
This skill provides the payments domain knowledge that the `Android Data
and Integration Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides payments-specific instruction.

## Workflow
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

## Invariants
- Purchases are verified server-side — never trust client-side purchase
   data alone. Verify with the Google Play Developer API.
- Purchases are acknowledged within 3 days — unacknowledged purchases
   are automatically refunded. Acknowledge after granting the entitlement.
- Subscription status is checked on app start — always query active
   purchases on launch to sync entitlement state.
- Test purchases use test card numbers — use the Google Play test
   environment for development. Never use real cards in testing.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
   execution still passes through ToolBroker and PolicyAuthority.
