# Android Design Import

Scope: Figma-to-Compose translation — parse Figma design files (nodes,
components, styles, constraints, assets), extract design tokens (colors,
typography, spacing, corner radius), map to Compose equivalents (Modifier,
Box, Row, Column, Text, theme, Material3), generate pixel-flavored Compose
UI code with proper semantics, validate translation fidelity against the
original design (BS §79.7; BS §50; ADR-225 ScreenModel).

Gated by ANDROID_BUILD_TOOLCHAIN and DESIGN_IMPORT (a valid Figma
access token or a local design file). When either resolves to UNAVAILABLE
or USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. The design-import skill does not replace the `UI Worker`
role — it provides the design-intent input that the UI Worker consumes.

## Workflow
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

## Invariants
- Design import requires DESIGN_IMPORT; a missing Figma token or file is
  USER_REQUIRED, never a guessed layout.
- Generated Compose code MUST pass through the normal `UI Worker` preview
  and validation pipeline before integration — design import alone does
  not satisfy the BrandAssetCompletionGate (BS §50.7).
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
- Figma access tokens are stored per the credential rules (BS §8.4) and
  are never persisted in project records.
- Translation fidelity is measured, not assumed — a generated screen that
  does not meet the fidelity threshold routes through RecoveryAuthority for
  repair, not silent acceptance.
