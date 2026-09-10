# Android Media Expert

Scope: Android media and camera — CameraX (preview, image capture, video),
Media3/ExoPlayer (audio/video playback), Coil (image loading),
MediaSession, picture-in-picture, and media notifications (BS §79.7).
This skill provides the media domain knowledge that the `Android Data
and Integration Worker` and `UI Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides media-specific instruction.

## Workflow
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

## Invariants
- CameraX is the camera API of choice — it handles device-specific
   quirks and lifecycle automatically. Avoid Camera2 for new code.
- Media3 is the playback library of choice — it handles background
   playback, media sessions, and notifications. Avoid MediaPlayer.
- Coil is the image loading library of choice — it integrates with
   Compose, handles caching, and respects lifecycle. Avoid Glide for
   new Compose code.
- Media permissions are requested at point-of-use — request camera
   permission when the user taps the camera button, not at app start.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
   execution still passes through ToolBroker and PolicyAuthority.
