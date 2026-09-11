# Android Printing Expert

Scope: Printing from an app — the platform print framework, print adapters
for documents and images, PDF generation and rendering, custom print
options and page ranges, the system print preview, and print service
discovery and status reporting (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `UI Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- Printing goes through the platform framework; no direct printer
  protocol or vendor SDK path bypasses it.
- An unsupported print option is absent from the options, never
  presented and then ignored.
- Large documents paginate lazily; content is not fully materialized in
  memory before printing.
- Job outcome is reported truthfully, including failure and user
  cancellation.
- A print capability that is unavailable is reported as unavailable;
  printing is never claimed to have succeeded without a completed job.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
