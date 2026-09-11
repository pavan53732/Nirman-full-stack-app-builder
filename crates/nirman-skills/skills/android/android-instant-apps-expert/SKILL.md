# Android Instant Apps Expert

Scope: Shipping a trial-able app — instant-enabled app bundles, URL
handling and app link verification, the instant size budget and the module
split needed to meet it, runtime permission and storage differences, and
state handover from the instant experience to the installed app (BS §79.7). This skill provides the domain knowledge
that the `Release Worker` and `Architecture Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN and ANDROID_PACKAGING. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- The instant bundle stays inside the size budget; a bundle that
  exceeds it is reported as a packaging defect, not shipped anyway.
- URL entry is verified end to end; an unverified app link is a defect.
- No background service, alarm, or persistent identifier is used in the
  instant runtime.
- State created in the instant session survives the transition to the
  installed app.
- Install is offered; it is never forced as the only way to continue.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
