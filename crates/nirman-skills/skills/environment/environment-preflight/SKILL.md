# Environment Preflight

Scope: identify host and target; inspect toolchain, SDKs, runtimes, and
native dependencies; classify executable and validation capabilities;
produce the environment fingerprint (BS §79.7).

Runs before implementation. Requires no §79.7 capability — it produces
the classification the other skills consume and must never be blocked
by it.

## Workflow
1. Observe the host: operating system and version, CPU architecture,
   available memory and disk, and whether hardware virtualization is
   present and usable by a hypervisor.
2. Enumerate every required tool by probing it and capturing an observed
   version string — SDKs, compilers, runtimes, package managers, and
   platform tools. A version declared by the repository is not evidence.
3. Detect the configuration that changes behavior: search-path order,
   SDK roots, environment overrides, and whether a proxy sits in the
   network path.
4. Classify the network path as DIRECT, SYSTEM_PROXY, or PAC from the
   host's own configuration. Report an authenticating or intercepting
   proxy by name and observed status; never prompt for and never store
   proxy credentials (TA §49.4).
5. Run the deterministic EnvironmentCapabilityPlanner against the
   observed facts for the declared target, producing one classification
   per capability id of the §79.3 matrix.
6. Record each capability as AVAILABLE, REPAIRABLE, USER_REQUIRED, or
   UNAVAILABLE, with the reason and the observation that produced it.
7. Persist the EnvironmentCapabilityRecord — durable and fingerprinted,
   superseding the previous record only when the environment identity
   actually changed.
8. Publish the fingerprint so every later artifact, observation, and
   evidence record can bind to it, and report blocked capabilities with
   their resume conditions rather than as failures of the goal.

## Invariants
- The model never sets or raises a capability state; the planner
  classifies from observation
  (CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION).
- A missing tool is reported as such — never silently substituted and
  never hard-coded as unavailable.
- Host environment, target platform, validation platform, and
  certification status stay distinct and are never collapsed into one
  build, validation, or completion result
  (CLAUSE.PLATFORM.HOST_TARGET_SEPARATION).
- Compiling on the host, or cross-compiling for a target, never
  establishes native target-runtime capability, runtime validation, or
  certification (CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE).
- Containers, virtual machines, the Windows subsystem for Linux, and
  simulated or remote environments never substitute for the declared
  target's native validation (CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET).
- Preflight is read-only: it inspects and classifies, and it never
  repairs, installs, or mutates the environment it observes.
- Output is the record and its fingerprint; it is evidence, not a claim.
