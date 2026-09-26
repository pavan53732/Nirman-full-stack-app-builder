# Android Release Readiness

Scope: the cross-cutting release gate for a generated native Android project —
consume the per-domain evidence the build, manifest, permissions, signing,
artifact, installability, runtime, test, visual, accessibility, performance,
privacy, dependency, and delivery skills produced, and emit one aggregate
readiness verdict with per-domain outcomes and explicit BLOCKED reasons
(BS §79.7). This skill supplies the gate instruction that the `Release Worker`
and the `Test and QA Worker` execute inside their scoped transactions
(BS §50). It reports readiness; it never promotes, never signs, never
delivers.

## Trigger
This skill is requested when the generated project must be judged as a whole
for release: aggregate every domain's evidence into one readiness verdict
before any promotion decision is proposed. It is requested by release_gate,
by promotion_readiness, and by release_evidence_aggregate. It does not replace
a worker role — it supplies the gate instruction the worker executes inside
its scoped asset transaction (BS §50).

## Required capabilities
- `ANDROID_RELEASE_VALIDATION`
- `ANDROID_ARTIFACT_INSPECTION`
- `ANDROID_SIGNING_INSPECTION`
- `ANDROID_PACKAGING`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to
  UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT execute and the
  blocked state MUST be reported.
- The source revision and artifact digest are known, because a release verdict
  with no artifact binding is meaningless.
- Per-domain outcomes exist to aggregate; the gate consumes evidence, it never
  fabricates the evidence it is missing.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- The per-domain evidence available to aggregate, with the identity of the
  skill run that produced each outcome.
- Revision and artifact digest, plus the environment fingerprint of the
  session.
- The evidence identifiers this run must bind to, so the verdict attaches to
  the same evidence graph as the work it judges.

## Allowed tools
- release_gate_matrix
- artifact_provenance_probe
- promotion_block_ledger

## Procedure
1. Collect the per-domain outcomes: for each domain the requirement set
   engages, locate the outcome its implementing skill produced; a domain with
   no outcome is named as unevaluated, never assumed ready.
2. Verify artifact provenance: confirm the artifact digest, signature
   identity, and packaging match the revision under test; a mismatch voids
   every outcome bound to a different identity.
3. Evaluate each domain against its release criteria: build, manifest,
   permissions, signing, artifact, installability, runtime, tests, visual,
   accessibility, performance, privacy, dependency, and delivery.
4. Record per-domain verdicts with their evidence and their explicit BLOCKED
   reasons where a capability or admission blocked the domain itself.
5. Emit one aggregate readiness verdict: ready only when every engaged domain
   reports ready; otherwise blocked with the named domains and reasons.
6. Bind the verdict to the revision, the artifact digest, and the environment
  fingerprint, so it cannot be replayed against another build.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `ReleaseReadinessResult` (§23 SkillPackage contract), carrying
  its classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every engaged domain appears in the verdict — a domain is ready, blocked
    with a reason, or named as unevaluated; none is silently absent.
  * Provenance binds the verdict — the revision, artifact digest, and
    environment fingerprint of the verdict match the artifact judged.
  * No aggregated outcome is fabricated — every per-domain input cites the
    skill run and evidence that produced it.
  * Readiness is unanimous — the verdict is ready only when every engaged
    domain reports ready; any other state is blocked.
  * Blockage is explicit — every blocked domain names its reason and the
    condition that would unblock it.
  * Every claim is reduced to an observable: what was seen, on which device or
    host, at which revision — never a statement of intent.

## Failure classification
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
- PROVENANCE_MISMATCH — the artifact digest, signature identity, or packaging
  does not match the revision under test; every dependent outcome is void.
- DOMAIN_UNEVALUATED — a domain the requirement set engages has no outcome
  and is not named as blocked or excluded; the verdict is blocked, never
  ready.
- PROMOTION_ATTEMPTED — the run produced a promotion, signature, or delivery
  action; this is forbidden to a readiness gate and is reported as a defect
  in the run, never as an outcome.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- A provenance mismatch is escalated to the build and packaging owners; the
  gate never rebinds a verdict to a different artifact by assumption.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `ReleaseReadinessResult` from `ReleaseReadinessRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed
- perDomainOutcomes: each engaged domain with its verdict, evidence, and
  BLOCKED reason where applicable
- overallVerdict: ready or blocked, with the domains and reasons behind it

## Fixtures
- Step 1 produces its expected outcome — Collect the per-domain outcomes: for each domain the requirement set engages, locate the outcome its implementing skill produced; a domain with no outcome is named as unevaluated, never assumed ready.
- Step 2 produces its expected outcome — Verify artifact provenance: confirm the artifact digest, signature identity, and packaging match the revision under test; a mismatch voids every outcome bound to a different identity.
- Step 3 produces its expected outcome — Evaluate each domain against its release criteria: build, manifest, permissions, signing, artifact, installability, runtime, tests, visual, accessibility, performance, privacy, dependency, and delivery.
- Step 4 produces its expected outcome — Record per-domain verdicts with their evidence and their explicit BLOCKED reasons where a capability or admission blocked the domain itself.
- Step 5 produces its expected outcome — Emit one aggregate readiness verdict: ready only when every engaged domain reports ready; otherwise blocked with the named domains and reasons.
- Step 6 produces its expected outcome — Bind the verdict to the revision, the artifact digest, and the environment fingerprint, so it cannot be replayed against another build.
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An artifact whose digest mismatches the revision — reported as PROVENANCE_MISMATCH, no verdict emitted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.

