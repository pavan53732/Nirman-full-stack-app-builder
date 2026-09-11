# Android UI List Performance

Scope: list and scroll performance in a generated Android application — view recycling and stable keys, item work kept off the scroll path, image loading and decode cost, and frame health measured under scroll (BS §79.7).

## Trigger
A list is built or changed, scrolling stutters, or the memory a long list consumes
grows as the user scrolls.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- A measurement method and a target frame budget are agreed before any change, so an
  improvement is measured rather than felt.

## Context requirements
- The list, its item shape, and the largest realistic item count.
- The frame budget in force and how frames are measured.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether images are involved and at what sizes.

## Allowed tools
- static_analyzer
- frame_profiler

## Procedure
1. Measure the baseline: frame times while scrolling the list over its realistic
   maximum, recorded rather than estimated.
2. Confirm the list recycles its views, so a long list holds a bounded number of item
   views rather than one per record.
3. Confirm keys are stable and derived from the record's identity, not from its position,
   so reordering and removal do not reuse the wrong view state.
4. Move per-item work off the scroll path: parsing, formatting, measuring, and layout are
   done before the frame that needs them, not during it.
5. Check item layout depth and confirm a shallow item does not nest layouts that must be
   measured repeatedly per frame.
6. Verify images are decoded at the size they are displayed and cached, so scrolling does
   not decode the same image repeatedly.
7. Re-measure after each change over the same list and the same scroll, and report
   against the baseline rather than against a target alone.
8. Verify memory stays bounded across a long scroll rather than growing with the number
   of records visited.

## Evidence
- Baseline frame times over the stated list and scroll.
- Recycling verification: views held against records in the list.
- Key stability results, including a reorder case.
- Item work identified on the scroll path, per item.
- Image decode sizes against display sizes, and cache hit behaviour.
- Post-change frame times against the baseline, and memory across a long scroll.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - The list holds a bounded number of item views regardless of record count.
  - Item identity derives from the record, not from its position.
  - No per-item work runs during the frame that needs it.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- NO_RECYCLING — the list holds a view per record rather than recycling.
- UNSTABLE_KEYS — item identity derives from position, so reordering reuses view state.
- WORK_ON_SCROLL_PATH — per-item work runs during the frame that needs it.
- MEMORY_UNBOUNDED — memory grows with the records visited rather than staying bounded.

## Recovery
- Jank is fixed by removing work from the scroll path, not by lowering the frame target
  until the measurement passes.
- Unstable keys are fixed by keying on record identity, not by clearing more view state
  on reuse to hide the mismatch.
- Unbounded memory is fixed in the recycling or the image cache, not by reducing the
  page size so users scroll less.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `ListPerformanceResult` (§23 SkillPackage contract):

- baselineFrameTimes: measured over the stated list and scroll
- recycling: views held against records
- keys: stability results including the reorder case
- scrollPathWork: per-item work found on the path
- images: decode size against display size, and cache behaviour
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured list evidence

## Fixtures
- Long list recycles views and stays bounded
- Reorder reuses the correct view state
- Per-item formatting moved off the scroll path
- Images decoded at display size and cached
- Frame times improve against the measured baseline
- Capability UNAVAILABLE — blocked, nothing measured

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
