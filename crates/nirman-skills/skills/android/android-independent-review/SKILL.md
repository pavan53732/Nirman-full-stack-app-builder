# Android Independent Review

Scope: independent adversarial review of a generated native Android project —
attack the construction plan, the validation strategy, and the completion
claim before authorization and promotion, and emit read-only findings in the
blocking, warning, and informational classes that M53 requires (BS §79.7).
This skill supplies the adversarial instruction that the `Critic Worker`
executes inside its scoped read-only transaction (BS §23.4, BS §50). It does
not replace a worker role: findings are evidence requests and verdicts on
claims, never approvals, never promotion, never completion.

## Trigger
This skill is requested when a plan, strategy, or completion claim must be
attacked before authorization and promotion: critique the construction plan
for unproven steps, the validation strategy for unexamined surfaces, and the
completion claim for uncited evidence. It is requested by plan_critique, by
strategy_review, and by completion_claim_challenge. It does not replace a
worker role — it supplies the adversarial instruction the `Critic Worker`
executes inside its scoped read-only asset transaction (BS §23.4, BS §50).

## Required capabilities
- `ANDROID_SOURCE_ENGINEERING`
- `ANDROID_EMULATOR_EXECUTION`
- `ANDROID_RELEASE_VALIDATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to
  UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT execute and the
  blocked state MUST be reported.
- The plan, strategy, or claim under review exists as a durable record; a
  finding NEVER addresses a model statement that has no record.
- The source revision and, where one exists, the artifact digest are known,
  so every finding binds to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- The plan, strategy, or claim under review, with its record identity.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so findings attach to
  the same evidence graph as the work they attack.

## Allowed tools
- critique_planner
- evidence_rebuttal_ledger
- finding_severity_binder

## Procedure
1. Fix the target: name the plan, strategy, or claim under review and the
   record that carries it; a critique without a named target is not a
   finding.
2. Attack the construction plan: challenge each step that no evidence
   supports, each dependency that is assumed rather than bound, and each
   technology choice whose rationale is missing.
3. Attack the validation strategy: enumerate the surfaces the strategy does
   not examine, the requirement with no executable acceptance path, and the
   scenario whose postcondition is asserted without an observation.
4. Attack the completion claim: verify that every cited evidence identifier
   exists, is current for this revision, and supports the claim it is cited
   for; a claim whose evidence is stale, missing, or mismatched is refuted.
5. For each refutation, name the evidence that would refute or confirm the
   claim (BS §6.5), so the finding is an evidence request, not an opinion.
6. Assign each finding its M53 class: blocking where promotion must stop,
   warning where the risk is recorded but work may continue, informational
   where the observation aids future review.
7. Emit one aggregate review result carrying every finding, its class, its
   target record, and the evidence request behind it.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `IndependentReviewResult` (§23 SkillPackage contract), carrying
  its classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every finding names its target record — a critique is addressed to a
    durable plan, strategy, or claim, never to a model statement.
  * Every refutation carries its evidence request — the evidence that would
    refute or confirm the claim is named for each finding.
  * Every finding carries its M53 class — blocking, warning, or
    informational, assigned deliberately rather than defaulted.
  * No finding approves, promotes, or completes — findings are read-only and
    a finding is never reported as an approval.
  * Nothing unexamined is reported as examined — a surface the review did not
    read is named as unexamined rather than passed.
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
- TARGET_UNRECORDED — the plan, strategy, or claim under review has no
  durable record; there is nothing to attack and no finding is emitted.
- EVIDENCE_REQUEST_UNANSWERED — a refutation names evidence that cannot be
  produced; the finding stands and the claim it attacks is reported as
  unsupported.
- APPROVAL_ATTEMPTED — the run produced a verdict that approves, promotes, or
  completes; this is forbidden to a read-only review and is reported as a
  defect in the run, never as an outcome.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- An unrecorded target is escalated to the producer of the plan, strategy, or
  claim; a review is never conducted against a transcript summary.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `IndependentReviewResult` from `IndependentReviewRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed
- findings: each finding with its target record, its M53 class, and its
  evidence request
- blockingFindings: the subset that must stop promotion

## Fixtures
- Step 1 produces its expected outcome — Fix the target: name the plan, strategy, or claim under review and the record that carries it; a critique without a named target is not a finding.
- Step 2 produces its expected outcome — Attack the construction plan: challenge each step that no evidence supports, each dependency that is assumed rather than bound, and each technology choice whose rationale is missing.
- Step 3 produces its expected outcome — Attack the validation strategy: enumerate the surfaces the strategy does not examine, the requirement with no executable acceptance path, and the scenario whose postcondition is asserted without an observation.
- Step 4 produces its expected outcome — Attack the completion claim: verify that every cited evidence identifier exists, is current for this revision, and supports the claim it is cited for; a claim whose evidence is stale, missing, or mismatched is refuted.
- Step 5 produces its expected outcome — For each refutation, name the evidence that would refute or confirm the claim (BS section 6.5), so the finding is an evidence request, not an opinion.
- Step 6 produces its expected outcome — Assign each finding its M53 class: blocking where promotion must stop, warning where the risk is recorded but work may continue, informational where the observation aids future review.
- Step 7 produces its expected outcome — Emit one aggregate review result carrying every finding, its class, its target record, and the evidence request behind it.
- A required capability is UNAVAILABLE — blocked, nothing attempted
- A plan with no durable record — reported as TARGET_UNRECORDED, never reviewed from summary
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.

