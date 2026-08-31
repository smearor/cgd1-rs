# Examples

## Cron Time Synchronization

Sync the device clock daily via cron:

```cron
# Sync CGD1 clock every day at 3:00 AM
0 3 * * * /home/user/.cargo/bin/cgd1 sync-time AA:BB:CC:DD:EE:FF
```

## Home Assistant Integration via WebSocket

Start the WebSocket server:

```bash
cgd1-ws --port 3000 &
```

Home Assistant custom component (Python):

```python
import asyncio
import json
import websockets

async def monitor_device():
    uri = "ws://localhost:3000/ws"
    async with websockets.connect(uri) as ws:
        # Connect to device
        await ws.send(json.dumps({
            "id": 1,
            "command": {"type": "connect", "address": "AA:BB:CC:DD:EE:FF"}
        }))
        print(await ws.recv())

        # Subscribe to sensor events
        await ws.send(json.dumps({
            "id": 2,
            "command": {"type": "subscribe_events", "address": "AA:BB:CC:DD:EE:FF"}
        }))
        print(await ws.recv())

        # Listen for events
        async for message in ws:
            data = json.loads(message)
            if data.get("event") == "sensor_update":
                temp = data["data"]["temperature"]
                humidity = data["data"]["humidity"]
                print(f"Temperature: {temp} C, Humidity: {humidity} %")

asyncio.run(monitor_device())
```

## Alarm Scheduling Script

Set up a weekday alarm using a shell script:

```bash
#!/bin/bash
DEVICE="AA:BB:CC:DD:EE:FF"

# Set weekday alarm at 07:00 in slot 0
cgd1 alarm-set "$DEVICE" 0 07:00 --repeat 3e

# Set weekend alarm at 08:30 in slot 1
cgd1 alarm-set "$DEVICE" 1 08:30 --repeat 41

# Verify
cgd1 alarm-list "$DEVICE"
```

## Batch Settings Configuration

Apply settings to multiple devices:

```bash
#!/bin/bash
DEVICES=(
    "AA:BB:CC:DD:EE:FF"
    "11:22:33:44:55:66"
)

for mac in "${DEVICES[@]}"; do
    echo "Configuring $mac..."
    cgd1 sync-time "$mac"
    cgd1 settings-write "$mac" \
        --volume 3 \
        --brightness 80 \
        --night-brightness 20 \
        --timezone 60 \
        --time-format 24 \
        --temp-unit C \
        --language en
done
```

## Library Usage: Full Device Setup

```rust
use cgd1_rs::{
    AlarmSlotIndex, Brightness, ClockManager, ClockTime, DayMask,
    BtleplugTransport, FileTokenStore, Language, MacAddress,
    TemperatureUnit, TimeFormat, TokenStore, Volume,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = Arc::new(BtleplugTransport::new().await?);
    let manager = ClockManager::new(transport.clone());
    let store = Arc::new(FileTokenStore::default_directory());

    let mac: MacAddress = "AA:BB:CC:DD:EE:FF".parse()?;
    let device = manager.connect(&mac).await?;

    // Authenticate
    let token_result = store.load_or_generate(&mac);
    device.set_token_store(store.clone() as Arc<dyn TokenStore>);
    device.authenticate(&token_result).await?;

    // Sync time
    device.sync_time_now().await?;
    if token_result.is_new() {
        store.save(&mac, &token_result)?;
    }

    // Set weekday alarm
    device.set_alarm(
        AlarmSlotIndex::new(0)?,
        ClockTime::new(7, 0)?,
        DayMask::WEEKDAYS,
        true,
        true,
    ).await?;

    // Configure settings
    let mut settings = device.read_settings().await?;
    settings.volume = Volume::new(3)?;
    settings.brightness = Brightness::new(80)?;
    settings.time_format = TimeFormat::TwentyFourHour;
    settings.temperature_unit = TemperatureUnit::Celsius;
    settings.language = Language::English;
    device.write_settings(&settings).await?;

    println!("Device configured successfully!");

    // Monitor sensors
    let mut receiver = device.subscribe();
    while let Ok(event) = receiver.recv().await {
        if let cgd1_rs::ClockEvent::SensorUpdate { temperature, humidity } = event {
            println!("Temperature: {:.1} C  Humidity: {:.1} %",
                temperature.value(), humidity.value());
        }
    }

    Ok(())
}
```

## WebSocket Server with systemd

Create a systemd service for the WebSocket server:

```ini
# /etc/systemd/system/cgd1-ws.service
[Unit]
Description=CGD1 WebSocket Server
After=bluetooth.target

[Service]
Type=simple
User=pi
ExecStart=/home/pi/.cargo/bin/cgd1-ws --port 3000
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable cgd1-ws
sudo systemctl start cgd1-ws
```
