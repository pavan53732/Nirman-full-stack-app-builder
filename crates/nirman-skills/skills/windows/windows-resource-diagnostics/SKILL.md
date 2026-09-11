# Windows Resource Diagnostics

Scope: diagnosing host resource problems — CPU, memory, and disk attributed by process,
unbounded growth across a session, disk exhaustion, and naming the contention source
rather than reporting that the host is slow (BS §79.7).

## Trigger
The host is slow, a build is starved, disk space ran out, or memory grew across a
session and the attributing process must be named.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- A measurement window is defined, so samples are comparable rather than isolated
  snapshots.

## Context requirements
- The symptom and when it started.
- The measurement window and sampling interval.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which processes are in scope for attribution.

## Allowed tools
- resource_probe
- process_probe

## Procedure
1. Establish the baseline: what CPU, memory, and disk the host reports while idle
   enough to be a reference, so later numbers have something to be compared against.
2. Attribute CPU by process across the measurement window, naming the top consumers
   rather than reporting a single system-wide figure.
3. Attribute memory by process and check the committed set against what is actually
   resident, so a leak is distinguished from a large but justified working set.
4. Sample memory across a sustained session and determine whether it grows without
   bound or plateaus; a plateau is not a leak.
5. Attribute disk usage and free space, and name the directories driving consumption
   rather than reporting only the total.
6. Identify contention: where two or more processes compete for the same resource,
   name the contenders and the resource, rather than reporting slowness alone.
7. Report per-process attribution with the measurement window stated, never a bare
   percentage.

## Evidence
- Idle-baseline figures for CPU, memory, and disk.
- Per-process CPU attribution across the stated window.
- Per-process memory attribution, with committed versus resident.
- Growth samples across the session, showing plateau or unbounded growth.
- Disk attribution by directory, and the contention sources named.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- ATTRIBUTION_FAILED — resource use could not be attributed to a process.
- UNBOUNDED_GROWTH — memory or disk grew without bound across the session.
- DISK_EXHAUSTED — free space fell below what the product needs to operate.
- CONTENTION_UNRESOLVED — slowness was observed but no contender was identified.

## Recovery
- Unbounded growth is bisected to the operation that introduces it, not mitigated by
  restarting the host periodically.
- Disk exhaustion is resolved by naming the consumer and reclaiming from it, not by
  lowering the free-space threshold.
- Unresolved contention is escalated with the samples taken, rather than reported as
  a slow host with no attribution.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `ResourceDiagnosticsResult` (§23 SkillPackage contract):

- baseline: CPU, memory, and disk at reference level
- attribution: per-process figures across the stated window
- growthTrend: sampled, with plateau or unbounded conclusion
- diskByDirectory: consumption attributed
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured resource evidence

## Fixtures
- Idle baseline captured
- CPU attributed to a specific process
- Memory plateaus across a sustained session
- Memory grows without bound across a session
- Disk exhaustion attributed to a directory
- Capability UNAVAILABLE — blocked, nothing measured

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
