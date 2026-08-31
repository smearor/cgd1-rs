# Sensors & Battery

The CGD1 provides temperature, humidity, and battery data through three independent channels.

## Data Sources

| Mode | Data | Source | Requires Connection |
|---|---|---|---|
| Passive | Temperature, humidity, battery | BLE advertisements (`FDCD`) | No |
| Connected | Temperature, humidity (real-time) | Sensor Notify (`00000100-...`) | Yes |
| Connected | Battery | GATT Battery Service (`0x180f` / `0x2a19`) | Yes |

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

The notification task parses sensor notifications and broadcasts `ClockEvent::SensorUpdate`:

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

## Battery (Connected)

Battery level is read from the standard GATT Battery Service:

- **Service UUID**: `0x180f`
- **Characteristic UUID**: `0x2a19`
- **Format**: 1 byte (percentage 0–100)

The library reads this characteristic and, when notifications are supported, subscribes for real-time battery updates. Battery changes are dispatched as `ClockEvent::BatteryLevel`.

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
