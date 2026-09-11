# Windows Packaging Expert

Scope: Windows packaging and delivery — MSIX package authoring and manifest
capability declaration, code signing and signature verification,
installer and uninstaller behavior, version and update semantics, and
package verification (BS §79.7). This skill provides the domain
knowledge that the `Release Worker` and `Architecture Worker` consume.

## Trigger
This skill is requested when windows packaging and delivery — MSIX package authoring and manifest capability declaration, code signing and signature verification, installer and uninstaller behavior, version and update semantics, and package verification (BS §79.7). This skill provides the domain knowledge that the `Release Worker` and `Architecture Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `WINDOWS_HOST_TOOLCHAIN`
- `WINDOWS_NATIVE_EXECUTION`

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
- msbuild
- msix_tooling
- signtool

## Procedure
1. Declare the package manifest completely and truthfully: identity,
   publisher, version, target device family, and only the capabilities
   the application actually requires.
2. Choose the packaging model deliberately: a packaged MSIX for store or
   sideload distribution, or an unpackaged build for development loops,
   and never mix the two in one artifact.
3. Sign with a certificate whose subject matches the declared publisher,
   then verify the signature on the produced package rather than trusting
   the build step that claims to have signed it.
4. Version monotonically: a package version never goes backwards, and a
   rebuild of identical content is distinguishable from a content change.
5. Test the installer on a clean target: install, launch, verify the
   installed layout and entry points, then uninstall and verify removal.
6. Test the update path explicitly: install the previous version, apply
   the new one, and confirm user data and settings survive the upgrade.
7. Verify uninstall completeness: nothing install created is left behind,
   except data the user explicitly chose to keep.
8. Capture the artifacts of record: the package, its signature, its
   version, and the install, update, and uninstall outcomes.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `PackagingResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * A declared capability is a real requirement; a capability declared
  *   speculatively is a defect.
  * The signature is verified on the produced package, never assumed from
  *   a successful sign step.
  * Version is monotonic, and a rebuild is distinguishable from a change.
  * Install, update, and uninstall are each verified on a clean target; an
  *   unverified lifecycle step is reported as unverified.
  * Uninstall removes what install created, except data the user chose to
  *   keep.
  * No credential or signing secret is written to a log, an artifact, or
  *   a memory record.
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
Emits `PackagingResult` from `PackagingResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Declare the package manifest completely and truthfully: identity,
- Step 2 produces its expected outcome — Choose the packaging model deliberately: a packaged MSIX for store or
- Step 3 produces its expected outcome — Sign with a certificate whose subject matches the declared publisher,
- Step 4 produces its expected outcome — Version monotonically: a package version never goes backwards, and a
- Step 5 produces its expected outcome — Test the installer on a clean target: install, launch, verify the
- Step 6 produces its expected outcome — Test the update path explicitly: install the previous version, apply
- Step 7 produces its expected outcome — Verify uninstall completeness: nothing install created is left behind,
- Step 8 produces its expected outcome — Capture the artifacts of record: the package, its signature, its
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
