# Windows Diagnostics Expert

Scope: Windows runtime diagnostics — ConPTY console hosting and stream
capture, Job Object isolation and resource limits, named-pipe transport
failures, process supervision and restart behavior, and collecting
event-log and crash-dump evidence (BS §79.7). This skill provides the domain
knowledge that the `Debugging Worker` and `Release Worker` consume.

Gated by WINDOWS_HOST_TOOLCHAIN and WINDOWS_NATIVE_EXECUTION. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked
state MUST be reported. This skill does not replace a worker role
— it provides domain-specific instruction that the worker executes
within its scoped asset transaction (BS §50).

## Workflow
1. Reproduce under observation: capture the exact command, working
   directory, environment, and account that produced the failure.
2. Separate the processes under test: the desktop application, the
   supervisor, and the workers — a symptom in one is frequently a cause
   in another.
3. Inspect the named-pipe transport first: connection establishment,
   framing, and which side closed and why.
4. Inspect ConPTY streams: confirm the console host is attached, that
   output is being drained, and that a full buffer is not the real fault.
5. Inspect Job Object state: which limits are configured, which were
   hit, which processes remain inside the job, and whether a termination
   was a limit kill.
6. Inspect process supervision: expected process count, restart count,
   and the reason recorded for each restart.
7. Collect the durable evidence: relevant event-log entries, structured
   log lines with their correlation identifiers, and a crash dump only
   where a crash is the failure.
8. State the finding with its evidence reference, and distinguish a
   diagnosed cause from a merely correlated symptom.

## Invariants
- A diagnosis is bound to evidence; a hypothesis without an evidence
  reference is reported as a hypothesis, not a cause.
- Observation is read-only with respect to the failure: reproducing a
  fault must not mutate the state that produced it.
- Correlation is not causation; a log line near a failure is not an
  explanation of it.
- No secret, credential, or user content is copied into a diagnostic
  artifact, a log, or a crash dump.
- A dump is collected only where a crash is the failure, and is
  recorded as an artifact with its retention rule.
- An unverified fix is reported as unverified; a restart that hides a
  fault is not a repair.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
