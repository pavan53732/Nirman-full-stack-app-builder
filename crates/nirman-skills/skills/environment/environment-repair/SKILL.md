# Environment Repair

Scope: authorized repairs — missing tool, wrong tool version, missing
target, broken PATH, missing SDK or dependency, incorrect configuration
(BS §79.7).

## Trigger
This skill is requested when authorized repairs — missing tool, wrong tool version, missing target, broken PATH, missing SDK or dependency, incorrect configuration (BS §79.7).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `HOST_TOOL_OBSERVATION`
- `ENVIRONMENT_REPAIR`

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
- package-manager
- sdk-manager

## Procedure
1. Start only from a REPAIRABLE classification in the current
   EnvironmentCapabilityRecord — never from a model's guess, and never
   from a capability already classified UNAVAILABLE.
2. Read the repair target precisely: which capability, which tool, the
   observed version, and the required version range.
3. Submit the repair for policy admission before touching anything, and
   record the admission decision as durable evidence.
4. Choose the smallest repair that restores the capability: install a
   missing tool, align a version into its range, add a missing target,
   correct search-path order, or fix one configuration value.
5. Execute through the normal transaction path, capturing what changed,
   from what value to what value, and the exact action that changed it.
6. Re-run preflight: the new record supersedes the old one and the
   capability is re-classified deterministically from fresh observation.
7. Bound retries by the recoveryAttemptPolicy — an identical repair is
   never re-run against unchanged evidence, and a materially different
   repair escalates rather than terminating the goal.
8. Report the outcome truthfully, including a repair rejected by policy,
   one that failed, and one that left the capability unchanged.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `EnvironmentRepairResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * No repair may mark a capability AVAILABLE; the planner re-classifies
  *   from fresh observation.
  * A repair never downgrades, substitutes, or pins around a tool to make a
  *   version check pass; it brings the tool into its required range.
  * UNAVAILABLE is not repairable — a host that cannot provide a capability
  *   is reported with its reason, not repaired around.
  * A repair never writes credentials, never disables verification, and
  *   never weakens a security control in order to restore a capability.
  * The repair is scoped to the named capability; changing unrelated
  *   toolchain state is a defect, not a side effect.
  * A rejected or failed repair is reported truthfully with the durable
  *   evidence reference, and the blocked node names its resume condition.
  * Repair actions are evidence: every action, admission decision, and
  *   resulting classification is recorded, never asserted after the fact.
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

- REPAIR_NOT_ADMITTED — a repair was executed before policy admission, or despite a rejection.
- REPAIR_INEFFECTIVE — the repair ran and the capability did not change classification.
- REPAIR_EXCEEDED_SCOPE — a repair larger than the smallest one that restores the capability was applied.

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
Emits `EnvironmentRepairResult` from `EnvironmentRepairRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Start only from a REPAIRABLE classification in the current
- Step 2 produces its expected outcome — Read the repair target precisely: which capability, which tool, the
- Step 3 produces its expected outcome — Submit the repair for policy admission before touching anything, and
- Step 4 produces its expected outcome — Choose the smallest repair that restores the capability: install a
- Step 5 produces its expected outcome — Execute through the normal transaction path, capturing what changed,
- Step 6 produces its expected outcome — Re-run preflight: the new record supersedes the old one and the
- Step 7 produces its expected outcome — Bound retries by the recoveryAttemptPolicy — an identical repair is
- Step 8 produces its expected outcome — Report the outcome truthfully, including a repair rejected by policy,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
