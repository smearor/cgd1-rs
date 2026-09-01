# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

### Changed

### Fixed

### Distribution

### Infrastructure


## 0.1.0 - 2026-09-01

### Added

- **Phase 0 — Project Setup**: Cargo workspace with 4 members (`cgd1-rs`,
  `cgd1-rs-cli`, `cgd1-rs-controller`, `cgd1-rs-ws`)
- **Phase 1 — BLE Transport**: `BleTransport` trait with `BtleplugTransport`
  (real hardware), `MockBleTransport` (unit tests), and
  `VirtualClockTransport` (in-memory device simulation)
  - `ClockScanner` for active BLE scanning with advertisement parsing
    (temperature, humidity, battery from `FDCD` service data)
  - `ClockManager` for multi-device connection management
  - GATT service/characteristic discovery and subscription
  - Frame-based protocol with length-prefixed commands and ACK matching
  - Automatic reconnection with exponential backoff and full state recovery
- **Phase 2 — Authentication & Time Sync**: Two-step token handshake
  (Auth Init + Auth Confirm) on Auth characteristics
  - `FileTokenStore` for per-MAC token persistence in platform data
    directories
  - Token persistence rule: new token saved only after privileged command
    succeeds
  - `AuthFailedError` with actionable context (reason, is_new_token,
    token_path)
  - Time synchronization via `sync_time_now()`
  - Firmware version query via `read_firmware()`
- **Phase 3 — Alarm Management**: Read, set, and delete up to 16 alarm
  slots
  - `AlarmEntry` (5 bytes: enabled, hour, minute, day mask, snooze)
  - `DayMask` newtype with constants (`ONCE`, `EVERY_DAY`, `WEEKDAYS`,
    `WEEKENDS`)
  - `AlarmSlotIndex` (0–15), `ClockTime` (HH:MM) newtypes
  - Multi-packet alarm read (6 packets × 3 entries)
- **Phase 4 — Device Settings**: Read and write 18-byte settings payload
  - Volume (1–5), brightness (0–150, packed nibbles), timezone (-720 to
    +840 minutes), time format (12/24h), temperature unit (°C/°F),
    language (en/zh/de/ja), night mode window, screen duration
  - `Brightness`, `Volume`, `Timezone`, `TimeFormat`, `TemperatureUnit`,
    `Language` newtypes
  - Immediate brightness preview and ringtone preview commands
- **Phase 5 — Sensors & Battery**: Three sensor data sources
  - Passive: advertisement parsing with `FDCD` service data
  - Connected: real-time sensor notifications (Sensor Notify characteristic)
  - Battery: standard GATT Battery Service (`0x180f` / `0x2a19`)
  - `ClockEvent` enum with `SensorUpdate`, `BatteryLevel`, `Disconnected`,
    `Reconnected`, `Ack`, `Advertisement` variants
  - `Temperature`, `Humidity`, `BatteryLevel` newtypes
- **Phase 6 — Audio Upload**: Custom ringtone upload via block-based BLE
  protocol
  - 8-bit unsigned PCM, 8 kHz, mono, max ~98 KB (~12 seconds)
  - MTU exchange (247 bytes requested, 130 minimum)
  - Block-based transfer: 4 packets × 128 bytes, block ACK after each
  - `RingtoneSignature` newtype with 9 built-in ringtones and 2 custom
    slots (`CustomSlotA`, `CustomSlotB`)
  - Audio validation and padding to 512-byte multiples
- **Phase 7 — CLI Tool**: `cgd1` binary with 14 subcommands
  - `scan`, `sync-time`, `alarm-list`, `alarm-set`, `alarm-delete`,
    `settings-read`, `settings-write`, `brightness`, `ringtone-preview`,
    `ringtone-upload`, `firmware`, `battery`, `monitor`, `repl`
  - `--backend` flag (`btleplug` or `virtual`) for hardware vs. simulation
  - Interactive REPL with persistent connection
  - `miette` error diagnostics with context and suggestions
  - Verbosity levels (`-v`, `-vv`, `-vvv`)
- **Phase 8 — GTK 4 Controller**: `cgd1-controller` desktop application
  - `ClockControllerApp` (gtk4::Application subclass)
  - Sidebar device management with tabbed content area
  - Custom seven-segment clock display (DSEG7 font)
  - Dialog system: alarms, audio, info, settings
  - Event loop integration via `glib::MainContext::spawn_local` with
    `CancellationToken`
  - CSS styling for sensor cards, alarm rows, and settings panel
- **Phase 9 — WebSocket Server**: `cgd1-ws` binary with Axum
  - WebSocket endpoint for command/response and event streaming
  - REST API for read-only operations (sensors, battery, firmware, alarms,
    settings)
  - `ServerState` with shared transport, manager, and token store
  - `WsSession` per connection with concurrent command handling
  - JSON protocol with `id`-based request/response matching
  - Event subscription with push notifications
- **Phase 10 — Documentation**: Full mdBook user guide with 16 chapters
  - Introduction, Getting Started, Architecture, BLE Protocol, Scanning &
    Connecting, Authentication, Alarms, Device Settings, Sensors & Battery,
    Audio Upload, CLI Tool, Controller, WebSocket Server, Platform Notes,
    Examples, Troubleshooting
  - Mermaid diagrams throughout
  - README.md with badges (crates.io, CI, MSRV, audit, book, license,
    edition)

### Fixed

### Distribution

- All 4 crates ready for crates.io publication
  - `cgd1-rs` (core library)
  - `cgd1-rs-cli` (CLI tool)
  - `cgd1-rs-controller` (GTK 4 app)
  - `cgd1-rs-ws` (WebSocket server)
- Workspace metadata: `homepage`, `repository`, `license`, `edition`,
  `rust-version`
- Per-crate metadata: `description`, `keywords`, `categories`,
  `documentation` (docs.rs links)

### Infrastructure

- Bumped all GitHub Actions CI runners from `ubuntu-24.04` to
  `ubuntu-26.04` (including `ubuntu-24.04-arm` → `ubuntu-26.04-arm`)
- 240 tests across all crates, all running without hardware via
  `VirtualClockTransport` and `MockBleTransport`

