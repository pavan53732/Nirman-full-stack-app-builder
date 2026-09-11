# Cross-Platform Build Diagnostics

Scope: for a host→target pair, determine what can be cross-built, which
artifacts can be produced, and which validation evidence necessarily
remains missing (BS §79.7).

## Trigger
This skill is requested when for a host→target pair, determine what can be cross-built, which artifacts can be produced, and which validation evidence necessarily remains missing (BS §79.7).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- host-tool-probe

## Procedure
1. Read the current EnvironmentCapabilityRecord for the pair; never
   re-derive capability from the repository or from a prior run.
2. State the pair precisely: host operating system and architecture,
   target operating system and architecture, and whether the target is
   the host.
3. Classify cross-buildability: which toolchains on this host can emit a
   target artifact, and under what constraints they do so.
4. List the producible artifacts — which binaries, packages, or bundles
   the host can actually emit for the target from the observed tools.
5. Name the validation evidence that necessarily remains absent: native
   runtime execution, native installer and uninstaller behavior,
   platform-specific UI behavior, and device-specific checks.
6. Separate the three states that are easily conflated — the artifact
   compiles, the artifact runs on the host, and the artifact runs on the
   target — and say plainly which one each piece of evidence supports.
7. Emit the blocked-node report: the reason, the resume condition, and
   both §79.11 lists.
8. Invalidate the diagnosis when the environment fingerprint changes; a
   report bound to a stale fingerprint is void and is re-derived.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `BuildDiagnosticsReport` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Missing evidence is named, never simulated.
  * The report is the blocked-node input: it states the reason, the resume
  *   condition, and both §79.11 lists.
  * A host-specific limitation is reported from observation, never
  *   hard-coded as universally unavailable.
  * Cross-compilation success never establishes target-runtime capability;
  *   a target artifact that has not run on the target is unverified.
  * The diagnosis binds to the environment fingerprint; a changed
  *   fingerprint voids it.
  * This skill diagnoses only — it neither repairs the environment nor
  *   produces the evidence it reports missing.
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

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `BuildDiagnosticsReport` from `BuildDiagnosticsRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Read the current EnvironmentCapabilityRecord for the pair; never
- Step 2 produces its expected outcome — State the pair precisely: host operating system and architecture,
- Step 3 produces its expected outcome — Classify cross-buildability: which toolchains on this host can emit a
- Step 4 produces its expected outcome — List the producible artifacts — which binaries, packages, or bundles
- Step 5 produces its expected outcome — Name the validation evidence that necessarily remains absent: native
- Step 6 produces its expected outcome — Separate the three states that are easily conflated — the artifact
- Step 7 produces its expected outcome — Emit the blocked-node report: the reason, the resume condition, and
- Step 8 produces its expected outcome — Invalidate the diagnosis when the environment fingerprint changes; a
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
