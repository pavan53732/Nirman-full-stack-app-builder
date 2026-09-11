# Cross-Platform Build Diagnostics

Scope: for a host→target pair, determine what can be cross-built, which
artifacts can be produced, and which validation evidence necessarily
remains missing (BS §79.7).

Gated by HOST_TOOL_OBSERVATION. This skill's job is to state the gap
truthfully: it reports what is missing, it does not produce the missing
evidence.

## Workflow
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

## Invariants
- Missing evidence is named, never simulated.
- The report is the blocked-node input: it states the reason, the resume
  condition, and both §79.11 lists.
- A host-specific limitation is reported from observation, never
  hard-coded as universally unavailable.
- Cross-compilation success never establishes target-runtime capability;
  a target artifact that has not run on the target is unverified.
- The diagnosis binds to the environment fingerprint; a changed
  fingerprint voids it.
- This skill diagnoses only — it neither repairs the environment nor
  produces the evidence it reports missing.
