use cgd1_rs::AlarmSlotIndex;
use cgd1_rs::Brightness;
use cgd1_rs::ClockTime;
use cgd1_rs::DayMask;
use cgd1_rs::DeviceSettings;
use cgd1_rs::MacAddress;
use cgd1_rs::ScanDuration;
use cgd1_rs::Volume;

use serde::Deserialize;

/// Supported WebSocket commands.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsCommand {
    /// Scan for devices.
    Scan {
        /// Scan duration in seconds (1–600).
        duration_secs: ScanDuration,
    },
    /// Connect to a device.
    Connect {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Disconnect from a device.
    Disconnect {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Synchronize device time to system time.
    SyncTime {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Read all alarms.
    ReadAlarms {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Set an alarm.
    SetAlarm {
        /// Device MAC address.
        address: MacAddress,
        /// Slot index (0–15).
        slot: AlarmSlotIndex,
        /// Alarm time (hour and minute).
        time: ClockTime,
        /// Repeat day bitmask.
        repeat_mask: DayMask,
        /// Whether the alarm is enabled.
        enabled: bool,
        /// Whether snooze is enabled.
        snooze: bool,
    },
    /// Delete an alarm.
    DeleteAlarm {
        /// Device MAC address.
        address: MacAddress,
        /// Slot index (0–15).
        slot: AlarmSlotIndex,
    },
    /// Read device settings.
    ReadSettings {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Write device settings.
    WriteSettings {
        /// Device MAC address.
        address: MacAddress,
        /// Settings to write.
        settings: DeviceSettings,
    },
    /// Set brightness preview.
    SetBrightness {
        /// Device MAC address.
        address: MacAddress,
        /// Brightness value.
        value: Brightness,
    },
    /// Preview ringtone.
    PreviewRingtone {
        /// Device MAC address.
        address: MacAddress,
        /// Optional volume level.
        volume: Option<Volume>,
    },
    /// Read firmware version.
    ReadFirmware {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Read battery level.
    ReadBattery {
        /// Device MAC address.
        address: MacAddress,
    },
    /// Subscribe to sensor events.
    SubscribeEvents {
        /// Device MAC address.
        address: MacAddress,
    },
}
