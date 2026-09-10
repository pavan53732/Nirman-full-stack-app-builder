# Android Security Expert

Scope: Android security implementation — the Android key store system, BiometricPrompt,
EncryptedSharedPreferences, EncryptedFile, network security config,
certificate pinning, app signing (debug/release), Play App Signing,
and security best practices (BS §79.7). This skill provides the
security domain knowledge that the `Security Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Security Worker`
role — it provides security-specific instruction.

## Workflow
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

## Invariants
- Secrets are never hardcoded — use the Android key store system or user-provided
  keystores. The `Security Worker` scans for and rejects hardcoded
  secrets.
- Biometric authentication requires a fallback — not all devices have
  biometrics enrolled. Always provide device credential fallback.
- Network security config is mandatory for API 28+ — cleartext traffic
  is blocked by default on API 28+. Explicitly configure exceptions.
- Debuggable flag is false in release — the `Release Worker` MUST
  verify `android:debuggable="false"` in the release manifest.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
