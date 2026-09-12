# Android Credential Manager and Passkeys Expert

Scope: Passwordless and password-based sign-in through the Credential
Manager — passkey (FIDO2) creation and assertion, Sign in with Google,
federated and password credentials, autofill integration, credential
enumeration and recovery, and the origin and digital-asset-link binding
that makes a passkey valid (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `Security Worker` consume.

## Trigger
This skill is requested when passwordless and password-based sign-in through the Credential Manager — passkey (FIDO2) creation and assertion, Sign in with Google, federated and password credentials, autofill integration, credential enumeration and recovery, and the origin and digital-asset-link binding that makes a passkey valid (BS §79.7). This skill provides the domain knowledge that the `Android Data and Integration Worker` and `Security Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_AUTHENTICATION`
- `ANDROID_EMULATOR_EXECUTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any
  resolves to UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT
  execute and the blocked state MUST be reported.
- The source revision and, where one exists, the artifact digest are
  known, so every record this skill emits can be bound to them.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- Revision, and artifact digest where an artifact exists, plus the
  environment fingerprint of the session.
- The evidence identifiers this run must bind to, so results attach to
  the same evidence graph as the work that preceded them.

## Allowed tools
- credential_manager
- managed_emulator

## Procedure
1. Decide the credential surface: passkey first, with password and
   federated options retained for accounts that have no passkey yet.
2. Bind the app to its relying-party origin: publish the Digital Asset
   Links JSON at the well-known path, and verify the origin and package
   fingerprint resolve before any passkey is created.
3. Implement passkey creation: build a CreatePublicKeyCredentialRequest
   from the server's registration challenge, require user verification
   where the policy demands it, and post the attestation to the server
   for verification and storage.
4. Implement passkey assertion: request credentials for the relying
   party, let the user select one when several match, and send the
   assertion to the server for signature verification.
5. Handle the failure vocabulary truthfully: no credentials available,
   user cancelled, no suitable provider, and provider failure are distinct
   outcomes and are surfaced distinctly, never collapsed into one error.
6. Offer credential creation after a successful password sign-in, so an
   existing account upgrades to a passkey without a separate flow.
7. Integrate autofill: annotate the sign-in fields and let Credential
   Manager fill them, so the flow works even where the bottom sheet is
   dismissed.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `CredentialFlowResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * The server verifies every attestation and assertion; the client never
  *   decides authentication success.
  * No credential, challenge, or token is written to logs, memory records,
  *   or crash reports.
  * A missing Digital Asset Links binding is reported as a blocked
  *   configuration defect, never bypassed for convenience.
  * Sign-out clears every locally cached credential and session artifact.
  * Every authentication outcome is durable evidence: which credential
  *   type, which provider, and the verification result.
- Every claim reduced to an observable: what was seen, on which device or
  host, at which revision — never a statement of intent.

## Failure classification
- BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- PRECONDITION_UNMET — a precondition below was not satisfied; the skill
  does not proceed past it.
- ACTION_FAILED — a procedure step was attempted and did not produce its
  expected outcome.
- INVARIANT_VIOLATED — the work completed but one of the invariant claims
  this skill must leave observable does not hold.
- TIMEOUT — a wait exceeded its bound; an unbounded wait is a hang, not a
  slow step.

- ORIGIN_UNVERIFIED — the relying-party origin is not bound, so a credential cannot be trusted to this app.
- CREDENTIAL_FAILURE_MISREPORTED — a cancelled or absent credential was reported as an error, or an error as a cancellation.
- AUTOFILL_UNBOUND — sign-in fields are not annotated, so Credential Manager cannot fill them.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `CredentialFlowResult` from `CredentialFlowResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Decide the credential surface: passkey first, with password and
- Step 2 produces its expected outcome — Bind the app to its relying-party origin: publish the Digital Asset
- Step 3 produces its expected outcome — Implement passkey creation: build a CreatePublicKeyCredentialRequest
- Step 4 produces its expected outcome — Implement passkey assertion: request credentials for the relying
- Step 5 produces its expected outcome — Handle the failure vocabulary truthfully: no credentials available,
- Step 6 produces its expected outcome — Offer credential creation after a successful password sign-in, so an
- Step 7 produces its expected outcome — Integrate autofill: annotate the sign-in fields and let Credential
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
