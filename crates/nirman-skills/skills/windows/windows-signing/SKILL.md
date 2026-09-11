# Windows Signing

Scope: code signing for release packages — whether a signature is valid and its chain
trusted, whether a timestamp is present and valid, whether the digest and publisher
identity match, and verifying every binary the installer ships (BS §79.7).

## Trigger
A release package must be signed, or a signature is rejected, expired, or the
publisher identity shown at install does not match what was expected.

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The package and the publisher identity it is expected to carry are known.
- The signing material is available to the signing step; where it is not, the step is
  blocked rather than skipped.

## Context requirements
- The binaries and the installer to sign.
- The expected publisher identity and the chain that must trust it.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether a timestamp service is reachable from the build host.

## Allowed tools
- signing_probe
- hash_verifier

## Procedure
1. Inventory every binary the installer ships, including helper executables and
   scripts, so nothing is signed selectively.
2. Sign each binary and the installer, and confirm a signature was actually applied
   rather than reported as applied.
3. Verify each signature and its chain: trusted root, valid purpose, and an unbroken
   chain to the publisher certificate.
4. Verify the digest the signature covers against the file as shipped, so a signed file
   that was later altered is detected.
5. Confirm a timestamp is present and valid, so a signature remains verifiable after
   the certificate expires.
6. Confirm the publisher identity shown at install matches the expected one, and that
   no binary in the package carries a different identity.
7. Run the verification on a clean host that has no signing material, so the chain is
   trusted by the platform rather than by local trust.

## Evidence
- The binary inventory, with the signing outcome for each.
- Per-signature chain verification: root, purpose, and chain integrity.
- Digest verification against each file as shipped.
- Timestamp presence and validity per signature.
- Publisher identity as shown at install, against the expected identity.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- UNSIGNED_BINARY — a shipped binary carries no signature.
- CHAIN_UNTRUSTED — the signing chain does not reach a trusted root.
- DIGEST_MISMATCH — the file as shipped differs from what was signed.
- TIMESTAMP_MISSING — a signature will not remain verifiable after expiry.

## Recovery
- An unsigned binary is signed before packaging, not removed from the package to make
  the verification pass.
- A digest mismatch invalidates the signature and requires re-signing; the file is
  never shipped on the strength of the old signature.
- A missing timestamp is fixed by using a timestamp service, and where none is
  reachable the release is blocked rather than shipped untimestamped.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `SigningResult` (§23 SkillPackage contract):

- inventory: each shipped binary and its signing outcome
- chainVerification: root, purpose, and chain integrity per signature
- digestVerification: per file as shipped
- timestamps: present and valid per signature
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured signing evidence

## Fixtures
- Every binary signed and chain-trusted
- Timestamp present and valid
- Digest matches the file as shipped
- Unsigned helper executable detected
- Signature invalid after the file was altered
- Capability UNAVAILABLE — blocked, nothing signed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
