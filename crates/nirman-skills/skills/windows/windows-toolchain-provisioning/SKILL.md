# Windows Toolchain Provisioning

Scope: provisioning the host toolchain — downloader behaviour and integrity verification,
proxy discovery and classification, archive validation, the per-user install
location, and resuming an interrupted download (BS §79.7).

## Trigger
A toolchain component must be fetched, or a download failed, stalled behind a proxy,
or failed integrity and the reason must be established rather than guessed.

## Required capabilities
- `HOST_TOOL_OBSERVATION`
- `ENVIRONMENT_REPAIR`
- `WINDOWS_HOST_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- `ENVIRONMENT_REPAIR` and `HOST_TOOL_OBSERVATION` resolve to AVAILABLE for any step
  that changes the host.
- The component, version, and expected digest are known from the manifest.

## Context requirements
- The component and version to provision, and its expected digest.
- The network path the downloader will take, and whether a proxy is expected.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- The per-user install location the configuration declares.

## Allowed tools
- downloader
- hash_verifier
- proxy_probe

## Procedure
1. Discover the network path from the host's own configuration — WinHTTP, then the
   signed-in user's Internet Options — and classify it as DIRECT, SYSTEM_PROXY, or
   PAC, recording which was used.
2. Read no proxy credentials from an environment variable, and never prompt for them;
   an authenticating proxy is a blocked state, not a prompt.
3. Detect an authenticating or intercepting proxy and report it by host and observed
   status as WAITING_NETWORK rather than as a download failure.
4. Download to the per-user toolchain location the configuration declares, never into a
   profile or a synced folder.
5. Verify the archive against its expected digest before it is unpacked; a mismatch is
   FAILED_INTEGRITY and the archive is not used.
6. Resume an interrupted download where the server supports it, and record that the
   resume happened rather than restarting silently.
7. Record the provisioning outcome on the toolchain record, including the network path
   taken and the host that served it.

## Evidence
- The network path classification and how it was discovered.
- The download record: source, bytes, duration, and whether a resume occurred.
- The digest verification result against the expected value.
- The install location used, and the per-user access control applied to it.
- Every record bound to revision and host environment fingerprint.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- WAITING_NETWORK — an authenticating, intercepting, or captive portal proxy blocked
  the download; reported with the host and observed status.
- FAILED_INTEGRITY — the archive digest did not match; the archive is rejected.
- PROXY_CREDENTIAL_REQUIRED — credentials were demanded; reported, never prompted for
  and never stored.
- LOCATION_INVALID — the install location is not the configured per-user root.

## Recovery
- A network wait resumes on a successful probe rather than being retried on a timer;
  the blocked node names the host and the observed status.
- A failed integrity check re-downloads from the source and re-verifies; the digest is
  never relaxed to let the archive through.
- A credential-demanding proxy is escalated to the user as a configuration decision,
  because this skill does not handle credentials.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `ProvisioningResult` (§23 SkillPackage contract):

- networkPath: DIRECT, SYSTEM_PROXY, or PAC, and how it was discovered
- download: source, bytes, duration, resumed
- integrityVerified: boolean against the expected digest
- installLocation: path and access control applied
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured provisioning evidence

## Fixtures
- Direct download verifies and installs
- Explicit proxy discovered and recorded
- Authenticating proxy reported as WAITING_NETWORK
- Altered archive rejected on digest mismatch
- Interrupted download resumed
- Capability UNAVAILABLE — blocked, nothing provisioned

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
