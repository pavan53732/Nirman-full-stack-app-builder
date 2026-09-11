# Payment Integration

Scope: payment flows in Nirman apps — selecting a provider and confirming what it can do,
creating charges idempotently, verifying and de-duplicating webhooks, and reconciling
provider state against local records (BS §79.7).

## Trigger
A payment flow is designed or reviewed, or a charge is duplicated, a webhook is lost,
or provider and local records disagree.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The provider contract is available, so capability and webhook behaviour come from it
  rather than from observed traffic.

## Context requirements
- The provider, the currencies and methods in scope, and what the provider supports
  for each.
- The webhook surface and its signing scheme.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which system is authoritative for a charge's final state.

## Allowed tools
- static_analyzer
- webhook_probe

## Procedure
1. Confirm the provider actually supports every method and currency the app offers,
   from its contract, rather than assuming a method is available because it is common.
2. Verify charge creation is idempotent: the same intent cannot produce two charges,
   including when a request is retried after a timeout.
3. Keep no card data in the app or on the server where the provider can tokenise it, and
   confirm no such value reaches a log or a plain store.
4. Verify every webhook signature before its contents are trusted, and reject a
   signature that does not verify rather than processing the body anyway.
5. De-duplicate webhooks by event id, since delivery is at-least-once and the same event
   will arrive more than once.
6. Verify the webhook handler is order-tolerant: an out-of-order event does not move a
   charge backwards in its state machine.
7. Reconcile periodically against the provider's own record of charges, and report any
   disagreement rather than trusting the local table.

## Evidence
- Capability confirmation per method and currency, from the provider contract.
- Idempotency evidence for a retried charge request.
- Card-data handling review, including a scan of logs and stores.
- Signature verification results, including a forged-signature case.
- De-duplication results for a replayed event, and ordering results for an
  out-of-order event.
- Reconciliation outcome: local versus provider per charge, with disagreements named.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- DUPLICATE_CHARGE — one intent produced two charges.
- UNVERIFIED_WEBHOOK — a webhook was processed without a verified signature.
- UNSUPPORTED_METHOD — the app offers a method the provider does not support here.
- RECONCILE_MISMATCH — local and provider state disagree for a charge.

## Recovery
- A duplicate charge is refunded and the idempotency key fixed; the duplicate is never
  left for the user to dispute.
- An unverified webhook is rejected and the delivery is re-fetched rather than
  processed on trust because it came from the expected address.
- A reconciliation mismatch is resolved against the provider's record, and the local
  transition that caused it is corrected rather than patched in the table.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `PaymentResult` (§23 SkillPackage contract):

- capabilities: method and currency support from the provider contract
- idempotency: retried-charge outcome
- webhooks: signature verification, de-duplication, and ordering
- reconciliation: local versus provider per charge
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured payment evidence

## Fixtures
- Retried charge request produces one charge
- Forged webhook signature rejected
- Replayed event de-duplicated
- Out-of-order event does not move state backwards
- Reconciliation finds a local and provider disagreement
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
