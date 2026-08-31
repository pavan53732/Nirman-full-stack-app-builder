# Environment Repair

Scope: authorized repairs — missing tool, wrong tool version, missing
target, broken PATH, missing SDK or dependency, incorrect configuration
(BS §79.7).

Gated by the repair capability plus policy approval through the normal
transaction path. The declared `environment.repair` request is evaluated
by the policy engine; loading this skill never grants it.

## Workflow
1. Take the repair action only from a REPAIRABLE classification in the
   current EnvironmentCapabilityRecord — never from a model's guess.
2. Execute the repair through the normal transaction path with durable
   evidence.
3. Re-run preflight: the new record supersedes the old one and the
   capability is re-classified deterministically.

## Invariants
- No repair may mark a capability AVAILABLE; the planner re-classifies
  from fresh observation.
- A rejected or failed repair is reported truthfully with the durable
  evidence reference; it is never retried beyond the declared budget.
