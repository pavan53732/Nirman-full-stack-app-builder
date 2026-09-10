# Android Camera ML Expert

Scope: Android camera and ML Kit — CameraX (preview, image capture,
video analysis), ML Kit (barcode scanning, face detection, text
recognition, image labeling, pose detection, selfie segmentation),
and custom model integration (TensorFlow Lite) (BS §79.7). This skill
provides the camera and ML domain knowledge that the `Android Data and
Integration Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides camera/ML-specific instruction.

## Workflow
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

## Invariants
- CameraX is the camera API of choice — it handles device-specific
  quirks and lifecycle automatically. Avoid Camera2 for new code.
- ML Kit runs on-device — no network connection is required for ML Kit
  features. Models are bundled with the app.
- ImageAnalysis frames are throttled — ML Kit processing is synchronous.
  Drop frames if processing takes longer than the frame interval.
- Camera permissions are requested at point-of-use — request camera
  permission when the user taps the camera button, not at app start.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
