# Android Network Debugging

Scope: debugging the network layer on the Nirman-managed emulator — traffic
capture, TLS and certificate pinning failures, offline and degraded-network
behaviour, latency and failure injection, and verifying the app against the
API contract it actually depends on (BS §79.7).

## Trigger
A network interaction fails or behaves unexpectedly: a failed request, a TLS
error, a response the app mishandles, or behaviour under a network that is slow,
intermittent, or absent.

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_NETWORK_INTEGRATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- A session is leased and `ANDROID_EMULATOR_EXECUTION` resolves to AVAILABLE.
- `ANDROID_NETWORK_INTEGRATION` resolves to AVAILABLE for the target session.
- The endpoints in scope and the API contract the app expects are known.

## Context requirements
- The endpoints in scope and the contract the app expects of each.
- The network condition to reproduce: normal, slow, intermittent, or absent.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the session, so every record binds to them.
- Whether TLS interception is permitted for this capture, and how the
  capture will avoid capturing credentials.

## Allowed tools
- managed_emulator
- traffic_capture
- network_conditioner

## Procedure
1. Reproduce the interaction under capture, bounding the capture window to the
   scenario rather than recording the whole session.
2. Read the request actually sent, not the one the code intends: method, path,
   headers, body, and the resolved host.
3. Read the response as received: status, headers, body, timing, and whether a
   redirect or a retry changed it.
4. Separate transport failures from application failures by layer: DNS,
   connection, TLS, HTTP status, and body parsing each have a different owner.
5. Investigate TLS failures specifically: trust chain, hostname verification,
   and pinning, since a pin mismatch presents as a generic failure.
6. Simulate degraded conditions — offline, high latency, packet loss — and
   verify the app degrades instead of hanging or claiming success.
7. Verify the app against the contract: field names, nullability, and error
   bodies, since a contract drift fails as a parse error far from its cause.

## Evidence
- The captured request and response for each interaction in scope, with
  timing, bounded to the scenario window.
- The layer at which each failure occurred: DNS, connection, TLS, status, or
  parsing.
- Behaviour under each simulated condition, including the offline case.
- Contract divergences: fields the app expected that the response did not
  carry, or carried with a different type.
- Every record bound to revision, artifact digest, and device identity.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - Every request in scope is observed on the wire with its outcome, not reconstructed from the application's own logging.
  - A failure is attributed to a named layer rather than to the network in general.
  - Behaviour under a simulated condition is measured, not assumed from the condition that was configured.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- TRANSPORT_FAILURE — DNS, connection, or TLS failed before HTTP.
- CONTRACT_DIVERGENCE — the response does not match the contract the app
  expects.
- UNGRACEFUL_DEGRADATION — the app hangs, crashes, or claims success while
  offline or severely degraded.
- CAPTURE_FAILED — traffic could not be captured or was truncated.

## Recovery
- A truncated capture narrows its window and is retaken; a partial capture is
  never presented as complete.
- A contract divergence is escalated with the offending field rather than
  handled by loosening the parser to accept anything.
- Ungraceful degradation is fixed by making the failure path explicit, not by
  adding a retry that hides it.
- One retry is permitted after a materially changed input; an identical
  capture is never re-run against unchanged evidence.

## Output contract
Emits `NetworkDiagnosticsResult` (§23 SkillPackage contract):

- interactions: request, response, timing, and outcome per call
- failureLayer: DNS, connection, TLS, status, or parsing
- conditionsExercised: normal, slow, intermittent, offline
- contractDivergences: field, expected, and observed
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured network evidence

## Fixtures
- Successful request captured and verified against the contract
- Offline behaviour degrading gracefully
- TLS pinning mismatch presented as a generic failure
- Slow network handled without hanging
- Contract drift surfacing as a parse error
- Capability UNAVAILABLE — blocked, nothing captured

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
