# Windows Packaging Expert

Scope: Windows packaging and delivery — MSIX package authoring and manifest
capability declaration, code signing and signature verification,
installer and uninstaller behavior, version and update semantics, and
package verification (BS §79.7). This skill provides the domain
knowledge that the `Release Worker` and `Architecture Worker` consume.

Gated by WINDOWS_HOST_TOOLCHAIN and WINDOWS_NATIVE_EXECUTION. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked
state MUST be reported. This skill does not replace a worker role
— it provides domain-specific instruction that the worker executes
within its scoped asset transaction (BS §50).

## Workflow
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

## Invariants
- A declared capability is a real requirement; a capability declared
  speculatively is a defect.
- The signature is verified on the produced package, never assumed from
  a successful sign step.
- Version is monotonic, and a rebuild is distinguishable from a change.
- Install, update, and uninstall are each verified on a clean target; an
  unverified lifecycle step is reported as unverified.
- Uninstall removes what install created, except data the user chose to
  keep.
- No credential or signing secret is written to a log, an artifact, or
  a memory record.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
