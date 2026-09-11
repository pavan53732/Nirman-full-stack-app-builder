# Android Notifications Expert

Scope: Android notifications — notification channels (importance, sound,
vibration, lights), notification types (basic, progress, media, messaging,
call), rich notifications (images, actions, replies), notification groups,
and notification permissions (BS §79.7). This skill provides the
notifications domain knowledge that the `Android Data and Integration Worker`
consumes.

## Trigger
This skill is requested when android notifications — notification channels (importance, sound, vibration, lights), notification types (basic, progress, media, messaging, call), rich notifications (images, actions, replies), notification groups, and notification permissions (BS §79.7). This skill provides the notifications domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_INTERACTION_EXECUTION`

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
- notification_manager
- notification_compat
- notification_channels

## Procedure
1. Analyze notification requirements: identify notification types,
   channel importance, user actions, and grouping strategy.
2. Create notification channels: use NotificationManager.createNotificationChannel
   with appropriate importance level (HIGH, DEFAULT, LOW, MIN). Set
   channel name, description, sound, and vibration pattern.
3. Build notifications: use NotificationCompat.Builder with small icon,
   title, content text, priority, and category. Add large icon, content
   image, and style (BigTextStyle, InboxStyle, MediaStyle, MessagingStyle).
4. Add actions and replies: use Notification.Action for user actions,
   RemoteInput for inline replies. Handle action intents with
   PendingIntent.
5. Handle notification groups: use setGroup to group related notifications,
   setGroupSummary for group summary. Use MessagingStyle for
   conversation notifications.
6. Request notification permission: use POST_NOTIFICATIONS permission
   (API 33+), request at point-of-use, handle permission denial gracefully.
7. Test notifications: verify channel settings, test on multiple API levels,
   test permission flows, and verify notification delivery.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `NotificationsResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Notification channels are immutable after creation — channel settings
  *    (importance, sound) can only be changed by the user after creation.
  *    Create channels on app start.
  * Notifications require a small icon — every notification MUST have a
  *    small icon. Use a transparent icon for the notification shade.
  * POST_NOTIFICATIONS is required on API 33+ — notifications are blocked
  *    without this permission. Request at point-of-use, not at app start.
  * Notification actions have a maximum of 3 — only the first 3 actions
  *    are displayed. Prioritize the most important actions.
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

- CHANNEL_MISCONFIGURED — a notification is posted to a channel whose importance does not match its purpose.
- NOTIFICATION_PERMISSION_DENIED — posting was attempted after POST_NOTIFICATIONS was denied.
- GROUP_SUMMARY_MISSING — grouped notifications are posted without a summary, so the group cannot be collapsed.

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
Emits `NotificationsResult` from `NotificationsRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze notification requirements: identify notification types,
- Step 2 produces its expected outcome — Create notification channels: use NotificationManager.createNotificationChannel
- Step 3 produces its expected outcome — Build notifications: use NotificationCompat.Builder with small icon,
- Step 4 produces its expected outcome — Add actions and replies: use Notification.Action for user actions,
- Step 5 produces its expected outcome — Handle notification groups: use setGroup to group related notifications,
- Step 6 produces its expected outcome — Request notification permission: use POST_NOTIFICATIONS permission
- Step 7 produces its expected outcome — Test notifications: verify channel settings, test on multiple API levels,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
