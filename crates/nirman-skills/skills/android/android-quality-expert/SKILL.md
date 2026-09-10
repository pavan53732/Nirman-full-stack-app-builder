# Android Quality Expert

Scope: Android code quality — Android Lint, Detekt, Ktlint, code smell
detection, static analysis enforcement, and coding standard compliance
(BS §79.7). This skill provides the quality domain knowledge that the
`Security Worker` and `Reconciliation Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Security Worker`
role — it provides quality-specific instruction.

## Workflow
1. Analyze quality requirements: identify coding standards, lint rules,
   and static analysis tools to apply.
2. Configure Android Lint: enable `abortOnError = true` for CI, disable
   irrelevant checks, and create custom lint rules for project-specific
   patterns. Run `./gradlew lint` on every build.
3. Configure Detekt: define detekt.yml with rule thresholds
   (complexity, long classes, function length), enable auto-correct
   for safe fixes, and run `./gradlew detekt` on every build.
4. Configure Ktlint: define .editorconfig for code style, enable
   ktlintFormat for auto-formatting, and run `./gradlew ktlintCheck`
   on every build.
5. Enforce quality gates: fail the build on lint errors, Detekt
   threshold breaches, or Ktlint violations. The QualityGate
   (CAP.ANDROID.QUALITY_GATE) MUST pass before artifact promotion.
6. Document quality decisions: record which rules are disabled and why
   in the ReasoningArtifact (BS §66.2).

## Invariants
- Quality gates are enforced in CI — lint, Detekt, and Ktlint run on
   every build. Failures block artifact promotion.
- Custom rules are documented — any disabled rule or custom rule has
   a documented rationale in the ReasoningArtifact.
- Auto-correct is preferred — Ktlint and Detekt auto-fix are applied
   before manual review. Only unfixable issues surface to the user.
- Quality is measured, not assumed — the `Security Worker` runs static
   analysis and reports findings as evidence, not as model claims.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
   execution still passes through ToolBroker and PolicyAuthority.
