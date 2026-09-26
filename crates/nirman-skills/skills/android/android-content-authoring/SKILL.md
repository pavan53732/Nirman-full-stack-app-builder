# Android Content Authoring

Scope: writing and revising product copy, localization strings, and accessibility
text for a generated Android application as `ContentMutation` proposals that carry a
`ContentRevisionDraft` (BS §79.7). The worker proposes only: admission is
`ContentTransactionCoordinator`'s, validation is `ContentValidator`'s, the revision is
minted by `ContentAuthority`, and persistence is `ContentStore`'s (TA §85.1).

## Trigger
Product copy, a localized string, or accessibility text is being written or revised, a
terminology change is propagating, or a content proposal is being prepared for
admission (BS §79.7). It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset transaction (BS §50).

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale; capability is
  read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to UNAVAILABLE
  or USER_REQUIRED, the gated steps MUST NOT execute and the blocked state MUST be
  reported.
- The base project revision is known and current. A proposal whose base revision is not
  the current project revision is stale and is rejected at admission, so this skill
  does not prepare one.
- The source revision and, where one exists, the artifact digest are known, so every
  record this skill emits can be bound to them.

## Context requirements
- The requirements and context the content serves, and the `TerminologyProfile` in
  force.
- The locale set the application declares, and which locales this change covers.
- The revision and the environment fingerprint the proposal will be bound to.
- The `ContentDependency` edges that will make this content propagate elsewhere, so a
  change is proposed where it actually lands.

## Allowed tools
- terminology_profile_check
- content_draft_authoring
- locale_coverage_scan
- accessibility_text_review

## Procedure
1. Read the requirements and the `TerminologyProfile` before writing a word. A term the
   profile already fixes is used as the profile words it, not as this proposal words
   it.
2. Draft the copy against the requirement it serves, and record which requirement each
   piece of content serves, so a string with no requirement behind it is visible as
   surplus rather than merged.
3. Write accessibility text for every element that conveys meaning through text
   alone, describing the purpose rather than restating the label, and never encoding an
   instruction the user is expected to infer.
4. Localize per declared locale without dropping a key, and keep placeholder names and
   their order identical across locales, so a translation cannot silently change what
   a formatted string receives.
5. Keep the proposal free of authoritative state. It carries a `ContentRevisionDraft`
   and nothing that asserts admission: no revision identity, no transaction identity,
   no validation status, no approval state, no source evidence identifiers. A proposal
   carrying any of those is rejected, so this skill never adds one to make a draft look
   finished.
6. Set the base project revision to the current one, and state which
   `ContentDependency` edges this change reaches, so propagation is proposed rather
   than assumed to have happened.
7. Emit the proposal bound to the revision, and state each string as covered,
   uncovered, or unverifiable per locale, so a locale this run did not touch is never
   read as one it did.

## Evidence
- A record of each procedure step that executed, with the outcome observed and the step
  that produced it, bound to the source revision and the environment fingerprint; a
  step that ran and recorded nothing is not evidence that it succeeded.
- The emitted `ContentAuthoringResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a statement the
  evidence above has to support:
  - This worker proposes and never admits. It cannot write canonical content state,
    cannot mint a revision, and cannot mark content complete; admission belongs to
    `ContentTransactionCoordinator`, `ContentValidator`, and `ContentAuthority`
    (TA §85.1).
  - A proposal carries a `ContentRevisionDraft` and no authoritative field. A draft
    carrying a revision identity, a transaction identity, a validation status, an
    approval state, or source evidence identifiers is rejected, and this skill never
    adds one to make a draft appear complete.
  - The base project revision equals the current project revision; a proposal built on
    a stale base is reported stale rather than carried forward.
  - Accessibility text describes purpose rather than repeating a label, and no
    content instructs the user by implication instead of statement.
  - Placeholder names and their order are identical across every locale, and a
    translation never changes what a formatted string receives.
  - Runtime perception is not required to author content, because nothing here
    observes the application: the output is text plus its proposal. Whether that text
    renders and reads correctly is observed by accessibility and visual validation,
    not here.
  - A locale this run did not cover is reported uncovered, never implied by the
    locales it did.
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
- AUTHORITATIVE_FIELD_SMUGGLED — a draft carries a revision identity, a transaction
  identity, a validation status, an approval state, or source evidence identifiers.
- STALE_BASE_REVISION — the proposal's base revision is not the current project
  revision.
- TERMINOLOGY_DRIFT — content uses a term the `TerminologyProfile` fixes differently.
- PLACEHOLDER_MISMATCH — a locale's placeholder names or their order differ from the
  base.
- ACCESSIBILITY_TEXT_MISSING — a meaning-bearing element has no text describing its
  purpose.
- LOCALE_UNCOVERED — a declared locale has no content for a changed string.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked node
  names its resume condition and is never reported as a failure of the goal.
- One retry is permitted after a repair that materially changed the input. An identical
  action is never re-run against unchanged evidence, and the recoveryAttemptPolicy
  bound escalates rather than terminating the goal.
- A smuggled authoritative field is removed from the draft, never legitimised by
  admitting it; the field is minted by the authority, not proposed by the worker.
- Terminology drift is repaired at the content, using the profile's term, never by
  amending the profile to match the copy.
- A placeholder mismatch is repaired in the translation, never by removing the
  placeholder from the base so the locales agree.
- An invariant violation is repaired at its cause and never by relaxing the check that
  detected it.

## Output contract
Emits `ContentAuthoringResult` from `ContentAuthoringRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- proposedMutations: the `ContentMutation` proposals, each with its `ContentRevisionDraft` and base project revision
- coveredLocales and uncoveredLocales: what this run addressed and what it did not
- propagationScope: the `ContentDependency` edges the change reaches
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Copy written against a named requirement — the requirement each string serves is recorded
- Draft carries a validation status — rejected as a smuggled authoritative field, not admitted
- Base revision is not the current one — reported stale at admission
- Content uses a term the profile fixes differently — reported as terminology drift
- A locale drops a placeholder or reorders them — reported as a placeholder mismatch
- A meaning-bearing element has no accessibility text — reported missing
- A declared locale has no content for a changed string — reported uncovered
- Capability UNAVAILABLE — blocked, nothing drafted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
