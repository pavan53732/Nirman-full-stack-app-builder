# Windows File System

Scope: host filesystem behaviour for Nirman workspaces — path handling and length limits,
the per-user workspace root and its access control, long-path and Unicode handling,
write atomicity and file locking, and path hygiene (BS §79.7).

## Trigger
A workspace path fails, a write is lost or truncated, two writers collide, or the
workspace layout and its access control must be verified.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The workspace root for the invoking account is known from the configuration, not
  assumed to be a default location.

## Context requirements
- The workspace root and the layout expected beneath it.
- Which paths the configuration declares, including any near the length limit.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The accounts that must and must not reach the workspace.

## Allowed tools
- filesystem_probe
- path_analyzer

## Procedure
1. Resolve the workspace root for the invoking account and confirm it matches the
   configuration rather than a profile default.
2. Verify the layout beneath the root: workspaces and toolchain directories where the
   configuration places them.
3. Verify access control: the invoking account can reach its own workspace and no
   other account can.
4. Test the longest path the product can generate end to end, rather than assuming the
   limit is not reached in practice.
5. Test Unicode and non-ASCII path components through create, read, and delete.
6. Verify write atomicity: a write is committed completely or not at all, so an
   interrupted write never leaves a half-written file that reads as valid.
7. Verify locking behaviour when two writers target the same file: one wins, the other
   is refused or waits, and neither silently loses data.

## Evidence
- The resolved workspace root and the observed layout beneath it.
- Access-control verification for the invoking and other accounts.
- Longest-path test result, with the length tested.
- Unicode path results across create, read, and delete.
- Atomicity and locking outcomes, including the interrupted-write case.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- ROOT_MISMATCH — the resolved root differs from the configured one.
- ACCESS_TOO_BROAD — an account that must not reach the workspace can.
- PATH_TOO_LONG — a generated path exceeds what the platform accepts.
- WRITE_NOT_ATOMIC — an interrupted write left a file that reads as valid.

## Recovery
- A too-long path is fixed by shortening the generated layout, not by relying on a
  long-path opt-in that may not be enabled on the host.
- A non-atomic write is fixed with a write-then-rename, not by checking the file size
  afterwards.
- Overbroad access is corrected on the directory, not by trusting callers to stay out.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `FileSystemResult` (§23 SkillPackage contract):

- resolvedRoot and observedLayout
- accessControl: who can reach the workspace
- longestPathTested: length and outcome
- atomicityAndLocking: outcomes, including interrupted writes
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured filesystem evidence

## Fixtures
- Workspace root resolves as configured
- Other accounts cannot reach the workspace
- Longest generated path succeeds end to end
- Unicode path components survive
- Interrupted write leaves no half-valid file
- Two writers to one file handled

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
