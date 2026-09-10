# Android Firebase Expert

Scope: Firebase integration — Firebase Authentication (email, Google,
Facebook, phone), Cloud Firestore (NoSQL database, real-time sync,
offline persistence), Cloud Messaging (FCM push notifications),
Analytics (user behavior tracking), Crashlytics (crash reporting),
Cloud Functions (serverless backend), and Cloud Storage (file storage)
(BS §79.7). This skill provides the Firebase domain knowledge that the
`Android Data and Integration Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides Firebase-specific instruction.

## Workflow
1. Analyze Firebase requirements: identify authentication methods,
   database structure, messaging needs, analytics events, and storage
   requirements.
2. Configure Firebase project: create a Firebase project in the console,
   register the Android app, download the configuration file, and add
   the Firebase SDK to the project.
3. Implement Firebase Authentication: use FirebaseAuth for email/password,
   Google Sign-In, Facebook Login, or phone authentication. Handle
   authentication state with AuthStateListener.
4. Implement Cloud Firestore: define collections and documents, use
   FirebaseFirestore for CRUD operations, enable offline persistence,
   and use SnapshotListener for real-time updates.
5. Implement Cloud Messaging: extend FirebaseMessagingService to handle
   token registration, message reception, and notification display.
6. Configure Analytics and Crashlytics: use FirebaseAnalytics for
   event logging, FirebaseCrashlytics for crash reporting. Add custom
   keys and user properties for debugging.
7. Test Firebase features: use Firebase Emulator Suite for local testing,
   Firebase Test Lab for device testing.

## Invariants
- Firebase configuration is per-project — each Nirman project has its own
  Firebase project. Never reuse configuration across projects.
- Authentication state is reactive — use AuthStateListener to react to
  sign-in/sign-out events. Never cache credentials locally.
- Firestore offline persistence is enabled by default — data is available
  offline and syncs when connectivity returns.
- FCM tokens can change — always register the latest token with the app
  server. Handle token refresh in onNewToken.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
