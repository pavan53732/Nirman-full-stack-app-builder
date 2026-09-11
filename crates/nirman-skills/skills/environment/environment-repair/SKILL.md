# Environment Repair

Scope: authorized repairs — missing tool, wrong tool version, missing
target, broken PATH, missing SDK or dependency, incorrect configuration
(BS §79.7).

Gated by HOST_TOOL_OBSERVATION and ENVIRONMENT_REPAIR plus policy
approval through the normal transaction path. The repair request is
evaluated by the policy engine (BS §26.11); loading this skill never
grants it.

## Workflow
1. Start only from a REPAIRABLE classification in the current
   EnvironmentCapabilityRecord — never from a model's guess, and never
   from a capability already classified UNAVAILABLE.
2. Read the repair target precisely: which capability, which tool, the
   observed version, and the required version range.
3. Submit the repair for policy admission before touching anything, and
   record the admission decision as durable evidence.
4. Choose the smallest repair that restores the capability: install a
   missing tool, align a version into its range, add a missing target,
   correct search-path order, or fix one configuration value.
5. Execute through the normal transaction path, capturing what changed,
   from what value to what value, and the exact action that changed it.
6. Re-run preflight: the new record supersedes the old one and the
   capability is re-classified deterministically from fresh observation.
7. Bound retries by the recoveryAttemptPolicy — an identical repair is
   never re-run against unchanged evidence, and a materially different
   repair escalates rather than terminating the goal.
8. Report the outcome truthfully, including a repair rejected by policy,
   one that failed, and one that left the capability unchanged.

## Invariants
- No repair may mark a capability AVAILABLE; the planner re-classifies
  from fresh observation.
- A repair never downgrades, substitutes, or pins around a tool to make a
  version check pass; it brings the tool into its required range.
- UNAVAILABLE is not repairable — a host that cannot provide a capability
  is reported with its reason, not repaired around.
- A repair never writes credentials, never disables verification, and
  never weakens a security control in order to restore a capability.
- The repair is scoped to the named capability; changing unrelated
  toolchain state is a defect, not a side effect.
- A rejected or failed repair is reported truthfully with the durable
  evidence reference, and the blocked node names its resume condition.
- Repair actions are evidence: every action, admission decision, and
  resulting classification is recorded, never asserted after the fact.
