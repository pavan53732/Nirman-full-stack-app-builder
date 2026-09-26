# Android Documentation

Scope: authoring and maintaining a managed project's documentation, decision records,
and release notes, and keeping them revision-bound and honest about status (BS §79.7).
The handbook and the artifact release report are **generated** by
`ProjectHandbookService` and `ReleaseReportService` (TA §53.9); this skill writes the
material those services consume and reviews their output, and never restates,
overrides, or hand-authors what they produce.

## Trigger
Project documentation, a decision record, or a release note is being written, revised,
or reconciled against a revision, or a promoted artifact is missing its release
report, or a managed project is missing its handbook (BS §79.7). It does not replace a
worker role — it supplies the domain instruction the worker executes inside its scoped
asset transaction (BS §50).

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale; capability is
  read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to UNAVAILABLE
  or USER_REQUIRED, the gated steps MUST NOT execute and the blocked state MUST be
  reported.
- The revision the documentation describes is known. Documentation that cannot name the
  revision it describes is not written, because it cannot be invalidated when that
  revision moves.
- The source revision and, where one exists, the artifact digest are known, so every
  record this skill emits can be bound to them.

## Context requirements
- The revision, and the artifact digest and checkpoint where an artifact is involved.
- The decision being recorded, its alternatives, and its consequences — or the artifact
  whose release report is being written.
- The evidence identifiers the documentation may cite, and the capability status values
  it may state.
- Which documentation paths are in scope, since the worker edits documentation and does
  not restate the product or architecture contracts.

## Allowed tools
- documentation_revision_binder
- decision_record_authoring
- release_note_composer
- handbook_gap_check

## Procedure
1. Bind the document to a revision before writing, and state that binding in the
   document itself. An unbound document cannot be invalidated, so it is worse than an
   absent one.
2. Write the decision record before the decision is described as settled elsewhere:
   the options considered, the option chosen, the reasons, the consequences accepted,
   and what would reverse it. A record with no reversal condition is an announcement.
3. Keep status claims to the vocabulary that actually exists. A capability is described
   with its real status, and a planned capability is never written in the present tense
   as though it were implemented.
4. Cite evidence rather than asserting it. A statement that a scenario ran names the
   evidence and the revision it ran against; a statement with no evidence behind it is
   written as an intention.
5. Compose the release note from what the artifact actually contains at its digest, not
   from what the plan intended to contain, and state the validation and evidence status
   the artifact really has.
6. Reconcile the handbook against the revision: every section that names a capability,
   a contract, or a milestone is checked against the current state, and a stale section
   is corrected or marked as describing an earlier revision.
7. Leave the generated artifacts to their services. When the handbook or the release
   report is missing or wrong, the cause is reported for the owning service to correct;
   this skill does not hand-write over generated output.
8. Emit the result bound to the revision, and state each document as written, revised,
   or unreconciled, so an unreconciled document is never read as current.

## Evidence
- A record of each procedure step that executed, with the outcome observed and the step
  that produced it, bound to the source revision and the environment fingerprint; a
  step that ran and recorded nothing is not evidence that it succeeded.
- The emitted `DocumentationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a statement the
  evidence above has to support:
  - Every document names the revision it describes, so a change to that revision can
    invalidate it. An unbound document is reported as a defect, not shipped.
  - A status claim uses the real status vocabulary. A planned or environment-qualified
    capability is never described in the present tense as supported, and documentation
    never claims completion, promotion, or a passing validation it cannot cite.
  - Documentation cannot create or weaken a contract. Where a document and a canonical
    section disagree, the canonical section wins and the document is corrected; the
    conflict is reported rather than resolved in the document's favour.
  - The handbook and the artifact release report are produced by
    `ProjectHandbookService` and `ReleaseReportService` (TA §53.9). This skill supplies
    and reviews their material and never hand-writes over their output.
  - A release note describes the artifact at its digest, not the plan that produced it.
  - A statement with no evidence behind it is written as an intention, never as an
    observation.
  - Documentation is not evidence of runtime behaviour. A document claiming that a
    capability works is a claim about a claim, and the real evidence remains the
    observation bound to the revision.
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
- UNBOUND_DOCUMENT — a document names no revision, so it cannot be invalidated.
- STATUS_OVERCLAIMED — a document describes a planned or environment-qualified
  capability as supported.
- UNCITED_CLAIM — a status or validation claim names no evidence.
- CONTRACT_CONFLICT — a document and a canonical section disagree.
- GENERATED_OUTPUT_HAND_EDITED — generated handbook or release-report content was
  written by hand.
- STALE_SECTION — a section describes a state that no longer holds at this revision.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked node
  names its resume condition and is never reported as a failure of the goal.
- One retry is permitted after a repair that materially changed the input. An identical
  action is never re-run against unchanged evidence, and the recoveryAttemptPolicy
  bound escalates rather than terminating the goal.
- An overclaimed status is corrected downward to the status the evidence supports,
  never upward to the status the document wanted.
- A contract conflict is resolved in the canonical section's favour and the document is
  corrected; the conflict is reported, and the document is never edited to argue the
  other side.
- A hand-edited generated artifact is reverted to the service's output and the
  underlying input is corrected, because the generator will overwrite the edit again.
- An invariant violation is repaired at its cause and never by softening the wording
  that made the violation legible.

## Output contract
Emits `DocumentationResult` from `DocumentationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- documents: each document written or revised, with the revision it is bound to
- reconciliation: per document, written, revised, or unreconciled
- evidenceRefs: identifiers of the evidence the documents cite
- revision and, where one exists, artifactDigest: what the documentation is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Documentation written for a named revision — the binding is stated in the document
- Decision recorded with alternatives, reasons, consequences, and a reversal condition
- Planned capability described in the present tense — reported as an overclaimed status
- Validation claimed with no evidence named — reported as an uncited claim
- Document contradicts a canonical section — reported as a contract conflict, canonical section wins
- Handbook missing a section the revision requires — reported as a gap for the owning service
- Release note describes intent rather than the artifact's digest — reported, not shipped
- Capability UNAVAILABLE — blocked, nothing written
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
