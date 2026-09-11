# Android Design Import

Scope: Figma-to-Compose translation — parse Figma design files (nodes,
components, styles, constraints, assets), extract design tokens (colors,
typography, spacing, corner radius), map to Compose equivalents (Modifier,
Box, Row, Column, Text, theme, Material3), generate pixel-flavored Compose
UI code with proper semantics, validate translation fidelity against the
original design (BS §79.7; BS §50; ADR-225 ScreenModel).

## Trigger
This skill is requested when figma-to-Compose translation — parse Figma design files (nodes, components, styles, constraints, assets), extract design tokens (colors, typography, spacing, corner radius), map to Compose equivalents (Modifier, Box, Row, Column, Text, theme, Material3), generate pixel-flavored Compose UI code with proper semantics, validate translation fidelity against the original design (BS §79.7; BS §50; ADR-225 ScreenModel).. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `DESIGN_IMPORT`

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
- figma_api_or_local_file_parser
- vision_model_for_design_analysis
- compose_code_generator
- preview_renderer

## Procedure
1. Obtain the design source: either a Figma file (via access token and the
   Figma API) or a local design-export file (JSON, PDF, or image with
   vision-assisted extraction).
2. Parse the design structure: identify screens, components, variants,
   constraints, and layout grids.
3. Extract design tokens: colors (light/dark), typography (font family,
   size, weight, line height), spacing scales, corner radii, elevation,
   and semantic color roles.
4. Map to Compose equivalents: each Figma frame becomes a Composable
   function; each Figma component becomes a reusable Composable; Figma
   auto-layout becomes Row/Column/Box with Modifier spacing;
   Figma styles become MaterialTheme tokens.
5. Generate Compose code: produce pixel-flavored Kotlin that matches the
   design within the defined fidelity threshold.
6. Validate translation fidelity: render the generated Compose code in the
   Preview surface and compare against the original design using the
   ScreenModel/ScreenGraph pipeline (ADR-225). Measure structural
   similarity, color delta, typographic match, and spatial alignment.
7. Integrate into the project: place generated Composables in the workspace,
   update the BrandManifest/AssetManifest (BS §50.3), and refresh the
   preview.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `DesignImportResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Design import requires DESIGN_IMPORT; a missing Figma token or file is
  *   USER_REQUIRED, never a guessed layout.
  * Generated Compose code MUST pass through the normal `UI Worker` preview
  *   and validation pipeline before integration — design import alone does
  *   not satisfy the BrandAssetCompletionGate (BS §50.7).
  * Figma access tokens are stored per the credential rules (BS §8.4) and
  *   are never persisted in project records.
  * Translation fidelity is measured, not assumed — a generated screen that
  *   does not meet the fidelity threshold routes through RecoveryAuthority for
  *   repair, not silent acceptance.
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
Emits `DesignImportResult` from `DesignImportRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Obtain the design source: either a Figma file (via access token and the
- Step 2 produces its expected outcome — Parse the design structure: identify screens, components, variants,
- Step 3 produces its expected outcome — Extract design tokens: colors (light/dark), typography (font family,
- Step 4 produces its expected outcome — Map to Compose equivalents: each Figma frame becomes a Composable
- Step 5 produces its expected outcome — Generate Compose code: produce pixel-flavored Kotlin that matches the
- Step 6 produces its expected outcome — Validate translation fidelity: render the generated Compose code in the
- Step 7 produces its expected outcome — Integrate into the project: place generated Composables in the workspace,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
