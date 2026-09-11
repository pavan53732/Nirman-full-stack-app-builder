# Android Instant Apps Expert

Scope: Shipping a trial-able app — instant-enabled app bundles, URL
handling and app link verification, the instant size budget and the module
split needed to meet it, runtime permission and storage differences, and
state handover from the instant experience to the installed app (BS §79.7). This skill provides the domain knowledge
that the `Release Worker` and `Architecture Worker` consume.

## Trigger
This skill is requested when shipping a trial-able app — instant-enabled app bundles, URL handling and app link verification, the instant size budget and the module split needed to meet it, runtime permission and storage differences, and state handover from the instant experience to the installed app (BS §79.7). This skill provides the domain knowledge that the `Release Worker` and `Architecture Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_PACKAGING`

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
- bundle_tool
- play_instant_sdk
- managed_emulator

## Procedure
1. Decide what the instant experience covers: one well-scoped entry
   point, reachable by URL, that demonstrates the app without an install.
2. Split the build so the instant path is small: an instant-enabled base
   module plus dynamic feature modules for the rest, keeping the
   downloaded instant bundle inside the size budget.
3. Map URLs: declare intent filters with autoVerify, serve the asset
   links JSON, and confirm the link opens the instant experience rather
   than a browser.
4. Adapt to the instant runtime: no background services, no persistent
   device identifiers, and permissions granted through the instant
   runtime rather than the installed model.
5. Persist state where the installed app can read it back, so a user who
   installs does not lose what they did in the instant session.
6. Offer install at a natural completion point, and hand over state on
   install through the shared storage the platform provides for this
   transition.
7. Verify the real artifact: install and launch the instant bundle from
   its URL on the Nirman-managed emulator, measure the downloaded size,
   and confirm the handover preserves state.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `InstantAppResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * The instant bundle stays inside the size budget; a bundle that
  *   exceeds it is reported as a packaging defect, not shipped anyway.
  * URL entry is verified end to end; an unverified app link is a defect.
  * No background service, alarm, or persistent identifier is used in the
  *   instant runtime.
  * State created in the instant session survives the transition to the
  *   installed app.
  * Install is offered; it is never forced as the only way to continue.
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

- URL_VERIFICATION_FAILED — the asset links JSON is absent or wrong, so app links do not resolve to the app.
- INSTANT_RUNTIME_VIOLATION — the instant path uses background services or persistent device identifiers.
- STATE_LOST_ON_INSTALL — state created in the instant experience is not readable by the installed app.
- INSTANT_BUNDLE_TOO_LARGE — the instant path exceeds the size the instant runtime permits.

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
Emits `InstantAppResult` from `InstantAppResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Decide what the instant experience covers: one well-scoped entry
- Step 2 produces its expected outcome — Split the build so the instant path is small: an instant-enabled base
- Step 3 produces its expected outcome — Map URLs: declare intent filters with autoVerify, serve the asset
- Step 4 produces its expected outcome — Adapt to the instant runtime: no background services, no persistent
- Step 5 produces its expected outcome — Persist state where the installed app can read it back, so a user who
- Step 6 produces its expected outcome — Offer install at a natural completion point, and hand over state on
- Step 7 produces its expected outcome — Verify the real artifact: install and launch the instant bundle from
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
