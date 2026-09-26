# Android Backend Service Engineering

Scope: implementation of the supporting backend/service layer a generated Android
application consumes — REST/GraphQL handlers, server-side business logic, server
schema and migrations, authentication and authorization, webhooks, background jobs,
backend integration tests, and the deployment configuration for the user's own backend
(BS §79.7). It does not restate HTTP surface design, which `android-backend-api-design`
owns, nor the generated application's client and adapter behavior normalization
(BS:6597), nor reachability and functional state, which `AndroidServiceIntegration`
owns (BS §5.7.5).

## Trigger
A declared `IntegrationSpec` is being implemented, or a supporting service is being
changed, extended, migrated, or given its deployment configuration (BS §79.7). It does
not replace a worker role — it supplies the domain instruction the worker executes
inside its scoped asset transaction (BS §50).

## Required capabilities
- `HOST_TOOL_OBSERVATION`
- `ANDROID_SOURCE_ENGINEERING`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale; capability is
  read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to UNAVAILABLE
  or USER_REQUIRED, the gated steps MUST NOT execute and the blocked state MUST be
  reported.
- The source revision and, where one exists, the artifact digest are known, so every
  record this skill emits can be bound to them.
- The declared `IntegrationSpec` is available in machine-readable form. Without it this
  skill does not proceed, because implementing against an assumed contract produces a
  surface no consumer declared.

## Context requirements
- The declared `IntegrationSpec` and its version in force, including the
  `requestSchemaRef`, `responseSchemaRef`, `errorSchemaRef`, `authState`,
  `credentialReference`, `endpointIdentity`, `offlinePolicy`, `retryPolicy`,
  `timeoutPolicy`, `idempotencyPolicy`, `privacyPolicy`, and `networkPolicy` it
  declares.
- The `functionalScenarioIds` the integration declares, and which of them this change
  can and cannot exercise.
- Revision, artifact digest where one exists, and the environment fingerprint, so every
  record binds to them.
- Whether the backend belongs to the user or is a local supporting component of the
  generated project.

## Allowed tools
- service_handler_codegen
- server_schema_migration
- auth_middleware_scaffold
- backend_integration_test

## Procedure
1. Read the declared `IntegrationSpec` first and implement against it. Where the code
   and the declaration disagree, the disagreement is reported as a contract defect; it
   is not resolved by editing the code to match an assumption.
2. Implement each declared method so its semantics hold on the server: a safe method
   has no side effects, and every non-safe method is idempotent under the declared
   `idempotencyPolicy` so a retry does not duplicate its effect.
3. Validate at the boundary. Every field of `requestSchemaRef` is checked server-side
   before any effect, and a rejection returns the declared `errorSchemaRef` code rather
   than an internal error or a stack trace.
4. Enforce authentication and authorization on the server for every declared endpoint,
   using the declared `authState` and credential reference. Access control that depends
   on the client is not access control.
5. Implement the server-side schema and its migrations with a stated recovery path for
   each step. A destructive step states what recovers it before it is proposed, and no
   migration is applied without the ability to reverse or to reconstruct.
6. Verify inbound webhooks by signature before any effect, and reject a replayed
   delivery. Make every background job handler idempotent and its retry bounded, so a
   redelivered or retried job does not duplicate its effect.
7. Keep the generated application's typed client and adapter consuming this surface.
   Retry, timeout, offline, idempotency, and error normalization belong to that adapter;
   do not duplicate them into server-side logic to compensate for a client gap.
8. State the backend integration test identity and what it proves, and record which
   declared `functionalScenarioIds` this change exercises and which it leaves
   unexercised.
9. Emit deployment configuration as configuration for a target this skill names
   explicitly, for a backend the user owns or a local supporting component the user
   owns. Nirman does not stand up, operate, or depend on a Nirman-operated hosted
   service.

