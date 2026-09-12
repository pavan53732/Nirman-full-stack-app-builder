# Android Quality Expert

Scope: Android code quality — Android Lint, Detekt, Ktlint, code smell
detection, static analysis enforcement, and coding standard compliance
(BS §79.7). This skill provides the quality domain knowledge that the
`Security Worker` and `Reconciliation Worker` consume.

## Trigger
This skill is requested when android code quality — Android Lint, Detekt, Ktlint, code smell detection, static analysis enforcement, and coding standard compliance (BS §79.7). This skill provides the quality domain knowledge that the `Security Worker` and `Reconciliation Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_SOURCE_ENGINEERING`
- `ANDROID_RELEASE_VALIDATION`

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
- android_lint
- detekt
- ktlint

## Procedure
1. Analyze quality requirements: identify coding standards, lint rules,
   and static analysis tools to apply.
2. Configure Android Lint: enable `abortOnError = true`, disable
   irrelevant checks, and create custom lint rules for project-specific
   patterns. Run `./gradlew lint` on every build.
3. Configure Detekt: define detekt.yml with rule thresholds
   (complexity, long classes, function length), enable auto-correct
   for safe fixes, and run `./gradlew detekt` on every build.
4. Configure Ktlint: define .editorconfig for code style, enable
   ktlintFormat for auto-formatting, and run `./gradlew ktlintCheck`
   on every build.
5. Enforce quality gates: fail the build on lint errors, Detekt
   threshold breaches, or Ktlint violations. The QualityGate
   (CAP.ANDROID.QUALITY_GATE) MUST pass before artifact promotion.
6. Document quality decisions: record which rules are disabled and why
   in the ReasoningArtifact (BS §66.2).

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `QualityConfigurationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Quality gates are enforced on every build — lint, Detekt, and Ktlint run on
  *    every build. Failures block artifact promotion.
  * Custom rules are documented — any disabled rule or custom rule has
  *    a documented rationale in the ReasoningArtifact.
  * Auto-correct is preferred — Ktlint and Detekt auto-fix are applied
  *    before manual review. Only unfixable issues surface to the user.
  * Quality is measured, not assumed — the `Security Worker` runs static
  *    analysis and reports findings as evidence, not as model claims.
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

- GATE_DISABLED — lint or a static analysis gate was disabled without a recorded reason.
- THRESHOLD_BREACHED — a complexity or length threshold was exceeded without failing the build.
- RULE_SUPPRESSED_UNRECORDED — a suppression exists with no recorded justification.

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
Emits `QualityConfigurationResult` from `QualityConfigurationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze quality requirements: identify coding standards, lint rules,
- Step 2 produces its expected outcome — Configure Android Lint: enable `abortOnError = true` for CI, disable
- Step 3 produces its expected outcome — Configure Detekt: define detekt.yml with rule thresholds
- Step 4 produces its expected outcome — Configure Ktlint: define .editorconfig for code style, enable
- Step 5 produces its expected outcome — Enforce quality gates: fail the build on lint errors, Detekt
- Step 6 produces its expected outcome — Document quality decisions: record which rules are disabled and why
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
