# Android Credential Manager and Passkeys Expert

Scope: Passwordless and password-based sign-in through the Credential
Manager — passkey (FIDO2) creation and assertion, Sign in with Google,
federated and password credentials, autofill integration, credential
enumeration and recovery, and the origin and digital-asset-link binding
that makes a passkey valid (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `Security Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN and ANDROID_AUTHENTICATION. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- The server verifies every attestation and assertion; the client never
  decides authentication success.
- No credential, challenge, or token is written to logs, memory records,
  or crash reports.
- A missing Digital Asset Links binding is reported as a blocked
  configuration defect, never bypassed for convenience.
- Sign-out clears every locally cached credential and session artifact.
- Every authentication outcome is durable evidence: which credential
  type, which provider, and the verification result.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
