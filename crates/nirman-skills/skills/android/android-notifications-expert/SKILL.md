# Android Notifications Expert

Scope: Android notifications — notification channels (importance, sound,
vibration, lights), notification types (basic, progress, media, messaging,
call), rich notifications (images, actions, replies), notification groups,
and notification permissions (BS §79.7). This skill provides the
notifications domain knowledge that the `Android Data and Integration Worker`
consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the blocked state MUST be reported. This skill does not
replace the `Android Data and Integration Worker` role — it provides
notifications-specific instruction.

## Workflow
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

## Invariants
- Notification channels are immutable after creation — channel settings
   (importance, sound) can only be changed by the user after creation.
   Create channels on app start.
- Notifications require a small icon — every notification MUST have a
   small icon. Use a transparent icon for the notification shade.
- POST_NOTIFICATIONS is required on API 33+ — notifications are blocked
   without this permission. Request at point-of-use, not at app start.
- Notification actions have a maximum of 3 — only the first 3 actions
   are displayed. Prioritize the most important actions.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
   execution still passes through ToolBroker and PolicyAuthority.
