# Android Push Notifications

Scope: push notification delivery for a generated Android application — the Firebase Cloud Messaging token lifecycle and its refresh, delivery treated as best-effort, payload limits, de-duplication, and the permission states a user actually sees (BS §79.7).

## Trigger
A push feature is designed or reviewed, or notifications arrive late, duplicated, or
not at all, or a token is stale.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The platform's delivery contract is known: size limits, delivery semantics, and
  whether delivery is guaranteed at all.

## Context requirements
- The platforms in scope and each one's payload limits.
- The user-visible permission states and when each is shown.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether delivery is best-effort or required for correctness.

## Allowed tools
- static_analyzer
- push_probe

## Procedure
1. Confirm the platform capability is present and permitted before any token is
   requested, rather than requesting and handling the denial afterwards.
2. Request permission at a moment the user understands, and record the outcome —
   granted, denied, or not asked — rather than assuming it.
3. Acquire the token and register it with the backend, then confirm the registration
   succeeded rather than assuming the token arrived.
4. Handle token refresh: a rotated token replaces the old one server-side, and the old
   one is removed rather than left registered.
5. Structure the payload within the platform's limit, putting volatile content behind a
   fetch rather than in the notification body.
6. Treat delivery as best-effort: never make correctness depend on a notification
   arriving, and always provide an in-app path that shows the same information.
7. Verify de-duplication and collapse behaviour, so a user is not shown the same
   notification repeatedly for one event.
8. Verify the fallback: a notification that cannot be delivered leaves the information
   available in-app rather than lost.

## Evidence
- Capability and permission state before any request.
- Permission outcome recorded, including the not-asked case.
- Token registration confirmation, and the refresh and removal behaviour.
- Payload sizes against the platform limit for each platform in scope.
- De-duplication and collapse results for a single event.
- Fallback verification: the same information reachable in-app.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- TOKEN_UNREGISTERED — a token was acquired but never confirmed registered.
- STALE_TOKEN_RETAINED — a rotated token left the old one registered.
- PAYLOAD_TOO_LARGE — the payload exceeds a platform limit.
- DELIVERY_DEPENDENT — correctness depends on a notification arriving.

## Recovery
- A stale token is removed server-side on rotation rather than letting delivery fail
  silently to it.
- An oversized payload is restructured with the volatile content fetched on open,
  not truncated until it fits.
- A delivery-dependent path is fixed by adding an in-app source of the same
  information, not by retrying delivery harder.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `PushNotificationResult` (§23 SkillPackage contract):

- permissionState: outcome recorded, including not asked
- registration: token registered, refreshed, and old one removed
- payloads: sizes against each platform limit
- delivery: de-duplication, collapse, and fallback results
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured push evidence

## Fixtures
- Permission granted and token registered
- Rotated token replaces the old registration
- Payload within the platform limit
- Same event shown once, not repeatedly
- Notification undelivered but information available in-app
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
