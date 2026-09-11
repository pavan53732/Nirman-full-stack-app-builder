# Windows Credential Security

Scope: credential handling on the Windows host — where a credential is stored and who can
read it, encryption at rest, retrieval and lifetime, provider API keys, and proving
no credential reaches a log, artifact, or diagnostic capture (BS §79.7).

## Trigger
A credential is introduced, stored, rotated, or suspected of leaking — or the host's
handling of secrets must be reviewed before a trust boundary is relied on.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The credentials in scope are known: which ones exist, what each is for, and where
  each is supposed to live.

## Context requirements
- The credential inventory: identifier, purpose, and intended store.
- The access control each store is supposed to enforce.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which logs, artifacts, and dumps are in scope for leak checking.

## Allowed tools
- credential_store_probe
- static_analyzer

## Procedure
1. Inventory the credentials and confirm each is in the store it belongs in, with
   none left in a file, an environment variable, or a source tree.
2. Verify access control on each store: only the intended principal can read it, and
   the worker's identity cannot.
3. Verify encryption at rest and that the key is held by the platform rather than
   stored beside the ciphertext.
4. Trace retrieval to lifetime: a credential is read when needed, held for the
   shortest workable time, and not cached longer than its purpose requires.
5. Check rotation: a credential can be replaced without a rebuild, and the old one
   stops working rather than lingering.
6. Search the leak surfaces — logs, artifacts, crash dumps, diagnostic captures — for
   each credential's value and for shapes that look like one.

## Evidence
- The credential inventory with the store each actually lives in.
- Access-control verification per store, including the worker's identity.
- Retrieval and lifetime observations per credential.
- Rotation result: replaced, and the old credential confirmed inert.
- Leak scan results across every surface in scope.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - Every credential lives in its intended store, with no copy in a file, an environment variable, or a source tree.
  - No credential value, nor its distinctive shape, appears in any log, artifact, or dump in scope.
  - A rotated credential replaces the old one, and the old one is confirmed inert.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- MISPLACED_CREDENTIAL — a credential lives outside its intended store.
- OVERBROAD_ACCESS — a principal that must not read the credential can.
- SECRET_IN_ARTIFACT — a credential value, or its distinctive shape, appears in a
  log, artifact, or dump.
- ROTATION_INCOMPLETE — the old credential still works after replacement.

## Recovery
- A leaked credential is rotated first and the leak path closed second; the rotation
  is never deferred until the leak is understood.
- A misplaced credential is moved to its store and the location it was found in is
  checked for other copies.
- Overbroad access is narrowed at the access control list rather than relying on the
  caller not to read it.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `CredentialSecurityResult` (§23 SkillPackage contract):

- inventory: credential, purpose, and actual store
- accessControl: who can read, including the worker identity
- leakScan: surfaces checked and findings
- rotation: replaced, old credential confirmed inert
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured credential evidence

## Fixtures
- Every credential in its intended store
- Worker identity cannot read the store
- Credential rotated and the old one inert
- Secret value found in a log
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
