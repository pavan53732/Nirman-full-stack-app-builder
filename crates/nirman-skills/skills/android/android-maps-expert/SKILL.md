# Android Maps Expert

Scope: Google Maps integration — Maps SDK for Android (map views,
markers, polylines, polygons, ground overlays), location services
(Fused Location Provider, foreground location), geofencing, and
custom map styling (BS §79.7). This skill provides the maps and
location domain knowledge that the `Android Data and Integration Worker`
consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Android Data and
Integration Worker` role — it provides maps-specific instruction.

## Workflow
1. Analyze maps requirements: identify map display needs, marker types,
   location tracking requirements, geofence regions, and custom styling.
2. Configure Google Maps: create a Google Cloud project, enable the Maps
   SDK for Android, obtain an API key, and add it to the manifest.
3. Implement the map view: use SupportMapFragment or MapView with
   GoogleMap callback. Configure map type, zoom controls, and
   compass settings.
4. Add markers and overlays: use MarkerOptions for points of interest,
   Polyline for routes, Polygon for regions, GroundOverlay for
   image overlays.
5. Implement location tracking: use FusedLocationProviderClient for
   location updates, request location permissions, handle foreground
   and background location access.
6. Implement geofencing: use GeofencingClient to add/remove geofences,
   handle geofence transitions with BroadcastReceiver, and define
   geofence expiration and dwell time.
7. Test maps features: use Google Maps emulator extensions, test location
   with mock locations, verify geofence transitions.

## Invariants
- Maps API key is restricted — restrict the API key to the app's package
  name and SHA-1 fingerprint. Never expose the API key in version control.
- Location permissions are requested at point-of-use — request foreground
  location when the user needs location, background location only when
  the feature requires it.
- Geofences have a minimum radius — geofences smaller than 100 meters may
  not trigger reliably. Use dwell time to reduce false positives.
- Map view lifecycle is managed — call onResume, onPause, onDestroy,
 onLowMemory on MapView to avoid memory leaks.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
