# Android Maps Expert

Scope: Google Maps integration — Maps SDK for Android (map views,
markers, polylines, polygons, ground overlays), location services
(Fused Location Provider, foreground location), geofencing, and
custom map styling (BS §79.7). This skill provides the maps and
location domain knowledge that the `Android Data and Integration Worker`
consumes.

## Trigger
This skill is requested when google Maps integration — Maps SDK for Android (map views, markers, polylines, polygons, ground overlays), location services (Fused Location Provider, foreground location), geofencing, and custom map styling (BS §79.7). This skill provides the maps and location domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
instruction the worker executes inside its scoped asset
transaction (BS §50).

## Required capabilities
- `ANDROID_BUILD_TOOLCHAIN`
- `ANDROID_NETWORK_INTEGRATION`
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
- maps_sdk
- fused_location_provider
- geofencing_client

## Procedure
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

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `MapsIntegrationResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * Maps API key is restricted — restrict the API key to the app's package
  *   name and SHA-1 fingerprint. Never expose the API key in version control.
  * Location permissions are requested at point-of-use — request foreground
  *   location when the user needs location, background location only when
  *   the feature requires it.
  * Geofences have a minimum radius — geofences smaller than 100 meters may
  *   not trigger reliably. Use dwell time to reduce false positives.
  * Map view lifecycle is managed — call onResume, onPause, onDestroy,
  *  onLowMemory on MapView to avoid memory leaks.
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
Emits `MapsIntegrationResult` from `MapsIntegrationRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze maps requirements: identify map display needs, marker types,
- Step 2 produces its expected outcome — Configure Google Maps: create a Google Cloud project, enable the Maps
- Step 3 produces its expected outcome — Implement the map view: use SupportMapFragment or MapView with
- Step 4 produces its expected outcome — Add markers and overlays: use MarkerOptions for points of interest,
- Step 5 produces its expected outcome — Implement location tracking: use FusedLocationProviderClient for
- Step 6 produces its expected outcome — Implement geofencing: use GeofencingClient to add/remove geofences,
- Step 7 produces its expected outcome — Test maps features: use Google Maps emulator extensions, test location
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
