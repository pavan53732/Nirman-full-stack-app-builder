# Environment Preflight

Scope: identify host and target; inspect toolchain, SDKs, runtimes, and
native dependencies; classify executable and validation capabilities;
produce the environment fingerprint (BS §79.7).

## Trigger
This skill is requested when identify host and target; inspect toolchain, SDKs, runtimes, and native dependencies; classify executable and validation capabilities; produce the environment fingerprint (BS §79.7).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- none — this skill produces the classification the others consume
  (BS §79.7) and must never be blocked by a capability it defines

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- No §79.7 capability is required, so this skill runs before
  implementation and is never gated by its own output.
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
1. Observe the host: operating system and version, CPU architecture,
   available memory and disk, and whether hardware virtualization is
   present and usable by a hypervisor.
2. Enumerate every required tool by probing it and capturing an observed
   version string — SDKs, compilers, runtimes, package managers, and
   platform tools. A version declared by the repository is not evidence.
3. Detect the configuration that changes behavior: search-path order,
   SDK roots, environment overrides, and whether a proxy sits in the
   network path.
4. Classify the network path as DIRECT, SYSTEM_PROXY, or PAC from the
   host's own configuration. Report an authenticating or intercepting
   proxy by name and observed status; never prompt for and never store
   proxy credentials (TA §49.4).
5. Run the deterministic EnvironmentCapabilityPlanner against the
   observed facts for the declared target, producing one classification
   per capability id of the §79.3 matrix.
6. Record each capability as AVAILABLE, REPAIRABLE, USER_REQUIRED, or
   UNAVAILABLE, with the reason and the observation that produced it.
7. Persist the EnvironmentCapabilityRecord — durable and fingerprinted,
   superseding the previous record only when the environment identity
   actually changed.
8. Publish the fingerprint so every later artifact, observation, and
   evidence record can bind to it, and report blocked capabilities with
   their resume conditions rather than as failures of the goal.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `EnvironmentCapabilityRecord` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * The model never sets or raises a capability state; the planner
  *   classifies from observation
  *   (CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION).
  * A missing tool is reported as such — never silently substituted and
  *   never hard-coded as unavailable.
  * Host environment, target platform, validation platform, and
  *   certification status stay distinct and are never collapsed into one
  *   build, validation, or completion result
  *   (CLAUSE.PLATFORM.HOST_TARGET_SEPARATION).
  * Compiling on the host, or cross-compiling for a target, never
  *   establishes native target-runtime capability, runtime validation, or
  *   certification (CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE).
  * Containers, virtual machines, the Windows subsystem for Linux, and
  *   simulated or remote environments never substitute for the declared
  *   target's native validation (CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET).
  * Preflight is read-only: it inspects and classifies, and it never
  *   repairs, installs, or mutates the environment it observes.
  * Output is the record and its fingerprint; it is evidence, not a claim.
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

- TOOL_VERSION_UNREAD — a required tool was probed and produced no observed version string.
- CAPABILITY_UNCLASSIFIED — a declared capability was left without a classification and without a reason.
- FINGERPRINT_UNPUBLISHED — the record was persisted but the fingerprint was not published for later binding.

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
Emits `EnvironmentCapabilityRecord` from `EnvironmentPreflightInput` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Observe the host: operating system and version, CPU architecture,
- Step 2 produces its expected outcome — Enumerate every required tool by probing it and capturing an observed
- Step 3 produces its expected outcome — Detect the configuration that changes behavior: search-path order,
- Step 4 produces its expected outcome — Classify the network path as DIRECT, SYSTEM_PROXY, or PAC from the
- Step 5 produces its expected outcome — Run the deterministic EnvironmentCapabilityPlanner against the
- Step 6 produces its expected outcome — Record each capability as AVAILABLE, REPAIRABLE, USER_REQUIRED, or
- Step 7 produces its expected outcome — Persist the EnvironmentCapabilityRecord — durable and fingerprinted,
- Step 8 produces its expected outcome — Publish the fingerprint so every later artifact, observation, and
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