## Evidence
- A record of each procedure step that executed, with the outcome observed and the step
  that produced it, bound to the source revision and the environment fingerprint; a
  step that ran and recorded nothing is not evidence that it succeeded.
- The emitted `BackendServiceImplementationResult` (§23 SkillPackage contract), carrying
  its classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a statement the
  evidence above has to support:
  - The supporting service is the user's backend or a local supporting component the
    user owns. It is never a Nirman product target, and no Nirman capability, account,
    subscription, license fee, or hosted-platform dependency is introduced for Nirman
    itself (BS §1.5; AGENTS.md §2).
  - The implemented surface conforms to the declared `IntegrationSpec`, or the
    divergence is reported. A silent divergence is a contract defect, not a
    convention.
  - A service is reported working only from a declared functional scenario observed
    against it. Code that compiles, a handler that exists, and a schema that migrated
    are implementation facts, not operationality; the state comparison belongs to
    `AndroidServiceIntegration` (BS §5.7.5).
  - A credential is referenced through the declared credential reference and is never
    read, logged, embedded, or echoed by this skill; credential custody stays
    supervisor-owned.
  - The generated application's API client and adapter remain separate from Nirman IPC
    and never write the Nirman ledger (TA §81.5).
  - HTTP surface design is owned by `android-backend-api-design`; this skill consumes
    that determination and does not restate or override it.
  - A declared functional scenario this change did not exercise is reported
    unexercised, never implied by the ones that passed.

## Failure classification
- BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED; the gated
  steps MUST NOT execute and the blocked state MUST be reported.
- PRECONDITION_UNMET — a precondition was not satisfied; the skill does not proceed past
  it.
- ACTION_FAILED — a procedure step was attempted and did not produce its expected
  outcome.
- INVARIANT_VIOLATED — the work completed but one of the invariant claims this skill
  must leave observable does not hold.
- TIMEOUT — a wait exceeded its bound; an unbounded wait is a hang, not a slow step.
- SPEC_DIVERGENCE — the implemented surface contradicts the declared `IntegrationSpec`.
- SERVER_VALIDATION_MISSING — a declared request field is accepted without a
  server-side check.
- AUTHORIZATION_CLIENT_SIDE — access control for a declared endpoint depends on the
  client.
- WEBHOOK_REPLAY_ACCEPTED — an inbound webhook is processed without signature
  verification and replay rejection.
- JOB_NOT_IDEMPOTENT — a retried or redelivered job duplicates its effect.
- MIGRATION_IRREVERSIBLE — a server schema migration is proposed without a stated
  recovery path.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked node
  names its resume condition and is never reported as a failure of the goal.
- One retry is permitted after a repair that materially changed the input. An identical
  action is never re-run against unchanged evidence, and the recoveryAttemptPolicy
  bound escalates rather than terminating the goal.
- A divergence is repaired at the contract, by changing the declaration or the
  implementation deliberately, never by loosening the consumer to accept both.
- An invariant violation is repaired at its cause. A service whose state cannot be
  established is reported as an unestablished state with the missing observation named,
  not as a pass and not as a failure of the whole goal.

## Output contract
Emits `BackendServiceImplementationResult` from `BackendServiceImplementationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- implementedSurface: the methods and schema this run implemented, against the declared spec version
- exercisedScenarioIds and unexercisedScenarioIds: what was proven and what was not
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Declared spec and handlers agree — surface matches `IntegrationSpec` with no divergence
- Non-safe method retried — the retry does not duplicate its effect
- Request field omitted or malformed — rejected at the boundary with the declared error code, no effect applied
- Endpoint with no server-side authorization — reported as client-side authorization, not accepted
- Webhook replayed — rejected before any effect
- Background job redelivered — the second delivery is a no-op
- Destructive migration with no stated recovery — not proposed; reported as irreversible
- Compile succeeds with no functional scenario run — implementation facts recorded, service state reported unestablished
- Capability UNAVAILABLE — blocked, nothing implemented
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
