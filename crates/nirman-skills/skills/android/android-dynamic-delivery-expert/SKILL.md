# Android Dynamic Delivery Expert

Scope: Android dynamic delivery — dynamic feature modules (on-demand
delivery, conditional delivery), Play Feature Delivery (install-time,
on-demand, conditional), Play Asset Delivery (install-time, fast-follow,
on-demand), and app bundles (BS §79.7). This skill provides the dynamic
delivery domain knowledge that the `Release Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the blocked state MUST be reported. This skill does not
replace the `Release Worker` role — it provides dynamic delivery-specific
instruction.

## Workflow
1. Analyze delivery requirements: identify which features can be deferred
   (on-demand), which are needed immediately (install-time), and which
   are device-conditional.
2. Create dynamic feature modules: use `com.android.dynamic-feature`
   plugin, define `dist:module` metadata, and configure delivery options
   in the module manifest.
3. Configure Play Feature Delivery: use `<dist:module dist:title="...">`
   with `dist:on-demand` or `dist:instant` attributes. Define conditions
   (`dist:device-feature`, `dist:min-sdk`, `dist:user-countries`).
4. Configure Play Asset Delivery: use `<dist:install-time>`,
   `<dist:fast-follow>`, or `<dist:on-demand>` for asset packs. Define
   asset pack metadata in build.gradle.kts.
5. Request on-demand modules: use SplitInstallManager to request
   module installation, handle SplitInstallRequest, monitor
   SplitInstallSessionStatus, and handle errors.
6. Manage asset packs: use AssetPackManager to fetch asset packs,
   handle AssetPackStatus, and access downloaded assets with
   AssetPackLocation.
7. Test dynamic delivery: use internal app sharing for testing, test
   on-demand module requests, test asset pack delivery, and verify
   module uninstall behavior.

## Invariants
- Dynamic feature modules are optional — the app MUST function without
  on-demand modules. Gracefully handle module unavailability.
- On-demand modules require Play Store — dynamic delivery only works
  through the Play Store. Use internal app sharing for testing.
- Asset packs have size limits — install-time asset packs are limited
  to 1 GB. Use fast-follow for larger assets.
- Module requests are monitored — SplitInstallManager provides
  real-time status updates. Handle all status values (pending,
  downloading, installed, failed, canceled).
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
