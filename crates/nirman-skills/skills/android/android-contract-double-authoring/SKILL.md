# Android Contract Double Authoring

Scope: authoring the fixtures and schema-derived bodies a supervisor-owned
`ContractDouble` serves for a declared integration — the declared scenarios, the
request, response, and error bodies generated from the integration's schema
references, and the classification of the evidence the double yields (BS §79.7). The
double itself is owned and run by the supervisor inside the Android runtime on a
loopback address and is torn down with the session; this skill never opens it, never
reaches it, and never contacts the real service (TA §74.1).

## Trigger
A declared integration needs its double's scenarios or bodies authored, a declared
schema reference changed in a way that invalidates existing fixtures, or a
double-backed observation is being read as evidence about the real service (BS §79.7).
It does not replace a worker role — it supplies the domain instruction the worker
executes inside its scoped asset transaction (BS §50).

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale; capability is
  read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to UNAVAILABLE
  or USER_REQUIRED, the gated steps MUST NOT execute and the blocked state MUST be
  reported.
- The integration's declared schema references and its declared scenarios are
  available. Without them there is nothing to conform to, and a body invented from
  imagination is a double failure, not a fixture.
- The source revision and, where one exists, the artifact digest are known, so every
  record this skill emits can be bound to them.

## Context requirements
- The integration identity, its `requestSchemaRef`, `responseSchemaRef`,
  `errorSchemaRef`, `authState`, and its declared functional scenario ids.
- The revision and environment fingerprint the double's evidence will carry.
- Which declared scenarios this change covers and which it leaves uncovered.

## Allowed tools
- double_fixture_authoring
- schema_conformance_check
- scenario_matrix_build
- double_evidence_classifier

## Procedure
1. Read the integration's declared schema references first. Every body this skill
   authors is generated from those references, never from a sample response captured
   from the real service.
2. Author one fixture per declared scenario, and state the scenario each fixture
   serves. A fixture serving no declared scenario is removed rather than kept as
   decoration, and a declared scenario with no fixture is reported uncovered.
3. Generate the request, response, and error bodies from the schema references
   directly. Confirm every field the schema requires is present and every field it
   forbids is absent, since the supervisor serves these bodies verbatim.
4. Conform to the error schema specifically: an error body carries the declared error
   code and the declared detail, and never an internal message, a stack trace, or a
   field the error schema does not declare.
5. Honour the declared `authState`. A double for an authenticated integration
   rejects an unauthenticated request the way the contract says it does, rather than
   serving a success to keep a test moving.
6. Keep every fixture deterministic. A double that varies between runs produces
   evidence that cannot be compared, so a value that must vary is declared as a
   scenario input rather than left to chance.
7. Classify the evidence the double yields as `DOUBLE_BACKED` and name it as such, so
   `EvidenceAuthority` can hold it out of the functional accounting for the real
   service (BS §76.5).
8. Emit the result bound to the revision, and state each declared scenario as covered,
   uncovered, or unverifiable, so an untested scenario is never read as a tested one.

## Evidence
- A record of each procedure step that executed, with the outcome observed and the step
  that produced it, bound to the source revision and the environment fingerprint; a
  step that ran and recorded nothing is not evidence that it succeeded.
- The emitted `ContractDoubleAuthoringResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a statement the
  evidence above has to support:
  - The double is owned and run by the supervisor. This skill authors fixtures and
    bodies only; it never opens the double, never sends a request to it, and never
    contacts the real service to learn what it should return (TA §74.1).
  - Every body served is producible from the integration's declared schema
    references. A response the schema cannot produce is a double failure, never
    application evidence and never a silently accepted fixture.
  - Evidence the double yields carries `DOUBLE_BACKED` and never counts toward the
    functional state of the real service (BS §76.5). A double-backed pass is not
    evidence that the service works.
  - Runtime perception is not required to author a double, because nothing in this
    skill observes a running service: the bodies come from declared schemas, and the
    double's own execution is the supervisor's. Observing the double belongs to
    interaction validation, not here.
  - A credential is referenced through the declared credential reference and is never
    read, embedded, or echoed; a fixture authenticates as the contract declares and
    holds no real secret.
  - A declared scenario with no fixture is reported uncovered, and a fixture serving
    no declared scenario is reported surplus. Neither is passed off as coverage.
  - Nothing unverified is reported as verified.

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
- FIXTURE_NOT_SCHEMA_PRODUCIBLE — a body carries a field the declared schema forbids
  or omits one it requires.
- SCENARIO_UNCOVERED — a declared functional scenario has no fixture.
- FIXTURE_SURPLUS — a fixture serves no declared scenario.
- NON_DETERMINISTIC_FIXTURE — a fixture's value varies between runs.
- DOUBLE_REACHED_DIRECTLY — the work required contacting the double or the real
  service, which this skill never does.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked node
  names its resume condition and is never reported as a failure of the goal.
- One retry is permitted after a repair that materially changed the input. An identical
  action is never re-run against unchanged evidence, and the recoveryAttemptPolicy
  bound escalates rather than terminating the goal.
- A schema divergence is repaired at the fixture, by regenerating the body from the
  declared reference, never by loosening the reference to accept what was written.
- A non-deterministic fixture is made deterministic by declaring its varying input as
  a scenario parameter, not by re-running until it happens to agree.
- An invariant violation is repaired at its cause and never by relaxing the check that
  detected it.

## Output contract
Emits `ContractDoubleAuthoringResult` from `ContractDoubleAuthoringRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- coveredScenarioIds and uncoveredScenarioIds: what the fixtures serve and what they do not
- schemaConformance: per schema reference, the fields confirmed present and forbidden
- evidenceClass: DOUBLE_BACKED, always, for anything this run produced
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Declared scenarios each have a fixture — every declared scenario covered, none surplus
- Response body carries a field the schema forbids — reported as a double failure, not served
- Error body contains an internal stack trace — rejected against the error schema
- Authenticated integration receives an unauthenticated request — rejected as the contract declares, not served
- Fixture value varies between runs — reported non-deterministic, not accepted
- Double-backed observation read as evidence about the real service — rejected, evidence stays DOUBLE_BACKED
- Declared scenario with no fixture — reported uncovered, never counted as covered
- Capability UNAVAILABLE — blocked, nothing authored
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
