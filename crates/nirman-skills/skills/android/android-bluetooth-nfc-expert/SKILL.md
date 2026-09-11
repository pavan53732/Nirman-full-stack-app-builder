# Android Bluetooth NFC Expert

Scope: Android Bluetooth and NFC — Bluetooth Low Energy (BLE) (GATT server,
GATT client, scanning, connections), Classic Bluetooth (RFCOMM, SPP),
NFC (NDEF reading/writing, HCE host card emulation), and Nearby
Connections (BS §79.7). This skill provides the Bluetooth and NFC
domain knowledge that the `Android Data and Integration Worker` consumes.

## Trigger
This skill is requested when android Bluetooth and NFC — Bluetooth Low Energy (BLE) (GATT server, GATT client, scanning, connections), Classic Bluetooth (RFCOMM, SPP), NFC (NDEF reading/writing, HCE host card emulation), and Nearby Connections (BS §79.7). This skill provides the Bluetooth and NFC domain knowledge that the `Android Data and Integration Worker` consumes.. It does not replace a worker role — it supplies the domain
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
- bluetooth_adapter
- nfc_adapter
- gatt_server

## Procedure
1. Analyze connectivity requirements: identify the protocol (BLE, Classic
   Bluetooth, NFC), device roles (central, peripheral, reader, card),
   and data transfer needs.
2. Implement BLE scanning: use BluetoothLeScanner with ScanFilter
   and ScanSettings. Handle scan results in ScanCallback, stop
   scanning after timeout to save battery.
3. Implement BLE connections: use BluetoothGatt for GATT connections,
   discover services with discoverServices, read/write characteristics
   with readCharacteristic/writeCharacteristic.
4. Implement BLE peripheral: use BluetoothGattServer for GATT server,
   create BluetoothGattService with characteristics, handle read/write
   requests in BluetoothGattServerCallback.
5. Implement NFC: use NfcAdapter for NFC operations, NdefMessage
   for NDEF data, enableReaderMode for reader/writer mode, HostApduService
   for HCE.
6. Handle permissions: request BLUETOOTH_SCAN, BLUETOOTH_CONNECT,
   BLUETOOTH_ADVERTISE (API 31+), NFC permission. Handle location
   permission for BLE scanning (API 30 and below).
7. Test connectivity features: use BLE emulator for scanning/connections,
   test NFC with physical tags, verify permission flows.

## Evidence
- A record of each procedure step that executed, with the outcome observed,
  bound to the source revision and the environment fingerprint.
- The emitted `BluetoothNFCResult` (§23 SkillPackage contract), carrying its
  classification and its evidence references.
- Invariant claims this skill must leave observable, each of which is a
  statement the evidence above has to support:

  * BLE scanning is battery-intensive — stop scanning when not needed.
  *   Use ScanSettings.SCAN_MODE_LOW_POWER for background scanning.
  * BLE connections are limited — Android supports a limited number of
  *   simultaneous BLE connections. Disconnect when not needed.
  * NFC requires foreground — NFC operations require the app to be in the
  *   foreground. Use enableReaderMode for reliable NFC reading.
  * Bluetooth permissions changed in API 31 — BLUETOOTH_SCAN,
  *   BLUETOOTH_CONNECT, BLUETOOTH_ADVERTISE replaced the old
  *   BLUETOOTH and BLUETOOTH_ADMIN permissions.
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
Emits `BluetoothNFCResult` from `BluetoothNFCRequest` (§23 SkillPackage contract):

- classification: one of the failure classes above, null on success
- evidenceRefs: identifiers of the evidence this run captured
- revision and, where one exists, artifactDigest: what the evidence is bound to
- unverified: outcomes this run could not verify, named rather than assumed

## Fixtures
- Step 1 produces its expected outcome — Analyze connectivity requirements: identify the protocol (BLE, Classic
- Step 2 produces its expected outcome — Implement BLE scanning: use BluetoothLeScanner with ScanFilter
- Step 3 produces its expected outcome — Implement BLE connections: use BluetoothGatt for GATT connections,
- Step 4 produces its expected outcome — Implement BLE peripheral: use BluetoothGattServer for GATT server,
- Step 5 produces its expected outcome — Implement NFC: use NfcAdapter for NFC operations, NdefMessage
- Step 6 produces its expected outcome — Handle permissions: request BLUETOOTH_SCAN, BLUETOOTH_CONNECT,
- Step 7 produces its expected outcome — Test connectivity features: use BLE emulator for scanning/connections,
- A required capability is UNAVAILABLE — blocked, nothing attempted
- An invariant of this skill is violated and is reported, not absorbed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
