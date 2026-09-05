# Scanning & Connecting

## Device Discovery

The CGD1 broadcasts BLE advertisements with the ClearGrass/Qingping service-data UUID `0xFDCD`. These advertisements carry sensor data (temperature, humidity, battery) and the device MAC address, allowing discovery without a connection.

### Scanning with the CLI

```bash
cgd1 scan --duration 10
```

The `--duration` flag accepts values from 1 to 600 seconds (default: 10).

Output:

```
Scanning for 10s...
Found 1 device(s):
  MAC: AA:BB:CC:DD:EE:FF
    Temperature: 23.4 C
    Humidity: 45.6 %
    Battery: 87 %
```

### Scanning with the Library

```rust
use cgd1_rs::{BtleplugTransport, ClockScanner, BleTransport};
use std::sync::Arc;
use std::time::Duration;

let transport = Arc::new(BtleplugTransport::new().await?);
let scanner = ClockScanner::new(transport);

let devices = scanner.scan_active(Duration::from_secs(10)).await?;

for device in &devices {
    println!("MAC: {}", device.address);
    if let Some(ad) = &device.advertisement {
        println!("  Temperature: {:.1} C", ad.temperature.value());
        println!("  Humidity: {:.1} %", ad.humidity.value());
        println!("  Battery: {} %", ad.battery.value());
    }
}
```

### Advertisement Data

The `AdvertisementData` struct is parsed from the raw service-data payload:

| Field | Type | Scaling |
|---|---|---|
| MAC | 6 bytes (reversed) | - |
| Temperature | Int16 BE | / 10 (°C) |
| Humidity | UInt16 BE | / 10 (%) |
| Battery | UInt8 | & 0x7F (mask bit 7) |

## Connecting

### Connection Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Manager as ClockManager
    participant Transport as BleTransport
    participant CGD1 as CGD1 Device

    App->>Manager: connect_authenticate_and_sync(mac, token)
    Manager->>Transport: connect(address)
    Transport->>CGD1: BLE connection
    CGD1-->>Transport: Connected
    Manager->>Transport: subscribe(Auth/Data/Sensor Notify)
    Manager->>Manager: spawn notification task
    Manager->>Manager: set_token_store(token_store)
    Manager->>CGD1: authenticate(token)
    CGD1-->>Manager: Auth ACKs
    Manager->>CGD1: sync_timezone()
    CGD1-->>Manager: Settings response
    Manager->>CGD1: sync_time_now()
    CGD1-->>Manager: TimeSync ACK
    Manager-->>App: ClockDevice (ready)
```

The full `connect_authenticate_and_sync` flow performs:

1. **BLE connect** - `transport.connect(address)`
2. **Subscribe** to Auth Notify, Data Notify, and Sensor Notify characteristics
3. **Spawn notification task** - Background task for processing BLE notifications
4. **Authenticate** - Two-step token handshake (Auth Init + Auth Confirm)
5. **Sync timezone** - Read device settings, compute local UTC offset, write correct timezone
6. **Sync time** - Send current Unix timestamp; token is persisted only after this succeeds

### Connecting with the Library

```rust
use cgd1_rs::{BtleplugTransport, ClockManager, MacAddress};
use std::sync::Arc;

let transport = Arc::new(BtleplugTransport::new().await?);
let manager = ClockManager::new(transport);

let mac: MacAddress = "AA:BB:CC:DD:EE:FF".parse()?;
let device = manager.connect(&mac).await?;
```

`ClockManager` tracks connected devices by MAC address. Calling `connect` on an already-connected device returns the existing handle.

### Multi-Device Management

`ClockManager` supports simultaneous connections to multiple CGD1 devices:

```rust
let device_a = manager.connect(&mac_a).await?;
let device_b = manager.connect(&mac_b).await?;

// Both devices are now connected and can be operated independently
device_a.sync_time_now().await?;
device_b.read_alarms().await?;
```

### Disconnecting

```rust
manager.disconnect(&mac).await?;
```

This aborts the notification task (via `JoinHandle::abort()`) and tears down the BLE connection. Aborting the notification task is critical - without it, a zombie task from a failed connection can steal notifications from a subsequent connection to the same device.

### Automatic Reconnection

When the BLE connection drops (e.g., device goes out of range, battery dies, or alarm triggers a disconnect), the notification task automatically attempts reconnection with exponential backoff:

1. **Disconnect detected** - The notification stream ends or a `CentralEvent::DeviceDisconnected` is received
2. **Transport cleanup** - `transport.disconnect()` clears the connection state to avoid `AlreadyConnected` errors on retry
3. **Reconnect attempts** - Up to 10 attempts with exponential backoff (1s, 2s, 4s, 8s, 16s, 32s capped)
4. **BLE connect + subscribe** - `reconnect_and_restore` reconnects and re-subscribes to all notify characteristics
5. **Re-authentication** - A separate task re-authenticates using the stored token (spawned concurrently so the notification loop can process ACKs)
6. **State recovery** - On success, `ClockEvent::Reconnected` is emitted

The `connect()` call has a 10-second timeout to prevent hanging when the device is unavailable (e.g., during an alarm). The device typically becomes discoverable again ~15-20 seconds after an alarm-triggered disconnect.

> **Note**: While the alarm is sounding, the CGD1 is not discoverable. Dismissing the alarm quickly allows faster reconnection.

### Virtual Backend (Testing)

For testing without hardware, use the `virtual` backend:

```bash
cgd1 --backend virtual scan
cgd1 --backend virtual sync-time AA:BB:CC:DD:EE:FF
```

The virtual backend simulates a CGD1 device in memory, responding to all commands with appropriate ACKs and maintaining alarm/settings state.
