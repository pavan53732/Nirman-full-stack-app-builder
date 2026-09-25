# Android End-to-End Validation Orchestrator

Scope: end-to-end validation orchestration for a generated native Android
application — take the settled requirement set, enumerate every applicable
validation domain, enforce the ordering and dependency between those domains,
and emit one aggregate readiness result that names any domain that was not
evaluated (BS §79.7). This skill supplies composition and completeness
instruction that the `Test and QA Worker` and the `Release Worker` execute
inside their scoped transactions (BS §50). It does not replace a worker role,
and it is not itself a validator: it never produces a validation verdict that
the domain skill it invoked did not produce.

## Trigger
This skill is requested when a generated Android project's settled requirement
set is ready for validation and the applicable validation domains must be
enumerated, ordered, and accounted for before any release decision is
proposed. It is requested by end_to_end_validation, by
validation_completeness, and by release_readiness_orchestration. It does not
replace a worker role — it supplies the domain instruction the worker executes
inside its scoped asset transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_INTERACTION_EXECUTION`
- `ANDROID_RELEASE_VALIDATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to
  UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT execute and the
  blocked state MUST be reported.
- The construction contract is settled: the requirement set this run must
  account for has stable canonical requirement ids and one contract revision.
- The source revision is known, and where an artifact already exists its
  digest is known, so every record this skill emits binds to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- The settled requirement ids and their contract revision.
- Revision, and artifact digest where an artifact exists, plus the environment
  fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to the
  same evidence graph as the work that preceded them.

## Allowed tools
- validation_orchestrator
- coverage_matrix_ledger
- evidence_gap_reporter

## Procedure
1. Resolve the applicable domain set: from the settled requirement set, derive
   every validation domain this project's requirements actually engage, and
   record the derivation so an omitted domain is a visible omission rather than
   a silent one.
2. Order the domains: place each domain after the domains it depends on, so
   that a domain consuming another domain's observations never runs first.
3. Invoke each applicable domain skill in that order, passing the requirement
   ids in scope and binding each result to the same revision and environment
   fingerprint as the work that preceded it.
4. Record each domain outcome against its requirement ids, including a domain
   that was applicable but could not run in this environment.
5. Reconcile the union of per-domain outcomes with the requirement set, and
   name every requirement no invoked domain covered.
6. Audit completeness: confirm that no domain required by the requirement set
   was skipped, silently deferred, or reported without an observation.
7. Emit one aggregate readiness result carrying the ordered domain outcomes,
   the uncovered requirement ids, the domains that could not run and why, and
   the overall readiness determination for the caller to evaluate.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `EndToEndValidationResult` (§23 SkillPackage contract), carrying
  its classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every requirement in the settled set appears in the aggregate result as
    covered by at least one domain outcome or explicitly named as uncovered.
  * Every domain required by the requirement set appears in the aggregate
    result as evaluated, blocked with a stated reason, or named as not
    applicable with the reason it was excluded.
  * No domain outcome is reported without the observation it was derived from,
    and no unrun domain is reported as passed.
  * Each domain outcome is bound to the same revision and environment
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
- DOMAIN_NOT_EVALUATED — a domain the requirement set required neither ran nor
  was reported as blocked or excluded; completeness is not established and the
  aggregate result is not readiness.
- AGGREGATION_INCOMPLETE — a per-domain outcome could not be reconciled
  against the requirement set, so the aggregate result does not account for
  the whole set.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- A domain that fails is re-invoked only after its own precondition changed;
  the orchestrator never retries a domain on the domain's behalf and never
  substitutes its own judgment for that domain's verdict.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `EndToEndValidationResult` from `EndToEndValidationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed
- orderedDomainOutcomes: the per-domain outcomes in dependency order
- uncoveredRequirementIds: requirements no invoked domain covered
- notEvaluatedDomains: domains required but neither run nor reported blocked

## Fixtures
- Step 1 produces its expected outcome — Resolve the applicable domain set: from the settled requirement set, derive every validation domain this project's requirements actually engage, and record the derivation so an omitted domain is a visible omission rather than a silent one.
- Step 2 produces its expected outcome — Order the domains: place each domain after the domains it depends on, so that a domain consuming another domain's observations never runs first.
- Step 3 produces its expected outcome — Invoke each applicable domain skill in that order, passing the requirement ids in scope and binding each result to the same revision and environment fingerprint as the work that preceded it.
- Step 4 produces its expected outcome — Record each domain outcome against its requirement ids, including a domain that was applicable but could not run in this environment.
- Step 5 produces its expected outcome — Reconcile the union of per-domain outcomes with the requirement set, and name every requirement no invoked domain covered.
- Step 6 produces its expected outcome — Audit completeness: confirm that no domain required by the requirement set was skipped, silently deferred, or reported without an observation.
- Step 7 produces its expected outcome — Emit one aggregate readiness result carrying the ordered domain outcomes, the uncovered requirement ids, the domains that could not run and why, and the overall readiness determination for the caller to evaluate.
- A required capability is UNAVAILABLE — blocked, nothing attempted
- A required domain is silently omitted — reported as DOMAIN_NOT_EVALUATED, never as readiness
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.

    fingerprint as the run that produced it.
  * Ordering is preserved: a domain that consumes another domain's
    observations never reports before that domain produced them.

