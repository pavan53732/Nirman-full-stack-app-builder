# Environment Preflight

Scope: identify host and target; inspect toolchain, SDKs, runtimes, and
native dependencies; classify executable and validation capabilities;
produce the environment fingerprint (BS §79.7).

Runs before implementation. Gated by host tools only — it must never be
blocked by the classification it produces.

## Workflow
1. Observe the host platform, architecture, and every required tool
   (version strings, not assumptions).
2. Run the deterministic EnvironmentCapabilityPlanner against the
   observed tools for the declared target.
3. Persist the EnvironmentCapabilityRecord (durable, fingerprinted,
   superseding the previous record only when the environment identity
   changed).

## Invariants
- The model never sets or raises a capability state; the planner
  classifies from observation (CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION).
- A missing tool is reported as such — never silently substituted and
  never hard-coded as unavailable.
- Output is the record and its fingerprint; it is evidence, not a claim.
