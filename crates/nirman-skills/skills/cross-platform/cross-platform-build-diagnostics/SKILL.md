# Cross-Platform Build Diagnostics

Scope: for a host→target pair, determine what can be cross-built, which
artifacts can be produced, and which validation evidence necessarily
remains missing (BS §79.7).

Gated by host toolchain observation. This skill's job is to state the
gap truthfully: it reports what is missing, it does not produce the
missing evidence.

## Workflow
1. Read the current EnvironmentCapabilityRecord for the pair.
2. Classify: cross-buildable or not; list producible artifacts.
3. List the validation evidence that necessarily remains absent for the
   pair (native runtime, native installer behavior, device-specific
   checks).

## Invariants
- Missing evidence is named, never simulated.
- The report is the blocked-node input: it states the reason, the resume
  condition, and both §79.11 lists.
- A host-specific limitation is reported from observation, never
  hard-coded as universally unavailable.
