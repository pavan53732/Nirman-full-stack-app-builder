# Android Media Expert

Scope: Android media and camera — CameraX (preview, image capture, video),
Media3/ExoPlayer (audio/video playback), Coil (image loading),
MediaSession, picture-in-picture, and media notifications (BS §79.7).
This skill provides the media domain knowledge that the `Android Data
and Integration Worker` and `UI Worker` consume.

## Trigger
This skill is requested when android media and camera — CameraX (preview, image capture, video), Media3/ExoPlayer (audio/video playback), Coil (image loading), MediaSession, picture-in-picture, and media notifications (BS §79.7). This skill provides the media domain knowledge that the `Android Data and Integration Worker` and `UI Worker` consume.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NATIVE_DEVICE_CAPABILITIES`

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
- cameralibrary
- media3_library
- coil_library

## Procedure
1. Analyze media requirements: identify camera needs (photo, video,
   scan), playback needs (audio, video, streaming), and image loading
   needs (remote, local, caching).
2. Implement CameraX: use ProcessCameraProvider for camera lifecycle,
   PreviewView for preview, ImageCapture for photos, VideoCapture
   for video. Handle camera permissions and lifecycle.
3. Implement Media3/ExoPlayer: use ExoPlayer for media playback,
   MediaSessionService for background playback, MediaNotification
   for playback notifications. Handle audio focus and media buttons.
4. Implement image loading: use Coil for Compose (AsyncImage),
   with memory cache, disk cache, and placeholder/error images. Respect
   image size constraints to avoid OOM.
5. Handle media permissions: CAMERA, RECORD_AUDIO, READ_MEDIA_IMAGES,
   READ_MEDIA_VIDEO (API 33+). Request permissions with
   ActivityResultContracts.RequestPermission.
6. Test media features: use CameraX FakeImageCapture for unit tests,
   AndroidJUnit4 for instrumented tests with real camera.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `MediaImplementationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * CameraX is the camera API of choice — it handles device-specific
  *    quirks and lifecycle automatically. Avoid Camera2 for new code.
  * Media3 is the playback library of choice — it handles background
  *    playback, media sessions, and notifications. Avoid MediaPlayer.
  * Coil is the image loading library of choice — it integrates with
  *    Compose, handles caching, and respects lifecycle. Avoid Glide for
  *    new Compose code.
  * Media permissions are requested at point-of-use — request camera
  *    permission when the user taps the camera button, not at app start.
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
Emits `MediaImplementationResult` from `MediaImplementationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze media requirements: identify camera needs (photo, video,
- Step 2 produces its expected outcome — Implement CameraX: use ProcessCameraProvider for camera lifecycle,
- Step 3 produces its expected outcome — Implement Media3/ExoPlayer: use ExoPlayer for media playback,
- Step 4 produces its expected outcome — Implement image loading: use Coil for Compose (AsyncImage),
- Step 5 produces its expected outcome — Handle media permissions: CAMERA, RECORD_AUDIO, READ_MEDIA_IMAGES,
- Step 6 produces its expected outcome — Test media features: use CameraX FakeImageCapture for unit tests,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
