# Android Backend API Design

Scope: the HTTP surface a generated Android application consumes, when a supporting backend is required by the application — resource modelling and naming, versioning, error semantics, pagination, idempotency, and auth boundaries (BS §79.7).

## Trigger
An API is being designed, changed, or reviewed, or a client is failing against it
in a way that points at the contract rather than at the call site.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The current contract is available in machine-readable form, so the review compares
  against the contract rather than against the implementation.

## Context requirements
- The contract under review and the version in force.
- The consumers of the surface, and which of them cannot be changed freely.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether a breaking change is permitted in this change.

## Allowed tools
- static_analyzer
- contract_linter

## Procedure
1. Model the resources: name each one as a noun, keep the hierarchy shallow, and
   confirm no endpoint encodes a verb that the method already expresses.
2. Check every method against its semantics: safe methods have no side effects, and
   every non-safe method states what it changes.
3. Verify versioning: the version is explicit in the surface, and a change that breaks a
   consumer is released under a new version rather than changed in place.
4. Review error semantics: every failure returns a stable machine-readable code, a human
   message, and enough detail to act on — and no error leaks internals.
5. Verify pagination on every collection that can grow, with a stable ordering so pages
   do not repeat or skip records.
6. Verify idempotency on every non-safe method a client might retry, so a retry does not
   duplicate the effect.
7. Confirm the authentication boundary: every endpoint states whether it is public or
   authenticated, and no endpoint relies on the client to enforce access.
8. Record the breaking-change determination per change, with the consumer impact stated.

## Evidence
- Resource and method inventory, with any verb-encoded or non-semantic endpoint
  named.
- Versioning determination per change.
- Error contract per failure mode, with codes and messages.
- Pagination and ordering verification per collection.
- Idempotency determination per non-safe method.
- Authentication boundary per endpoint.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- BREAKING_CHANGE_UNDOCUMENTED — a change breaks a consumer without being marked
  breaking.
- NON_SEMANTIC_ENDPOINT — an endpoint encodes a verb or violates method semantics.
- UNSTABLE_ORDERING — a paginated collection can repeat or skip records.
- MISSING_IDEMPOTENCY — a retryable method duplicates its effect on retry.

## Recovery
- A breaking change is versioned rather than released in place; the version is never
  reused to avoid incrementing it.
- Unstable ordering is fixed with a deterministic sort key, not by increasing the page
  size to hide the overlap.
- A missing idempotency key is added to the method contract, not worked around by
  de-duplicating on the client.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `ApiDesignResult` (§23 SkillPackage contract):

- resourcesAndMethods: inventory with semantic findings
- versioning: per-change determination
- errorContract: codes and messages per failure mode
- paginationAndIdempotency: findings per endpoint
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured API evidence

## Fixtures
- Resource-oriented surface with correct method semantics
- Breaking change versioned and marked
- Paginated collection with a stable sort key
- Retryable method idempotent on retry
- Endpoint that encodes a verb
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
