# Android Security Expert

Scope: Android security implementation — the Android key store system, BiometricPrompt,
EncryptedSharedPreferences, EncryptedFile, network security config,
certificate pinning, app signing (debug/release), Play App Signing,
and security best practices (BS §79.7). This skill provides the
security domain knowledge that the `Security Worker` consumes.

## Trigger
This skill is requested when android security implementation — the Android key store system, BiometricPrompt, EncryptedSharedPreferences, EncryptedFile, network security config, certificate pinning, app signing (debug/release), Play App Signing, and security best practices (BS §79.7). This skill provides the security domain knowledge that the `Security Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_AUTHENTICATION`
- `ANDROID_SIGNING_INSPECTION`

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
- android_keystore
- biometric_prompt
- network_security_config

## Procedure
1. Analyze security requirements: identify sensitive data (tokens,
   PII, credentials), authentication needs (biometric, PIN, password),
   network security requirements, and compliance needs.
2. Implement secure storage: use EncryptedSharedPreferences for
   key-value data, EncryptedFile for file data, the Android key store system
   for cryptographic keys. Never store secrets in plaintext.
3. Implement biometric authentication: use BiometricPrompt with
   CryptoObject for cryptographic operations, handle authentication
   errors gracefully, provide fallback to device credentials.
4. Configure network security: use `network-security-config.xml` to
   restrict cleartext traffic, pin certificates for production, and
   disable debug-overrides in release builds.
5. Handle app signing: use the Nirman-managed debug keystore for
   development, a user-provided release keystore for production.
   Document the signing configuration in the SigningIdentityBinding
   (BS §5.7.3).
6. Audit the app: use `Security Worker` to scan for hardcoded secrets,
   insecure network configurations, exported components without
   permissions, and debuggable flags in release builds.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `SecurityImplementationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Secrets are never hardcoded — use the Android key store system or user-provided
  *   keystores. The `Security Worker` scans for and rejects hardcoded
  *   secrets.
  * Biometric authentication requires a fallback — not all devices have
  *   biometrics enrolled. Always provide device credential fallback.
  * Network security config is mandatory for API 28+ — cleartext traffic
  *   is blocked by default on API 28+. Explicitly configure exceptions.
  * Debuggable flag is false in release — the `Release Worker` MUST
  *   verify `android:debuggable="false"` in the release manifest.
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

- SECRET_IN_SOURCE — a credential or key was found in source or in a resource file.
- CLEARTEXT_ALLOWED — cleartext traffic is permitted where the network policy forbids it.
- UNENCRYPTED_SENSITIVE_STORE — sensitive data is stored without the platform's encryption.

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
Emits `SecurityImplementationResult` from `SecurityImplementationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze security requirements: identify sensitive data (tokens,
- Step 2 produces its expected outcome — Implement secure storage: use EncryptedSharedPreferences for
- Step 3 produces its expected outcome — Implement biometric authentication: use BiometricPrompt with
- Step 4 produces its expected outcome — Configure network security: use `network-security-config.xml` to
- Step 5 produces its expected outcome — Handle app signing: use the Nirman-managed debug keystore for
- Step 6 produces its expected outcome — Audit the app: use `Security Worker` to scan for hardcoded secrets,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
