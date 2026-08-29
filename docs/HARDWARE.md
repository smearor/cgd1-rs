# Qingping CGD1 Hardware Specifications

Hardware overview for the Qingping Bluetooth Alarm Clock (Model CGD1).

Sources:
- [Qingping Shop — Product Page](https://shop.qingping.co/products/qingping-bluetooth-alarm-clock)
- [Qingping — Overview](https://www.qingping.co/bluetooth-alarm-clock/overview)
- [Qingping — Specifications](https://qingping.co/bluetooth-alarm-clock/specifications)
- [Theengs Decoder — CGD1](https://decoder.theengs.io/devices/CGD1.html)
- [User Manual](https://manualspro.net/622733-qingping-bluetooth-alarm-clock-user-manual)

## General

| Property | Value |
|---|---|
| Brand | ClearGrass / Qingping |
| Model | Alarm Clock |
| Model ID | CGD1 (also referenced as CGC1/CGD1) |
| FCC ID | 2AQ3F-CGD1 |

## Design

| Property | Value                                 |
|---|---------------------------------------|
| Size | 80.3 × 41 × 83 mm                     |
| Weight | 99 g (without batteries) |
| Colors | Beige, Green, Blue                    |
| Screen size | 61 × 61 mm                            |
| Display | LCD with polarizing film              |
| Casing | Matte plastic                         |
| Base | Soft rubber (press-down mechanism)    |

## Power

| Property | Value |
|---|---|
| Battery | 2 × AA batteries |
| Standby time | > 1 year (varies with usage) |
| Power consumption | Low (BLE 5.0 low-power design) |

## Connectivity

| Property | Value |
|---|---|
| Wireless | Bluetooth 5.0 (BLE) |
| Frequency range | 2405–2480 MHz (2.4 GHz ISM band) |
| Max output power | ≤ 20 dBm |
| Advertisement | Service Data UUID `0xFDCD` (ClearGrass/Qingping) |
| Encryption | No (passive advertisements are unencrypted) |

## Sensors

The device uses a **Sensirion** sensor for temperature and humidity measurement.

| Sensor | Range | Notes |
|---|---|---|
| Temperature | -9.9 ~ 49.9 °C (14.2 ~ 121.8 °F) | Signed Int16 LE |
| Humidity | 0 ~ 99.9 %RH | Non-condensing environment; do not place in humidity > 90% for long-term use |

### Sensor Data Availability

| Mode | Data | Source |
|---|---|---|
| Passive (advertising) | Temperature, humidity, battery | BLE advertisements with `FDCD` service-data UUID — no connection required |
| Connected | Temperature, humidity (real-time) | Notify characteristic `00000100-...` |
| Connected | Battery | Standard GATT battery service (`0x180f` / `0x2a19`) |

### Passive Advertisement Format

The Theengs decoder identifies CGD1 advertisements by:
- Service data length = 34 hex characters (17 bytes)
- Byte at offset 1 = `0x0C` or `0x1E`

Decoded properties (hex string offsets):

| Property | Offset | Length | Type | Post-processing |
|---|---|---|---|---|
| Temperature | 20 | 4 hex (2 bytes) | Int16 LE, signed | / 10 |
| Humidity | 24 | 4 hex (2 bytes) | UInt16 LE, unsigned | / 10 |
| Battery | 32 | 2 hex (1 byte) | UInt8 | & 0x7F (mask bit 7) |
| MAC | 4 | 12 hex (6 bytes) | Reversed | — |

> **Note**: The clOwOck protocol specification documents a scaling of / 100.0 for the connected sensor stream. The Theengs decoder uses / 10 for the passive advertisement stream. This discrepancy may be firmware-dependent. See [BLE.md §7](BLE.md#7-real-time-sensor-stream-connected) and [BLE.md §8](BLE.md#8-passive-sensor-stream-advertising) for details.

## Alarm Features

| Feature | Details |
|---|---|
| Alarm slots | Up to 16 independent alarms |
| Repeat | Per-alarm day bitmask (Mon–Sun or once) |
| Snooze | Per-alarm enable/disable |
| Ringtones | 8 built-in ringtones |
| Custom ringtones | 2 custom slots (upload via BLE, ~12 s / 98 KB max) |
| Volume | 5 levels (adjustable via app) |
| Global alarm switch | Enable/disable all alarms at once |

## Display Features

| Feature | Details |
|---|---|
| Time format | 12-hour or 24-hour (configurable) |
| Date display | Yes |
| Temperature display | °C or °F (configurable) |
| Humidity display | %RH |
| Backlight | Adjustable brightness, press-to-light |
| Backlight duration | 0–30 seconds (configurable) |
| Night mode | Configurable time window with reduced brightness |
