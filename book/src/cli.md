# CLI Tool

The `cgd1` command-line tool provides access to all device operations through subcommands. It uses `clap` for argument parsing and `miette` for rich error diagnostics.

## Installation

```bash
cargo install --path cgd1-rs-cli
```

## Global Options

```
Options:
  -v, --verbose...  Verbosity level (-v, -vv, -vvv)
      --backend <BACKEND>  BLE backend: `btleplug` (real hardware) or `virtual` (in-memory) [default: btleplug]
  -h, --help         Print help
  -V, --version      Print version
```

The `--backend virtual` flag uses an in-memory simulation instead of real BLE hardware. This is useful for testing and demos without a device.

## Subcommands

### scan

Scan for nearby CGD1 devices.

```bash
cgd1 scan --duration 10
```

| Flag | Default | Range | Description |
|---|---|---|---|
| `-d, --duration` | 10 | 1–600 | Scan duration in seconds |

Output includes MAC address, temperature, humidity, and battery from passive advertisements.

### sync-time

Synchronize the device clock to the current system time. This is the recommended first command after connecting, as it confirms the authentication token is accepted.

```bash
cgd1 sync-time AA:BB:CC:DD:EE:FF
```

### alarm-list

Read all 16 alarm slots from the device.

```bash
cgd1 alarm-list AA:BB:CC:DD:EE:FF
```

### alarm-set

Set or modify an alarm at a specific slot.

```bash
cgd1 alarm-set AA:BB:CC:DD:EE:FF 3 07:30 --repeat 3e --no-snooze
```

| Argument | Description |
|---|---|
| `address` | Device MAC address |
| `slot` | Slot index 0–15 |
| `time` | Alarm time in HH:MM format |
| `-r, --repeat` | Day mask as hex (default: `7f` = every day) |
| `--no-snooze` | Disable snooze |

### alarm-delete

Delete an alarm at a specific slot.

```bash
cgd1 alarm-delete AA:BB:CC:DD:EE:FF 3
```

### settings-read

Read all device settings.

```bash
cgd1 settings-read AA:BB:CC:DD:EE:FF
```

### settings-write

Write device settings. Only specified fields are updated; unspecified fields are preserved from the device.

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
| `--timezone` | -720 to +840 | Timezone offset in minutes |
| `--time-format` | 12 or 24 | Time display format |
| `--temp-unit` | C or F | Temperature unit |
| `--language` | en, zh, de, ja | Display language |

### brightness

Set immediate brightness (preview, not persisted).

```bash
cgd1 brightness AA:BB:CC:DD:EE:FF 80
```

### ringtone-preview

Play a preview beep sound on the device.

```bash
cgd1 ringtone-preview AA:BB:CC:DD:EE:FF --volume 3
```

### ringtone-upload

Upload a custom ringtone from a PCM file.

```bash
cgd1 ringtone-upload AA:BB:CC:DD:EE:FF audio.pcm --signature CustomSlotA
```

| Argument | Description |
|---|---|
| `address` | Device MAC address |
| `file` | Path to 8-bit PCM audio file (8 kHz, mono) |
| `-s, --signature` | Ringtone name or 4-byte hex (default: `CustomSlotA`) |

### firmware

Read the device firmware version.

```bash
cgd1 firmware AA:BB:CC:DD:EE:FF
```

### battery

Read the device battery level.

```bash
cgd1 battery AA:BB:CC:DD:EE:FF
```

### monitor

Monitor sensor data (temperature, humidity) in real-time.

```bash
cgd1 monitor AA:BB:CC:DD:EE:FF --duration 60
```

| Flag | Default | Description |
|---|---|---|
| `-d, --duration` | 0 | Duration in seconds (0 = indefinite) |

### repl

Start an interactive REPL session with a persistent connection. State changes (e.g., `settings-write`) are visible in subsequent commands (e.g., `settings-read`).

```bash
cgd1 repl --address AA:BB:CC:DD:EE:FF
```

If `--address` is omitted, use `connect <mac>` inside the REPL.

Available REPL commands mirror the CLI subcommands (without the `cgd1` prefix):

```
cgd1> help
Available commands:
  scan, sync-time, alarm-list, alarm-set, alarm-delete,
  settings-read, settings-write, brightness, ringtone-preview,
  ringtone-upload, firmware, battery, monitor, connect, disconnect, exit
```

## Token Management

The CLI automatically manages authentication tokens via `FileTokenStore`. Tokens are stored per MAC address in the platform's data directory:

- **Linux**: `~/.local/share/cgd1-rs/tokens/`
- **macOS**: `~/Library/Application Support/cgd1-rs/tokens/`

A new token is generated on first connection and persisted only after `sync-time` succeeds. Subsequent connections reuse the stored token.

If a token becomes invalid (e.g., the device was paired with a different app), delete the token file and run `sync-time` again to generate a new one.

## Error Handling

The CLI uses `miette` for rich error diagnostics. Errors include context, source spans, and suggestions:

```
Error: Authentication failed
  → The device rejected the authentication token.
  help: This may happen if the device was paired with a different app.
        Delete the token file and try again:
        rm ~/.local/share/cgd1-rs/tokens/AA_BB_CC_DD_EE_FF.bin
```

## Verbosity

The `-v` flag controls log output:

| Level | Output |
|---|---|
| (none) | Errors only |
| `-v` | Warnings + errors |
| `-vv` | Info + warnings + errors |
| `-vvv` | Debug (full trace) |
