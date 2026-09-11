# Windows IPC Validation

Scope: Nirman's named-pipe transport — connection establishment and teardown, framed
message integrity, version negotiation between peers, peer disconnection behaviour,
and backpressure under load (BS §79.7).

## Trigger
The pipe layer must be proven, or a fault is traced to it: a failed connection,
a truncated or partially parsed message, a peer that vanished, or a suspected stall
under load.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `WINDOWS_NATIVE_EXECUTION` resolves to AVAILABLE, which needs a leased Windows
  ValidationEnvironment; without a lease there is no native-execution claim.
- The two peers under test are identified: which is the pipe server and which the
  client, and the pipe name each expects.

## Context requirements
- Pipe name, framing format, and the protocol version each peer speaks.
- The message sequences to exercise, including the largest and the malformed.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether the run is single-message or under sustained load.

## Allowed tools
- named_pipe_probe
- process_probe

## Procedure
1. Establish the connection and observe which side binds and which connects,
   recording the pipe name and the security descriptor actually applied.
2. Negotiate version first and prove that a peer speaking an older version is
   rejected explicitly rather than partially parsed.
3. Exchange framed messages and verify framing on the wire: length prefix, body,
   and that a message split across reads is reassembled.
4. Exercise the malformed cases: truncated frame, oversized frame, and a body that
   does not match its declared length; each must be rejected, never parsed
   optimistically.
5. Disconnect abruptly from each side and verify the other detects it and cleans up
   rather than blocking forever.
6. Apply sustained load and verify backpressure: the writer blocks or is refused
   rather than the buffer growing without bound.

## Evidence
- Connection record: pipe name, which side bound, security descriptor, and
  negotiated version.
- Per-message framing verification, including the split-across-reads case.
- The malformed-case results: each rejection and the reason recorded.
- Disconnection behaviour from each side, and the cleanup observed.
- Backpressure measurements under load: queue depth over time and the writer's
  behaviour at the bound.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CONNECTION_FAILED — the pipe could not be established, with which side failed
  and why.
- FRAMING_ERROR — a message was truncated, oversized, or mis-framed.
- VERSION_REJECTED_MALFORMEDLY — an older peer was partially parsed instead of
  rejected cleanly.
- HANG_ON_DISCONNECT — a peer did not detect disconnection and blocked.

## Recovery
- A hang is reported as a hang and the wait is bounded; the bound is never removed
  to make the test finish.
- A framing error is fixed in the framing layer, not by widening the parser to
  accept malformed input.
- A version mismatch is resolved by explicit negotiation, never by guessing the
  layout from the message body.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `IpcValidationResult` (§23 SkillPackage contract):

- connection: pipe, binder, security descriptor, negotiated version
- messagesVerified: count, with framing results
- malformedRejected: each case and its recorded reason
- disconnectBehaviour: detected and cleaned, per side
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured IPC evidence

## Fixtures
- Clean connection with version negotiated
- Message split across reads reassembled
- Truncated frame rejected
- Older-version peer rejected cleanly
- Abrupt disconnect detected and cleaned
- Sustained load bounded by backpressure

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
