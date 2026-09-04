# Sensors & Battery

The CGD1 provides temperature, humidity, and battery data through three independent channels.

## Data Sources

| Mode | Data | Source | Requires Connection |
|---|---|---|---|
| Passive | Temperature, humidity, battery | BLE advertisements (`FDCD`) | No |
| Connected | Temperature, humidity (real-time) | Sensor Notify (`00000100-...`) | Yes |
| Connected | Battery (cached from advertising) | `KnownDeviceStore` battery cache | No (cached) |
| On-demand | Battery (unreliable) | GATT Battery Service (`0x180f` / `0x2a19`) | Yes |

> **Warning**: The GATT Battery Service characteristic (`0x2A19`) consistently returns 99% on the CGD1 and is **not reliable**. The controller and recommended connect flow use advertising-based battery data exclusively. The `read_battery()` method remains available for CLI/WS diagnostic use but is not used in the connect flow.

## Passive Sensor Stream (Advertising)

The device broadcasts sensor data in BLE advertisement packets via Service Data under UUID `0xFDCD`.

### Format

```
[08|88] 0C [MAC 6B] 01 04 [Temp 2B] [Humidity 2B] 02 01 [Battery]
```

| Field | Type | Scaling |
|---|---|---|
| Temperature | Int16 LE | / 10.0 (°C) |
| Humidity | UInt16 LE | / 10.0 (%RH) |
| Battery | UInt8 | & 0x7F (mask bit 7) |

> **Note**: The passive advertisement stream uses a scaling of / 10 (per Theengs decoder). The connected sensor stream uses / 100.0 (per clOwOck). This discrepancy may be firmware-dependent.

### Parsing

The `AdvertisementData::parse` method extracts temperature, humidity, battery, and MAC address from the raw service-data payload. This is used by `ClockScanner` during active scanning.

## Connected Sensor Stream (Notifications)

After connecting, the device sends real-time sensor data via the Sensor Notify characteristic (`00000100-0000-1000-8000-00805f9b34fb`).

### Format

```
[00] [Temp L] [Temp H] [Hum L] [Hum H]
```

5 bytes, starting with a constant `00`. This stream does **not** follow the length-byte framing.

| Field | Type | Scaling |
|---|---|---|
| Temperature | Signed Int16 LE | / 100.0 (°C) |
| Humidity | Unsigned UInt16 LE | / 100.0 (%RH) |

### Event Dispatch

The notification task parses sensor notifications and broadcasts `ClockEvent::SensorUpdate`. Battery is **not** included in sensor notifications (the CGD1 sends only 5 bytes without battery data):

```rust
pub enum ClockEvent {
    SensorUpdate { temperature: Temperature, humidity: Humidity },
    BatteryLevel { level: BatteryLevel },
    Disconnected,
    Reconnected,
    Ack { command: u8, status: AckStatus },
    Advertisement(AdvertisementData),
}
```

`ClockEvent::BatteryLevel` is sent from the controller's connect handler using the advertising battery cache, not from sensor notifications.

## Battery (Advertising Cache)

The CGD1 only advertises battery data when **not connected** and the button is held for 3 seconds. The battery level is encoded in advertising TLV type `0x02` as a single byte (masked with `0x7F`).

### KnownDeviceStore Battery Cache

The `KnownDeviceStore` persists battery levels from advertising scans in a separate `battery_cache.json` file, keyed by MAC address:

```json
{"58:2d:34:82:cc:81": 31}
```

- **`save_battery(address, level)`** — Called by the controller's scan callback when advertising battery data is received.
- **`load_battery()`** — Called on startup to populate the in-memory `scan_battery_cache`.

### Connect Flow Integration

The controller's connect handler reads the cached battery value from `scan_battery_cache` after a successful `connect_authenticate_and_sync` and sends `ClockEvent::BatteryLevel` to update the UI:

```rust
if let Some(level) = scan_battery_cache.lock().unwrap().get(&addr).copied() {
    let _ = event_tx.send(ClockEvent::BatteryLevel {
        level: BatteryLevel::new(level),
    });
}
```

### Device Behavior Notes

- The device **stops advertising** when connected, so battery data is only available passively.
- Advertising battery values can **fluctuate** between packets (e.g., 4%, 22%, 31% in the same scan window).
- Sensor notifications (5 bytes) do **not** contain battery data.
- Device settings responses do **not** contain battery data.

## Battery (GATT — Diagnostic Only)

The `read_battery()` method reads the standard GATT Battery Service characteristic (`0x2A19`). This is available for CLI and WebSocket diagnostic use but is **not used in the controller connect flow** because it consistently returns 99% on the CGD1.

```rust
// Diagnostic use only — unreliable on CGD1
let battery = device.read_battery().await?;
```

## CLI Usage

### Monitor sensors

```bash
cgd1 monitor AA:BB:CC:DD:EE:FF --duration 60
```

Streams temperature and humidity in real-time. Use `--duration 0` (default) for indefinite monitoring.

Output:

```
Monitoring AA:BB:CC:DD:EE:FF for 60s...
[2024-01-15T10:30:00] Temperature: 23.4 C  Humidity: 45.6 %
[2024-01-15T10:30:05] Temperature: 23.5 C  Humidity: 45.4 %
...
```

### Read battery

```bash
cgd1 battery AA:BB:CC:DD:EE:FF
```

Output:

```
Battery: 87 %
```

## Library API

### Subscribe to events

```rust
use cgd1_rs::ClockEvent;

let mut receiver = device.subscribe();

while let Ok(event) = receiver.recv().await {
    match event {
        ClockEvent::SensorUpdate { temperature, humidity } => {
            println!("Temperature: {:.1} C  Humidity: {:.1} %",
                temperature.value(), humidity.value());
        }
        ClockEvent::BatteryLevel { level } => {
            println!("Battery: {} %", level.value());
        }
        ClockEvent::Disconnected => {
            println!("Device disconnected");
            break;
        }
        ClockEvent::Reconnected => {
            println!("Device reconnected");
        }
        _ => {}
    }
}
```

### Read battery directly

```rust
let battery = device.read_battery().await?;
println!("Battery: {} %", battery);
```
