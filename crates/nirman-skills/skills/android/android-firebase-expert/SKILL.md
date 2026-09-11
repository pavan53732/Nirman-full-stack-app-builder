# Android Firebase Expert

Scope: Firebase integration — Firebase Authentication (email, Google,
Facebook, phone), Cloud Firestore (NoSQL database, real-time sync,
offline persistence), Cloud Messaging (FCM push notifications),
Analytics (user behavior tracking), Crashlytics (crash reporting),
Cloud Functions (serverless backend), and Cloud Storage (file storage)
(BS §79.7). This skill provides the Firebase domain knowledge that the
`Android Data and Integration Worker` consumes.

## Trigger
This skill is requested when firebase integration — Firebase Authentication (email, Google, Facebook, phone), Cloud Firestore (NoSQL database, real-time sync, offline persistence), Cloud Messaging (FCM push notifications), Analytics (user behavior tracking), Crashlytics (crash reporting), Cloud Functions (serverless backend), and Cloud Storage (file storage) (BS §79.7). This skill provides the Firebase domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NETWORK_INTEGRATION`
- `ANDROID_AUTHENTICATION`

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
- firebase_sdk
- firebase_auth
- cloud_firestore
- cloud_messaging

## Procedure
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

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `FirebaseIntegrationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Firebase configuration is per-project — each Nirman project has its own
  *   Firebase project. Never reuse configuration across projects.
  * Authentication state is reactive — use AuthStateListener to react to
  *   sign-in/sign-out events. Never cache credentials locally.
  * Firestore offline persistence is enabled by default — data is available
  *   offline and syncs when connectivity returns.
  * FCM tokens can change — always register the latest token with the app
  *   server. Handle token refresh in onNewToken.
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
Emits `FirebaseIntegrationResult` from `FirebaseIntegrationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze Firebase requirements: identify authentication methods,
- Step 2 produces its expected outcome — Configure Firebase project: create a Firebase project in the console,
- Step 3 produces its expected outcome — Implement Firebase Authentication: use FirebaseAuth for email/password,
- Step 4 produces its expected outcome — Implement Cloud Firestore: define collections and documents, use
- Step 5 produces its expected outcome — Implement Cloud Messaging: extend FirebaseMessagingService to handle
- Step 6 produces its expected outcome — Configure Analytics and Crashlytics: use FirebaseAnalytics for
- Step 7 produces its expected outcome — Test Firebase features: use Firebase Emulator Suite for local testing,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
