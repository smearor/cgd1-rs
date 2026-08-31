# Getting Started

## Prerequisites

### Rust

A working Rust toolchain is required. The project uses Rust Edition 2024.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Minimum Supported Rust Version (MSRV)

Each crate declares its own MSRV in its `Cargo.toml`:

| Crate | MSRV |
|---|---|
| `cgd1-rs` | 1.88 |
| `cgd1-rs-cli` | 1.89 |
| `cgd1-rs-ws` | 1.88 |
| `cgd1-rs-controller` | 1.92 |

### Linux System Dependencies

The project requires BlueZ for BLE access via `btleplug`. The GTK 4 controller additionally requires GTK 4 development libraries.

**Ubuntu/Debian:**

```bash
# Core + CLI + WebSocket server
sudo apt-get install -y pkg-config libdbus-1-dev

# GTK 4 controller (optional)
sudo apt-get install -y libgtk-4-dev

# mdBook documentation (optional)
cargo install mdbook
cargo install mdbook-mermaid
```

**Fedora:**

```bash
sudo dnf install -y pkg-config dbus-devel gtk4-devel
```

**Arch Linux:**

```bash
sudo pacman -S dbus gtk4
```

### macOS

macOS uses CoreBluetooth internally. No extra system packages are needed for the core library or CLI. The GTK 4 controller is not supported on macOS.

```bash
brew install gtk4
```

## Building from Source

Clone the repository and build all crates:

```bash
git clone https://github.com/smearor/cgd1-rs.git
cd cgd1-rs
cargo build --release
```

To build only a specific crate:

```bash
cargo build --release -p cgd1-rs-cli
cargo build --release -p cgd1-rs-ws
cargo build --release -p cgd1-rs-controller
```

## Installation

### From Source

```bash
cargo install --path cgd1-rs-cli
cargo install --path cgd1-rs-ws
```

This installs the `cgd1` and `cgd1-ws` binaries into `~/.cargo/bin/`.

### From crates.io

_Not yet published. Build from source for now._

## Quick Start

### 1. Scan for Devices

```bash
cgd1 scan --duration 10
```

Output:

```
Scanning for 10s...
Found 1 device(s):
  MAC: AA:BB:CC:DD:EE:FF
    Temperature: 23.4 C
    Humidity: 45.6 %
    Battery: 87 %
```

### 2. Synchronize Time

The first command after scanning should be time synchronization. This also confirms that the authentication token is accepted by the device.

```bash
cgd1 sync-time AA:BB:CC:DD:EE:FF
```

### 3. Read Alarms

```bash
cgd1 alarm-list AA:BB:CC:DD:EE:FF
```

### 4. Set an Alarm

```bash
cgd1 alarm-set AA:BB:CC:DD:EE:FF 3 07:30 --repeat 3e --no-snooze
```

This sets alarm slot 3 to 07:30, repeating on weekdays (Mon–Fri), with snooze disabled.

### 5. Monitor Sensors

```bash
cgd1 monitor AA:BB:CC:DD:EE:FF --duration 60
```

Streams temperature and humidity data for 60 seconds.

## Running Tests

```bash
# All tests (unit + integration)
cargo test --all

# Only core library tests
cargo test -p cgd1-rs

# CLI integration tests
cargo test -p cgd1-rs-cli

# WebSocket integration tests
cargo test -p cgd1-rs-ws
```

All 240 tests run without hardware. Hardware-dependent tests are behind `#[ignore]` and can be run with `cargo test -- --ignored`.

## Building the Documentation

```bash
cd book
mdbook build
```

The rendered HTML is placed in `book/book/`. Open `book/book/index.html` in a browser, or run `mdbook serve` for live preview during development.
