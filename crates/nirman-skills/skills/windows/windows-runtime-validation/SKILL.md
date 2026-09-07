# Windows Runtime Validation

Scope: native Windows runtime validation — startup, IPC, ConPTY, process
supervision, Job Objects, isolation, restart/recovery, credential
storage, installer/uninstaller behavior (BS §79.7).

Gated by `WINDOWS_HOST_TOOLCHAIN` AND `WINDOWS_NATIVE_EXECUTION`, the
latter requiring a leased Windows ValidationEnvironment (BS §79.8). When
no lease can be acquired `WINDOWS_NATIVE_EXECUTION` resolves to
UNAVAILABLE (or USER_REQUIRED), so the gated steps MUST NOT execute and
the blocked state MUST be reported with the §79.11 lists. A simulated
pass, or a build result reinterpreted as a runtime pass, is prohibited
and is a certification failure.

## Workflow
1. Verify a durable ValidationEnvironment lease exists for a native
   Windows target (BS §79.8). No lease, no validation claim.
2. Launch the built executable on the native host and capture the
   observation set (process identity, runtime output, IPC, recovery).
3. Bind the observations to evidence and update the validation gate.

## Invariants
- The output field `simulated` is fixed to false; a skill that cannot
  observe a real native process must not emit a pass.
- Build evidence is never reinterpreted as runtime evidence
  (BS §79.10).
- A missing environment is a truthful USER_REQUIRED/UNAVAILABLE node,
  not a substitute target.
