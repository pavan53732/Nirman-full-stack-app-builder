# Android Bluetooth NFC Expert

Scope: Android Bluetooth and NFC — Bluetooth Low Energy (BLE) (GATT server,
GATT client, scanning, connections), Classic Bluetooth (RFCOMM, SPP),
NFC (NDEF reading/writing, HCE host card emulation), and Nearby
Connections (BS §79.7). This skill provides the Bluetooth and NFC
domain knowledge that the `Android Data and Integration Worker` consumes.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the blocked state MUST be reported. This skill does not
replace the `Android Data and Integration Worker` role — it provides
Bluetooth/NFC-specific instruction.

## Workflow
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

## Invariants
- BLE scanning is battery-intensive — stop scanning when not needed.
  Use ScanSettings.SCAN_MODE_LOW_POWER for background scanning.
- BLE connections are limited — Android supports a limited number of
  simultaneous BLE connections. Disconnect when not needed.
- NFC requires foreground — NFC operations require the app to be in the
  foreground. Use enableReaderMode for reliable NFC reading.
- Bluetooth permissions changed in API 31 — BLUETOOTH_SCAN,
  BLUETOOTH_CONNECT, BLUETOOTH_ADVERTISE replaced the old
  BLUETOOTH and BLUETOOTH_ADMIN permissions.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
