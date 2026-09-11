# UI Design System

Scope: design system consistency across Nirman app surfaces — tokens used instead of literal
values, components reused rather than re-implemented, spacing and typography on the
declared scale, and themes and dark mode correct (BS §79.7).

## Trigger
A surface is built or changed and must match the design system, or a review finds
literal values, one-off components, or a theme that breaks in dark mode.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The token set and component library in force are known, so the review compares
  against them rather than against a screenshot.

## Context requirements
- The token set and component library in force.
- The themes that must render correctly, including dark mode.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which surfaces are in scope for this review.

## Allowed tools
- static_analyzer
- ui_inspector

## Procedure
1. Scan for literal values where a token exists, and name each one with the token that
   should replace it rather than reporting a count.
2. Verify spacing and typography land on the declared scale, and name any value between
   steps.
3. Check component reuse: a control that already exists in the library is used rather
   than re-implemented, and any divergence is deliberate and recorded.
4. Verify each component's variants cover the states actually used: default, disabled,
   loading, error, and empty.
5. Render every surface in each theme, including dark mode, and confirm contrast holds
   and nothing depends on a colour that only works in one theme.
6. Verify state is visible without relying on colour alone, for every state a surface can
   show.
7. Verify the surface works at the largest supported text size and scale factor without
   clipping or overlap.

## Evidence
- Literal-value findings, each with the token that should replace it.
- Spacing and typography findings, with off-scale values named.
- Component reuse findings, including re-implemented controls.
- Variant coverage per component.
- Per-theme render results, including dark mode and contrast.
- Large-text and scaling results per surface.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- LITERAL_INSTEAD_OF_TOKEN — a token exists and a literal was used.
- OFF_SCALE_VALUE — spacing or typography does not land on the declared scale.
- COMPONENT_REIMPLEMENTED — a library control was rebuilt instead of used.
- THEME_BROKEN — a surface fails to render correctly in a supported theme.

## Recovery
- A literal value is replaced with the token, not with a new literal that happens to
  match the token today.
- A broken theme is fixed by using semantic tokens rather than by adding a theme-specific
  override for the failing surface.
- A re-implemented component is replaced with the library one, and any genuinely needed
  difference is added to the library rather than kept as a one-off.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `DesignSystemResult` (§23 SkillPackage contract):

- literalFindings: each value and the token that should replace it
- scaleFindings: off-scale spacing and typography
- reuseFindings: re-implemented components
- themes: render results per theme, including dark mode and contrast
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured design-system evidence

## Fixtures
- Surface uses tokens throughout
- Spacing and typography land on the declared scale
- Library component used rather than re-implemented
- Every theme renders with contrast holding
- Literal colour that breaks in dark mode
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
