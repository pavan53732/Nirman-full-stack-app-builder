# Android Architecture Expert

Scope: Android app architecture patterns — MVI/MVVM/MVP separation,
unidirectional data flow, domain/data/UI layering, dependency injection
(Hilt/Koin/manual), modularization strategy, and repository pattern
(BS §79.7). This skill provides the architectural domain knowledge that
the `Architecture Worker` and `Android Data and Integration Worker` consume.

## Trigger
This skill is requested when android app architecture patterns — MVI/MVVM/MVP separation, unidirectional data flow, domain/data/UI layering, dependency injection (Hilt/Koin/manual), modularization strategy, and repository pattern (BS §79.7). This skill provides the architectural domain knowledge that the `Architecture Worker` and `Android Data and Integration Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_SOURCE_ENGINEERING`

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
- architecture_documentation
- di_framework_selector

## Procedure
1. Analyze the product intent and identify architectural requirements:
   complexity scale, team size (single developer vs multi-module), data
   sources, offline requirements, and testing strategy.
2. Select the architectural pattern: MVI for complex state machines with
   predictable state transitions; MVVM for simpler screens with ViewModel
   + StateFlow; avoid MVP unless integrating with legacy code.
3. Define the layer structure: UI layer (Composables/ViewModels/Activities),
   Domain layer (UseCases/Interactors — optional for simple apps), Data
   layer (Repositories, DataSources, APIs, DAOs).
4. Design dependency injection: use Hilt for compile-time safety on
   complex projects, Koin for simplicity on smaller projects, or manual
   DI (AppContainer/ServiceLocator) for minimal overhead.
5. Define module boundaries: by feature (recommended) or by layer. Each
   module has its own build configuration files, DI module, and internal API.
6. Implement the repository pattern: single source of truth for each
   entity, with remote and local data sources coordinated through the
   repository.
7. Document the architecture decision in the ReasoningArtifact
   (BS §66.2) with the selected pattern, rationale, and trade-offs.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `ArchitectureDesignResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Unidirectional data flow: state flows down, events flow up. Never
  *   bypass the ViewModel/Presenter to mutate state directly.
  * Each layer depends only on layers below it — the UI layer never
  *   accesses data sources directly.
  * Repositories are the single source of truth — ViewModels/UseCases
  *   access data only through repositories.
  * DI is consistent across the project — do not mix Hilt and Koin.
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
Emits `ArchitectureDesignResult` from `ArchitectureDesignRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze the product intent and identify architectural requirements:
- Step 2 produces its expected outcome — Select the architectural pattern: MVI for complex state machines with
- Step 3 produces its expected outcome — Define the layer structure: UI layer (Composables/ViewModels/Activities),
- Step 4 produces its expected outcome — Design dependency injection: use Hilt for compile-time safety on
- Step 5 produces its expected outcome — Define module boundaries: by feature (recommended) or by layer. Each
- Step 6 produces its expected outcome — Implement the repository pattern: single source of truth for each
- Step 7 produces its expected outcome — Document the architecture decision in the ReasoningArtifact
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
