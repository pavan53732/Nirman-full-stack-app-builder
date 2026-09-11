# Windows Preview Rendering

Scope: preview rendering on the Windows host — the health of the frame stream and the
staleness contract, resize and DPI handling, recovering a detached or lost surface,
and telling a frozen frame apart from a live surface (BS §79.7).

## Trigger
The preview is frozen, stale, blank, or a different size than expected — or frame
health must be proven before the preview is relied on.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The surface contract is known: how stale a displayed frame may be before it must be
  marked stale rather than shown.

## Context requirements
- The surface contract, including the staleness bound.
- The display configuration: scale factor and size.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether the preview is expected to stream continuously or update on change.

## Allowed tools
- preview_probe
- resource_probe

## Procedure
1. Confirm the frame stream is arriving and record the frame timestamps, so a frozen
   stream is distinguishable from a slow one.
2. Compare the newest frame timestamp against the staleness bound and mark the surface
   stale when the bound is exceeded, rather than continuing to present it as live.
3. Verify the displayed frame matches the current application state, so a stale frame
   is never presented as current.
4. Exercise resize and DPI change and confirm the surface re-renders at the new size
   rather than stretching or clipping.
5. Recovery-test the surface: detach it and confirm reattachment restores the stream
   and resumes frames.
6. Record every gap: a rendered frame is displayed, or a placeholder explaining why
   rendering is unavailable — never a perpetual loading indicator.

## Evidence
- Frame timestamps, and whether the stream is advancing or frozen.
- Staleness determination against the declared bound.
- State-match verification between the displayed frame and application state.
- Resize and DPI results, with the re-rendered sizes.
- Reattachment outcome, and every gap with its stated reason.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- STREAM_STALLED — frames stopped arriving; the surface is marked stale, not shown as
  live.
- STALE_FRAME_PRESENTED — a frame older than the bound was displayed as current.
- SURFACE_LOST — the surface detached and did not reattach.
- RENDER_MISMATCH — the displayed frame does not match application state.

## Recovery
- A stalled stream is marked stale and the reason is surfaced; the staleness bound is
  never extended to keep the surface looking live.
- A lost surface is reattached and frame arrival is re-proven, rather than leaving the
  last frame displayed as if it were current.
- A mismatch is fixed in the render path, not by refreshing until the frames happen to
  agree.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `PreviewRenderingResult` (§23 SkillPackage contract):

- streamHealth: frame timestamps, advancing or stalled
- staleness: age of the newest frame against the bound
- surfaceState: attached, stale, or lost, with reason
- resizeResults: sizes re-rendered
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured preview evidence

## Fixtures
- Frames stream and advance
- Staleness bound exceeded — surface marked stale
- Resize re-renders at the new size
- Detached surface reattaches and resumes
- Displayed frame matches application state
- Capability UNAVAILABLE — blocked, nothing measured

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
