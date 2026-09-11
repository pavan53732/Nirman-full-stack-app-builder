# Android Printing Expert

Scope: Printing from an app — the platform print framework, print adapters
for documents and images, PDF generation and rendering, custom print
options and page ranges, the system print preview, and print service
discovery and status reporting (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `UI Worker` consume.

## Trigger
This skill is requested when printing from an app — the platform print framework, print adapters for documents and images, PDF generation and rendering, custom print options and page ranges, the system print preview, and print service discovery and status reporting (BS §79.7). This skill provides the domain knowledge that the `Android Data and Integration Worker` and `UI Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`

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
- print_framework
- pdf_renderer
- managed_emulator

## Procedure
1. Choose the right surface: hand content to the system print framework
   rather than talking to a printer directly, so every print service the
   user has installed can produce it.
2. Implement a print adapter that matches the content type: a document
   adapter that paginates text and vector content, or a photo adapter for
   images that must preserve resolution.
3. Render paginated output to PDF through the platform PDF APIs for
   documents; generate the pages lazily so a large document does not
   materialize in memory at once.
4. Expose honest print options: page range, copies, color mode, and
   orientation, each supported only where the adapter can actually honour
   it.
5. Let the user reach the system print preview before committing; the app
   supplies a job name and content, and the platform owns the preview and
   the destination choice.
6. Track the print job: observe queued, started, completed, failed, and
   cancelled states, and report the outcome to the user in the app's own
   vocabulary.
7. Verify on the Nirman-managed emulator with a print service available:
   produce a multi-page document, confirm pagination and page range
   behave, and confirm cancellation and failure are reported.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `PrintJobResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Printing goes through the platform framework; no direct printer
  *   protocol or vendor SDK path bypasses it.
  * An unsupported print option is absent from the options, never
  *   presented and then ignored.
  * Large documents paginate lazily; content is not fully materialized in
  *   memory before printing.
  * Job outcome is reported truthfully, including failure and user
  *   cancellation.
  * A print capability that is unavailable is reported as unavailable;
  *   printing is never claimed to have succeeded without a completed job.
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

- PRINT_ADAPTER_MISMATCH — the adapter does not match the content type, so pagination is wrong.
- PRINT_OPTION_DISHONESTED — an option is offered that the print service cannot honour.
- JOB_STATE_UNKNOWN — the job's state could not be observed, so success was assumed from submission.

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
Emits `PrintJobResult` from `PrintJobResultRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Choose the right surface: hand content to the system print framework
- Step 2 produces its expected outcome — Implement a print adapter that matches the content type: a document
- Step 3 produces its expected outcome — Render paginated output to PDF through the platform PDF APIs for
- Step 4 produces its expected outcome — Expose honest print options: page range, copies, color mode, and
- Step 5 produces its expected outcome — Let the user reach the system print preview before committing; the app
- Step 6 produces its expected outcome — Track the print job: observe queued, started, completed, failed, and
- Step 7 produces its expected outcome — Verify on the Nirman-managed emulator with a print service available:
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
