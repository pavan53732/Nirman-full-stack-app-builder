# Android Camera ML Expert

Scope: Android camera and ML Kit — CameraX (preview, image capture,
video analysis), ML Kit (barcode scanning, face detection, text
recognition, image labeling, pose detection, selfie segmentation),
and custom model integration (TensorFlow Lite) (BS §79.7). This skill
provides the camera and ML domain knowledge that the `Android Data and
Integration Worker` consumes.

## Trigger
This skill is requested when android camera and ML Kit — CameraX (preview, image capture, video analysis), ML Kit (barcode scanning, face detection, text recognition, image labeling, pose detection, selfie segmentation), and custom model integration (TensorFlow Lite) (BS §79.7). This skill provides the camera and ML domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NATIVE_DEVICE_CAPABILITIES`
- `ANDROID_UI_OBSERVATION`

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
- ml_kit
- tensorflow_lite

## Procedure
1. Analyze camera/ML requirements: identify the use case (barcode scanning,
   face detection, text recognition, image labeling, custom model),
   camera requirements (front/back, resolution), and performance needs.
2. Set up CameraX: use ProcessCameraProvider for camera lifecycle,
   PreviewView for preview, ImageCapture for photos, ImageAnalysis
   for ML processing.
3. Implement ML Kit features: use BarcodeScanning, FaceDetection,
   TextRecognition, ImageLabeling, PoseDetection. Configure
   detector settings (speed vs accuracy, min face size).
4. Process camera frames: use ImageAnalysis.Analyzer with CameraX
   ImageProxy. Convert ImageProxy to InputImage for ML Kit.
   Handle frame throttling for performance.
5. Integrate custom models: use TensorFlow Lite with the GPU delegate
   for acceleration. Convert models to TFLite format, optimize with
   quantization.
6. Handle camera permissions: request CAMERA permission, handle
   permission denial gracefully, provide fallback UI.
7. Test camera/ML features: use CameraX FakeImageCapture for unit
   tests, test ML Kit with sample images, verify performance on
   different devices.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `CameraMLResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * CameraX is the camera API of choice — it handles device-specific
  *   quirks and lifecycle automatically. Avoid Camera2 for new code.
  * ML Kit runs on-device — no network connection is required for ML Kit
  *   features. Models are bundled with the app.
  * ImageAnalysis frames are throttled — ML Kit processing is synchronous.
  *   Drop frames if processing takes longer than the frame interval.
  * Camera permissions are requested at point-of-use — request camera
  *   permission when the user taps the camera button, not at app start.
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
Emits `CameraMLResult` from `CameraMLRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze camera/ML requirements: identify the use case (barcode scanning,
- Step 2 produces its expected outcome — Set up CameraX: use ProcessCameraProvider for camera lifecycle,
- Step 3 produces its expected outcome — Implement ML Kit features: use BarcodeScanning, FaceDetection,
- Step 4 produces its expected outcome — Process camera frames: use ImageAnalysis.Analyzer with CameraX
- Step 5 produces its expected outcome — Integrate custom models: use TensorFlow Lite with the GPU delegate
- Step 6 produces its expected outcome — Handle camera permissions: request CAMERA permission, handle
- Step 7 produces its expected outcome — Test camera/ML features: use CameraX FakeImageCapture for unit
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
