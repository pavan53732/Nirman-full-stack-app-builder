# Android Authentication Flows

Scope: authentication and session flows in a generated Android application — credential handling, token acquisition and refresh, session expiry and revocation, and the failure states a user actually sees (BS §79.7).

## Trigger
An auth flow is designed or reviewed, or users are being logged out unexpectedly,
stuck in a refresh loop, or left in a state where the app looks signed in but is not.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The token types, lifetimes, and the revocation mechanism are known from the provider
  contract rather than observed from traffic.

## Context requirements
- The token types in use and their lifetimes.
- The endpoints that require authentication and those that do not.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether refresh token rotation is in force.

## Allowed tools
- static_analyzer
- auth_probe

## Procedure
1. Verify credentials are handled without ever being logged, cached in a plain store,
   or passed in a URL where they would be recorded.
2. Verify acquisition: the token request carries only what it needs, and the response is
   validated before it is trusted rather than stored on arrival.
3. Store tokens in secure storage, and confirm none is present in a plain store, a
   backup, or a log.
4. Verify refresh happens before expiry by a margin sufficient for a slow request,
   rather than after a failure.
5. Verify the refresh failure path: one retry, then a clean sign-out with a stated
   reason — never a silent loop and never an indefinite retry.
6. Verify rotation where it is in force: a reused refresh token is rejected and the
   session is treated as compromised rather than merely failed.
7. Verify expiry and revocation reach the UI: an expired or revoked session signs the
   user out and says why, rather than failing each request silently.
8. Confirm no endpoint relies on the client to enforce access; the server checks every
   authenticated request.

## Evidence
- Credential handling review, including a scan of logs and plain stores.
- Token storage location, and the scan of backups and logs.
- Refresh timing observed against expiry, with the margin measured.
- Refresh failure outcome, including the retry count and the user-visible result.
- Rotation behaviour on a reused token.
- UI states for expiry and revocation.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - No credential is present in a log, a plain store, or a URL.
  - An expired or revoked session signs the user out and says why, never leaving requests failing silently.
  - A refresh that fails exhausts a bounded number of attempts before signing out; it never retries indefinitely.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CREDENTIAL_EXPOSED — a credential or token was found in a log, URL, or plain
  store.
- REFRESH_LOOP — refresh retried without bound instead of signing out.
- SILENT_EXPIRY — an expired session kept failing requests without telling the user.
- REVOCATION_IGNORED — a revoked session continued to be treated as valid.

## Recovery
- An exposed credential is rotated and the storage path corrected, rather than relying
  on the log rotation to remove it eventually.
- A refresh loop is fixed with an explicit attempt bound and a clean sign-out, not by
  adding backoff until the user stops noticing.
- Silent expiry is fixed by surfacing the state in the UI, not by refreshing more
  aggressively in the background.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `AuthFlowResult` (§23 SkillPackage contract):

- credentialHandling: review and scan results
- tokenStorage: location and scan results
- refresh: timing margin, failure outcome, and retry count
- sessionStates: expiry and revocation as the user sees them
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured auth evidence

## Fixtures
- Credential never logged or stored in plain storage
- Refresh happens before expiry with a measured margin
- Refresh failure signs out cleanly with a reason
- Reused refresh token rejected and session treated as compromised
- Revoked session surfaces in the UI
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
