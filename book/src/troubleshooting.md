# Troubleshooting

## BLE Connection Issues

### Device not found during scan

**Symptom**: `cgd1 scan` returns no devices.

**Solutions**:

1. Ensure the CGD1 is powered on (batteries inserted)
2. Move closer to the device (within 5 meters)
3. Verify Bluetooth is enabled on the host:
   ```bash
   bluetoothctl power on
   ```
4. Check that the BLE adapter is available:
   ```bash
   hciconfig
   ```
5. Stop other BLE applications that may be holding the adapter (e.g., other scanning tools)

### Connection fails

**Symptom**: `Error: Transport(NotConnected)` or `Error: Transport(Timeout)`.

**Solutions**:

1. Ensure the device is not currently connected to another host (the CGD1 supports only one active BLE connection)
2. Restart Bluetooth:
   ```bash
   sudo systemctl restart bluetooth
   ```
3. Remove any existing pairing from the OS Bluetooth manager:
   ```bash
   bluetoothctl remove AA:BB:CC:DD:EE:FF
   ```
4. Try connecting again after a few seconds

### Connection drops unexpectedly

**Symptom**: `ClockEvent::Disconnected` events or `Error: Transport(Timeout)` during operations.

**Solutions**:

1. Check battery level — low batteries can cause disconnections
2. Reduce distance between host and device
3. Avoid 2.4 GHz Wi-Fi interference (switch to 5 GHz or change Wi-Fi channel)
4. The library will attempt automatic reconnection with exponential backoff

## Authentication Issues

### Authentication fails on first connection

**Symptom**: `Error: AuthFailed` with `is_new_token: true`.

**Solutions**:

1. This should not happen on a fresh device. Ensure you are connecting to the correct MAC address
2. Try generating a new token by deleting the token file:
   ```bash
   rm ~/.local/share/cgd1-rs/tokens/AA_BB_CC_DD_EE_FF.bin
   ```
3. Run `sync-time` again to generate and store a new token

### Authentication fails after previously working

**Symptom**: `Error: AuthFailed` with `is_new_token: false`.

**Cause**: The device was likely paired with a different app (e.g., the official Qingping app), which overwrites the authentication token.

**Solutions**:

1. Delete the stored token:
   ```bash
   rm ~/.local/share/cgd1-rs/tokens/AA_BB_CC_DD_EE_FF.bin
   ```
2. Reconnect with `sync-time` to generate a new token
3. Note: The official app and `cgd1-rs` cannot share the same token. Using one will invalidate the other's token

### `sync-time` succeeds but other commands fail

**Symptom**: `sync-time` works, but `alarm-set` or `settings-write` returns errors.

**Solutions**:

1. Verify the device is still connected (`cgd1 battery <mac>`)
2. Check that the token was persisted (look for the token file in `~/.local/share/cgd1-rs/tokens/`)
3. Try disconnecting and reconnecting

## Audio Upload Issues

### Upload fails with MTU error

**Symptom**: `Error: MtuTooSmall { mtu: ... }`.

**Solutions**:

1. The device or host does not support a sufficient MTU. This is a hardware limitation
2. Try restarting Bluetooth and reconnecting
3. Some BLE adapters negotiate a lower MTU on first connection; disconnect and reconnect

### Upload aborts mid-transfer

**Symptom**: Upload starts but fails partway through.

**Solutions**:

1. Ensure no other BLE operations are running concurrently (alarm reads, settings reads, RSSI polling)
2. Keep the device close to the host during the entire transfer
3. Verify the audio file is valid 8-bit unsigned PCM at 8 kHz mono
4. Check the file size is under 98 KB

### Audio plays incorrectly after upload

**Solutions**:

1. Verify the source audio is 8-bit unsigned PCM (not signed, not 16-bit)
2. Verify the sample rate is exactly 8000 Hz
3. Verify the audio is mono
4. Try alternating the signature slot (`CustomSlotA` → `CustomSlotB`)

## GTK Controller Issues

### Application fails to start

**Symptom**: `error: failed to run command: cgd1-rs-controller` or GTK warnings.

**Solutions**:

1. Verify GTK 4 is installed:
   ```bash
   pkg-config --modversion gtk4
   ```
2. Check for missing CSS or font resources

### Sensor cards not updating

**Solutions**:

1. Verify the device is connected (check the sidebar)
2. Try disconnecting and reconnecting
3. Check the application logs with verbosity enabled

## Virtual Backend

### Using the virtual backend for testing

The `--backend virtual` flag uses an in-memory device simulation:

```bash
cgd1 --backend virtual scan
cgd1 --backend virtual sync-time AA:BB:CC:DD:EE:FF
cgd1 --backend virtual alarm-list AA:BB:CC:DD:EE:FF
```

This works without any BLE hardware and is useful for testing CLI behavior, scripts, and the WebSocket server.

## Reporting Issues

If you encounter a bug or have a feature request, please open an issue on [GitHub](https://github.com/smearor/cgd1-rs/issues).

Include:

- The command or code that triggered the issue
- The full error output (use `-vvv` for maximum verbosity)
- Your OS and Bluetooth adapter model
- The device firmware version (`cgd1 firmware <mac>`)
