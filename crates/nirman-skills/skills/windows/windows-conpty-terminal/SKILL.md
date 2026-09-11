# Windows ConPTY Terminal

Scope: Nirman's ConPTY console hosting — session creation and attachment, output stream
draining and buffer health, resize and encoding, exit code and error propagation,
and diagnosing a console that stalls or garbles (BS §79.7).

## Trigger
A terminal session misbehaves: output stops, arrives garbled, truncates, or the
exit code does not reach the caller — or terminal hosting must be proven before it
is relied on.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The hosted command and its expected output shape are known.

## Context requirements
- The command being hosted, its working directory, and its environment.
- The expected output shape and encoding.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether the session is interactive or a one-shot command.

## Allowed tools
- conpty_probe
- process_probe

## Procedure
1. Create the session and confirm the console host attached, recording the
   pseudo-console and pipe handles actually created.
2. Read output continuously rather than after completion, so a full buffer is
   distinguishable from a finished command.
3. Verify draining: output is consumed at least as fast as the child produces it, and
   the buffer does not grow without bound.
4. Exercise resize and confirm the child is notified and reflows rather than clipping
   or wrapping into garbage.
5. Verify encoding end to end, so wide characters and escape sequences survive rather
   than being mangled by a code-page mismatch.
6. Let the command exit and confirm the exit code reaches the caller intact,
   including a non-zero code and a signal-terminated run.
7. Diagnose a stall by layer: not started, started but not attached, attached but not
   drained, or drained but never exiting.

## Evidence
- Session record: handles created, attachment confirmed, and command started.
- Output captured with timing, and the observed buffer depth over the run.
- Resize and encoding results, including a wide-character case.
- Exit propagation: the exit code the caller received versus the child's.
- For a stall, the layer the diagnosis landed on.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- ATTACH_FAILED — the console host did not attach to the session.
- BUFFER_STALL — output was produced but not drained; the buffer filled.
- ENCODING_MANGLED — output arrived with corrupted characters or sequences.
- EXIT_LOST — the child's exit code did not reach the caller.

## Recovery
- A stall is fixed by draining continuously, not by enlarging the buffer to delay it.
- A mangled encoding is fixed at the code page in use, not by stripping the
  characters that do not decode.
- A lost exit code is fixed in the propagation path; the caller never infers success
  from a missing code.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `ConPtyResult` (§23 SkillPackage contract):

- session: handles, attachment, and command started
- outputBytes and observedBufferDepth over the run
- exitCodePropagated: the code the caller received
- stallLayer: where a stall was diagnosed
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured terminal evidence

## Fixtures
- Session attaches and output drains
- Long-running command with continuous draining
- Resize reflows correctly
- Wide characters survive intact
- Non-zero exit code reaches the caller
- Capability UNAVAILABLE — blocked, nothing hosted

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
