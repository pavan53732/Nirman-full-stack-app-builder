# Rust Control Plane Expert

Scope: Rust control-plane engineering — crate layout and workspace
boundaries, async runtime discipline, typed error handling, named-pipe
interprocess communication, structured logging and diagnostics, and Rust
test and static-analysis practice (BS §79.7). This skill provides the domain
knowledge that the `Architecture Worker` and `Debugging Worker` consume.

Gated by WINDOWS_HOST_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked
state MUST be reported. This skill does not replace a worker role
— it provides domain-specific instruction that the worker executes
within its scoped asset transaction (BS §50).

## Workflow
1. Keep crate boundaries honest: one crate per responsibility,
   dependencies pointing one way, and no cycle between crates.
2. Put every long-running or concurrent path on an explicit async runtime
   and name it; nothing spawns a detached task that nobody owns.
3. Cancel and bound deliberately: every spawned task is joinable, every
   wait has a bound, and shutdown is a coordinated sequence rather than a
   process kill.
4. Model failure as typed errors with context, not as strings, so a
   caller can distinguish retryable from terminal without parsing a
   message.
5. Build interprocess communication on named pipes with framed messages,
   explicit versioning, and defined behavior for peer disconnection.
6. Log structurally: stable event names, typed fields, and correlation
   identifiers — never secrets, tokens, or user content.
7. Test at the boundary that matters: unit tests for pure logic,
   integration tests at the pipe and process boundaries, and static
   analysis in the build rather than in review.
8. Verify on the host: build with static analysis enabled, run the suite,
   and confirm the binaries start, connect, and shut down cleanly.

## Invariants
- No unwrap or panic on a reachable error path in control-plane code; a
  panic in a long-lived process is a defect, not an error-handling
  strategy.
- Every task is owned and joinable; no task outlives its owner silently.
- Every wait is bounded; an unbounded wait is a hang waiting to happen.
- Named-pipe framing is versioned; a peer speaking an older version is
  rejected explicitly, never partially parsed.
- No secret, credential, or user content is written to a log, a memory
  record, or a crash report.
- An unverified path is reported as unverified; a successful compile is
  not evidence of correct concurrent behavior.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
