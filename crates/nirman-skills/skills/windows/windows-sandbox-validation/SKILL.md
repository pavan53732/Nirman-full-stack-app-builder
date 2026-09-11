# Windows Sandbox Validation

Scope: the isolation boundary around a Nirman worker — what a sandboxed worker can and
cannot reach, credential and handle isolation, filesystem and network reach, and
proving an escape fails rather than leaving it untested (BS §79.7).

## Trigger
Isolation must be proven rather than asserted: before a trust boundary is relied
upon, after a change to the worker host, or when a worker is suspected of reaching
something outside its grant.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The grant the worker is supposed to hold is known: which paths, which handles,
  and which network destinations.

## Context requirements
- The declared grant: allowed paths, denied paths, and network policy.
- Which credentials the host holds that the worker must not see.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The isolation mechanism in force for this configuration.

## Allowed tools
- sandbox_probe
- process_probe

## Procedure
1. Enumerate what the worker can see from inside: filesystem roots, handles,
   environment, and the identity it runs under.
2. Test each denied path explicitly and confirm access is refused, recording the
   refusal rather than inferring it from the grant.
3. Test credential isolation: the worker must not be able to read the host's stored
   credentials or the handles that carry them.
4. Test network reach against the declared policy: allowed destinations reachable,
   denied destinations refused, not silently routed.
5. Attempt an escape deliberately — elevate, inject, or reach a sibling process —
   and confirm it fails and is logged.
6. Record the negative results as evidence: what was refused, and how the refusal
   surfaced.

## Evidence
- The worker's observed view: identity, visible roots, and handles.
- Per-denied-path refusal, recorded with the error surfaced.
- Credential isolation result: what was attempted and what was refused.
- Network policy results: allowed and denied, each observed.
- Escape attempts and their outcomes, each logged.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - The worker reaches nothing outside its declared grant.
  - No host credential is readable from inside the sandbox.
  - Every deliberate escape attempt fails and is logged; untested is not the same as impossible.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- GRANT_EXCEEDED — the worker reached something outside its declared grant.
- CREDENTIAL_REACHABLE — the worker could read credentials it must not see.
- NETWORK_POLICY_BYPASSED — a denied destination was reached.
- ESCAPE_SUCCEEDED — an escape attempt worked; treat as a security defect.

## Recovery
- A grant breach is fixed by narrowing the worker's grant, not by moving the resource
  somewhere the worker also reaches.
- A reachable credential is rotated and the storage path corrected; the skill does
  not merely confirm the worker failed to read it this time.
- A successful escape is escalated as a security defect and the configuration is not
  used until it is fixed.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `SandboxResult` (§23 SkillPackage contract):

- observedView: identity, visible roots, handles
- denialsObserved: each refused access and the error surfaced
- credentialIsolation: attempted and refused
- escapeAttempts: each attempt and its outcome
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured isolation evidence

## Fixtures
- Worker sees only its declared grant
- Denied path refused and logged
- Host credentials unreachable from the worker
- Denied network destination refused
- Escape attempt fails and is logged
- Capability UNAVAILABLE — blocked, nothing validated

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
