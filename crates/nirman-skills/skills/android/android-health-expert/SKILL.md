# Android Health Services Expert

Scope: Health and fitness data — Health Connect read and write with
granular permissions and disclosure, Health Services exercise and passive
sensor tracking on Wear OS, heart rate and location series, records and
aggregations, and durable sync of health data (BS §79.7). This skill provides the domain knowledge
that the `Android Data and Integration Worker` and `UI Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN and ANDROID_NATIVE_DEVICE_CAPABILITIES. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
1. Determine the data path: Health Connect as the on-device store for
   records the user owns, and Health Services for live exercise and
   passive sensor sampling while a session runs.
2. Declare granular permissions for exactly the record types used, and
   request them at the point of use rather than at startup.
3. Check availability explicitly: Health Connect may be absent or not
   updated, and the app must offer the install path instead of failing
   silently.
4. Write records with a client-generated record id and a deterministic
   time range so a retry is idempotent rather than duplicative.
5. Read with aggregations and time ranges rather than fetching entire
   histories into memory; page large reads.
6. For live tracking, run an exercise session through Health Services:
   declare the exercise type and data types up front, sample at the
   required rate, and keep the session in a foreground service with a
   visible ongoing notification.
7. Verify on the Nirman-managed emulator: grant and deny each
   permission, confirm the denial path degrades gracefully, and confirm
   written records read back with the same values and time ranges.

## Invariants
- Permissions are granular and requested at the point of use, never in
  a startup batch.
- Missing Health Connect is reported with an install path, never
  treated as empty data.
- Writes are idempotent: a retry with the same record id does not
  duplicate a record.
- A denied permission degrades the feature; it never blocks the rest of
  the app and never fabricates a value.
- Health data is handled as sensitive: no health value is written to
  logs, memory records, or crash reports.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
