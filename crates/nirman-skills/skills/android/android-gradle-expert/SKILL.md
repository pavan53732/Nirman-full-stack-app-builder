# Android Gradle Expert

Scope: Android Gradle build system — version catalogs (libs.versions.toml),
convention plugins, build variants (debug/release/staging), signing
config, ProGuard/R8 rules, dependency resolution, and build optimization
(BS §79.7). This skill provides the build-system domain knowledge that
the `Release Worker` and ToolchainAuthority consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Release Worker`
role — it provides Gradle-specific instruction.

## Workflow
1. Analyze build requirements: identify build variants (debug, release,
   staging), dependency groups, and build optimization needs.
2. Set up version catalogs: define libs.versions.toml with versions,
   libraries, and bundles. Use libs.android.gradle.plugin syntax for
   plugins, libs.bundles.compose for grouped dependencies.
3. Create convention plugins: use `build-logic` module with
   convention.gradle.kts files for Android library, Android app,
   and Compose configuration. Avoid duplicating build logic across modules.
4. Configure build variants: define debug, release, and optional
   staging variants with different application IDs, signing configs,
   and build config fields.
5. Configure ProGuard/R8: define `proguard-rules.pro` for release builds,
   keep rules for reflection-based libraries, and test with
   `minifyEnabled = true` on debug for early detection.
6. Optimize build: enable build cache, configuration cache, parallel
   execution, and non-transitive R classes. Use Gradle build scans
   for bottleneck identification.

## Invariants
- Version catalogs are the single source of truth — dependencies are
   referenced via `libs.*`, never hardcoded as `group:artifact:version`.
- Convention plugins are used for shared build logic — each module
   applies a convention plugin, not raw `android {}` blocks.
- Signing config is separate from build logic — the `Release Worker`
   manages signing identity, not the Gradle script.
- ProGuard/R8 rules are tested — release builds are tested on the
   emulator to catch reflection/serialization issues.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
   execution still passes through ToolBroker and PolicyAuthority.
