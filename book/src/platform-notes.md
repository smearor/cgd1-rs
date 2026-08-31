# Platform Notes

## Linux

### BlueZ

Linux BLE access goes through BlueZ via D-Bus. The `btleplug` crate handles this internally, but BlueZ must be running and the user must have appropriate permissions.

**Requirements:**

- `bluez` package installed and running
- `dbus` running
- User has permission to access the BLE adapter

**Common issues:**

- **`Permission denied`**: Add the user to the `bluetooth` group or run with appropriate capabilities
- **`Adapter not found`**: Ensure Bluetooth is enabled in system settings
- **`Connection refused`**: Verify the D-Bus system socket is running (`systemctl status dbus`)

### GTK 4 Controller

The GTK 4 controller requires GTK 4 development libraries:

```bash
# Ubuntu/Debian
sudo apt-get install -y libgtk-4-dev

# Fedora
sudo dnf install -y gtk4-devel

# Arch
sudo pacman -S gtk4
```

## macOS

### CoreBluetooth

macOS uses CoreBluetooth for BLE access. No extra system packages are needed for the core library or CLI.

**Limitations:**

- The GTK 4 controller is not supported on macOS
- macOS requires Bluetooth to be enabled in System Settings
- The first BLE scan may prompt for Bluetooth permission

### Token Storage

Tokens are stored in `~/Library/Application Support/cgd1-rs/tokens/`.

## Windows

Windows uses the Windows Bluetooth API. The core library and CLI should work, but this is less tested than Linux.

**Requirements:**

- Windows 10 or later
- Bluetooth adapter enabled

## BLE Range and Stability

- **Range**: The CGD1 uses Bluetooth 5.0 with a typical indoor range of 10–15 meters
- **Interference**: 2.4 GHz Wi-Fi can cause BLE interference. If connections are unstable, try moving closer or switching Wi-Fi channels
- **Multiple connections**: The library supports multiple simultaneous device connections, but BLE bandwidth is shared

## Firmware Compatibility

Known firmwares:

- `1.0.1_0046`
- `1.0.1_0063`
- `1.0.1_0067`
- `1.0.1_0126`
- `1.0.1_0130`
- `1.0.1_0132`

If you encounter a new firmware version, please report it on [GitHub Issues](https://github.com/smearor/cgd1-rs/issues).

### Alarm Slot Count

The clOwOck specification documents 16 alarm slots. The ov1d1u Home Assistant integration uses 19 slots. This discrepancy may be firmware-dependent. The library defaults to 16 slots (indices 0–15).

## Battery Notes

- The CGD1 uses 2 × AA batteries with a standby time of over 1 year
- Battery level is available passively via advertisements and actively via the GATT battery service
- The passive advertisement battery byte has bit 7 masked (`& 0x7F`)

## Audio Upload Notes

- Audio uploads require a stable connection; parallel BLE operations can abort the transfer
- The MTU exchange is critical for audio uploads — without a sufficient MTU, packets would need fragmentation
- Maximum audio duration is approximately 12 seconds (~98 KB at 8 kHz, 8-bit mono)
