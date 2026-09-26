# Android Security Audit

Scope: auditing the finished Android artifact and its source for the defects
that implementation leaves behind — secrets in source and resources, exported
components without permissions, cleartext traffic policy, debuggable release
flags, network-security posture, and keystore usage — and emitting an
aggregate audit with per-finding severity and the evidence behind each
(BS §79.7). This skill supplies the audit instruction that the `Security
Worker` and the `Critic Worker` execute inside their scoped transactions
(BS §50). It audits; it never fixes, never softens a finding, and never
signs off by assertion.

## Trigger
This skill is requested when the finished app must be audited rather than
built: review the artifact and its source for the release-blocking security
defects. It is requested by security_release_audit, by
export_surface_review, and by signing_posture_review. It does not replace a
worker role — it supplies the audit instruction the worker executes inside
its scoped asset transaction (BS §50).

## Required capabilities
- `ANDROID_SOURCE_ENGINEERING`
- `ANDROID_AUTHENTICATION`
- `ANDROID_SIGNING_INSPECTION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and is not stale;
  capability is read from the record, never assumed.
- Every capability listed above resolves to AVAILABLE. When any resolves to
  UNAVAILABLE or USER_REQUIRED, the gated steps MUST NOT execute and the
  blocked state MUST be reported.
- The source revision and artifact digest are known, because an audit bound
  to no artifact cannot release-block anything.
- The implementation under audit is complete; auditing a half-written
  feature reports its incompleteness, not its security.

## Context requirements
- The task this skill was invoked for, and the outcome the caller expects.
- The manifest, network-security configuration, and signing configuration.
- Revision and artifact digest, plus the environment fingerprint of the
  session.
- The evidence identifiers this run must bind to, so findings attach to
  the same evidence graph as the implementation they judge.

## Allowed tools
- secrets_surface_scanner
- export_surface_lister
- audit_finding_binder

## Procedure
1. Scan every surface for secrets: source, resources, assets, and build
   outputs; a credential or key found in any of them is a finding, and its
   location is named.
2. List every exported component and confirm each carries the permission or
   protection its exposure requires; an unguarded export is a finding.
3. Verify the cleartext policy: confirm the network-security configuration
   forbids what the release policy forbids, and that debug overrides are
   absent from release builds.
4. Verify the release flags: debuggable is false, backup rules are declared,
   and the signing configuration matches the declared release identity.
5. Review the network-security posture: certificate handling, pinned hosts
   where the policy requires them, and authentication error paths that do
   not leak through fallback.
6. Confirm keystore usage: sensitive values live behind the platform key
   store, never in plaintext preferences, caches, or logs.
7. Bind each finding to its severity and its evidence, and emit one
   aggregate audit that a release gate can consume without re-reading the
   source.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `SecurityAuditResult` (§23 SkillPackage contract), carrying
  its classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Every surface was scanned — source, resources, assets, and build outputs
    each appear in the record as scanned or as explicitly out of scope.
  * Every finding binds its location — a defect is named by file, component,
    or configuration, never as an unsupported assertion.
  * Every finding carries its severity — the aggregate audit assigns the
    severity deliberately; nothing defaults to informational.
  * Nothing fixed by assertion — a finding remains open until the source is
    re-audited and the defect is gone; the audit never closes its own
    findings.
  * An unaudited surface is named — what the audit did not read is reported
    as unaudited rather than passed.
  * Every claim is reduced to an observable: what was seen, on which device
    or host, at which revision — never a statement of intent.

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
- SECRET_IN_SOURCE — a credential or key was found in source or in a
  resource file; the location is named and the finding is blocking.
- CLEARTEXT_ALLOWED — cleartext traffic is permitted where the network
  policy forbids it.
- UNGUARDED_EXPORT — an exported component lacks the permission its
  exposure requires.
- DEBUGGABLE_RELEASE — the release build permits debugging or backup the
  release policy forbids.
- UNENCRYPTED_SENSITIVE_STORE — sensitive data is stored without the
  platform's encryption.

## Recovery
- A blocked capability resumes when the capability record changes; the blocked
  node names its resume condition and is never reported as a failure of the
  goal.
- One retry is permitted after a repair that materially changed the input. An
  identical action is never re-run against unchanged evidence, and the
  recoveryAttemptPolicy bound escalates rather than terminating the goal.
- An invariant violation is repaired at its cause, never by relaxing the check
  that detected it.
- A finding is closed only by re-audit after repair; the audit never closes
  a finding because the implementation claims to have fixed it.
- Nothing unverified is reported as verified; an unverified outcome is
  reported as unverified.

## Output contract
Emits `SecurityAuditResult` from `SecurityAuditRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed
- findings: each finding with its location, severity, and evidence
- blockingFindings: the subset that must stop promotion

## Fixtures
- Step 1 produces its expected outcome — Scan every surface for secrets: source, resources, assets, and build outputs; a credential or key found in any of them is a finding, and its location is named.
- Step 2 produces its expected outcome — List every exported component and confirm each carries the permission or protection its exposure requires; an unguarded export is a finding.
- Step 3 produces its expected outcome — Verify the cleartext policy: confirm the network-security configuration forbids what the release policy forbids, and that debug overrides are absent from release builds.
- Step 4 produces its expected outcome — Verify the release flags: debuggable is false, backup rules are declared, and the signing configuration matches the declared release identity.
- Step 5 produces its expected outcome — Review the network-security posture: certificate handling, pinned hosts where the policy requires them, and authentication error paths that do not leak through fallback.
- Step 6 produces its expected outcome — Confirm keystore usage: sensitive values live behind the platform key store, never in plaintext preferences, caches, or logs.
- Step 7 produces its expected outcome — Bind each finding to its severity and its evidence, and emit one aggregate audit that a release gate can consume without re-reading the source.
- A required capability is UNAVAILABLE — blocked, nothing attempted
- A secret found in source — reported as SECRET_IN_SOURCE with its location, never absorbed
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.

