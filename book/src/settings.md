# Device Settings

The CGD1 stores all configuration in a single 18-byte settings payload, read and written via the Data characteristics.

## Settings Payload

```
13 01 [Vol] [Hdr1] [Hdr2] [Flags] [TZ] [Duration] [Brightness] [NightStartH] [NightStartM] [NightEndH] [NightEndM] [TzSign] [NightEn] [Reserved] [Sig 4B]
```

| Offset | Field | Bytes | Description |
|---|---|---|---|
| 0 | Header | 1 | `0x13` (length byte) |
| 1 | Command | 1 | `0x01` (set) or `0x02` (read response) |
| 2 | Volume | 1 | Sound volume (1–5) |
| 3 | Hdr1 | 1 | Fixed `0x58` |
| 4 | Hdr2 | 1 | Fixed `0x02` |
| 5 | Flags | 1 | Mode bitfield (see below) |
| 6 | Timezone | 1 | Offset in 6-minute units (minutes / 6) |
| 7 | Duration | 1 | Screen light duration in seconds |
| 8 | Brightness | 1 | Packed brightness (see below) |
| 9 | NightStartH | 1 | Night mode start hour (0–23) |
| 10 | NightStartM | 1 | Night mode start minute (0–59) |
| 11 | NightEndH | 1 | Night mode end hour (0–23) |
| 12 | NightEndM | 1 | Night mode end minute (0–59) |
| 13 | TzSign | 1 | `0x01` = positive, `0x00` = negative |
| 14 | NightEn | 1 | `0x01` = enabled, `0x00` = disabled |
| 15 | Reserved | 1 | Set to `0xFF` |
| 16–19 | Signature | 4 | Ringtone signature (`0xFFFFFFFF` when unused) |

### Flags Bitfield (Byte 5)

| Bit | Mask | Field | 0 | 1 |
|---|---|---|---|---|
| 0 | `0x01` | Language | Chinese | English |
| 1 | `0x02` | Time Format | 24-hour | 12-hour |
| 2 | `0x04` | Temperature Unit | Celsius | Fahrenheit |
| 4 | `0x10` | Alarms | Enabled | Disabled |

### Brightness Encoding (Byte 8)

Two nibbles packed into one byte:

```
High nibble = daytime_brightness / 10
Low nibble  = nighttime_brightness / 10
```

Each value must be 0–150 and a multiple of 10. Typical range is 0–100.

Example: Daytime 80%, Nighttime 30% → `(8 << 4) | 3 = 0x83`

### Night Mode Workaround

Disabling night mode is done by setting a 1-minute night mode window (`00:00`–`00:01`). Even the official app does this.

## Protocol

### Read Settings

Send `01 02` to Data Write. Response on Data Notify: `13 02 [Settings Payload 18B]`

### Write Settings

Send `13 01 [Settings Payload 18B]` to Data Write. ACK: `04 ff 01 00 00`

### Set Immediate Brightness (Preview)

Send `02 03 [Value]` to Data Write, where Value is brightness / 10 (0–15).

ACK: `04 ff 03 00 00`

### Preview Ringtone

Plays a generic beep sound for testing volume.

- Current volume: `01 04`
- Specific volume: `02 04 [Vol]` (volume 1–5)

ACK: `04 ff 04 00 00`

## CLI Usage

### Read settings

```bash
cgd1 settings-read AA:BB:CC:DD:EE:FF
```

### Write settings

Only specified fields are updated; unspecified fields are read from the device first and preserved:

```bash
cgd1 settings-write AA:BB:CC:DD:EE:FF \
    --volume 3 \
    --brightness 80 \
    --night-brightness 30 \
    --timezone 60 \
    --time-format 24 \
    --temp-unit C \
    --language en
```

| Flag | Values | Description |
|---|---|---|
| `--volume` | 1–5 | Sound volume |
| `--brightness` | 0–150 (multiple of 10) | Daytime brightness |
| `--night-brightness` | 0–150 (multiple of 10) | Nighttime brightness |
| `--timezone` | -720 to +840 (minutes) | Timezone offset |
| `--time-format` | 12 or 24 | Time display format |
| `--temp-unit` | C or F | Temperature unit |
| `--language` | en, zh, de, ja | Display language |

### Set brightness (preview)

```bash
cgd1 brightness AA:BB:CC:DD:EE:FF 80
```

### Preview ringtone

```bash
cgd1 ringtone-preview AA:BB:CC:DD:EE:FF --volume 3
```

## Library API

```rust
use cgd1_rs::{Brightness, Language, TemperatureUnit, TimeFormat, Timezone, Volume};

// Read current settings
let settings = device.read_settings().await?;
println!("Volume: {}", settings.volume);
println!("Brightness: {}", settings.brightness);

// Modify and write
let mut settings = device.read_settings().await?;
settings.volume = Volume::new(3)?;
settings.brightness = Brightness::new(80)?;
settings.time_format = TimeFormat::TwentyFourHour;
settings.temperature_unit = TemperatureUnit::Celsius;
settings.language = Language::English;
device.write_settings(&settings).await?;

// Set immediate brightness (preview)
device.set_brightness(Brightness::new(80)?).await?;

// Preview ringtone
device.preview_ringtone(Some(Volume::new(3)?)).await?;
```
