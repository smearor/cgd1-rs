use crate::BatteryLevel;
use crate::ClockTime;
use crate::Humidity;
use crate::Temperature;
use crate::command::AlarmEntry;
use crate::command::Brightness;
use crate::command::DeviceSettings;
use crate::command::Language;
use crate::command::RingtoneSignature;
use crate::command::ScreenLightDuration;
use crate::command::TemperatureUnit;
use crate::command::TimeFormat;
use crate::command::Timezone;
use crate::command::Volume;
use std::time::Instant;

/// Number of alarm slots supported by the CGD1 device.
pub const ALARM_SLOT_COUNT: usize = 16;

/// Internal state of the virtual CGD1 device.
pub struct VirtualDeviceState {
    /// Alarm slots (16 slots, each 5 bytes or None if empty).
    pub alarms: Vec<Option<AlarmEntry>>,
    /// Device settings.
    pub settings: DeviceSettings,
    /// Battery level (0–100).
    pub battery: BatteryLevel,
    /// Current temperature in degrees Celsius.
    pub temperature: Temperature,
    /// Current relative humidity in percent.
    pub humidity: Humidity,
    /// Whether the device has been authenticated.
    pub authenticated: bool,
    /// The token accepted by the device (any token is accepted in virtual mode).
    pub token: Option<[u8; 16]>,
    /// Whether audio upload is in progress.
    pub audio_upload_active: bool,
    /// Total audio bytes expected in the current upload.
    pub audio_upload_total: usize,
    /// Audio bytes received so far.
    pub audio_upload_received: usize,
    /// Audio packets received in the current block (for ACK every 4 packets).
    pub audio_block_packets: usize,
    /// Unix timestamp set by the last Time Sync command.
    pub synced_time: Option<u32>,
    /// Instant when the last Time Sync command was received.
    pub synced_at: Option<Instant>,
}

impl Default for VirtualDeviceState {
    fn default() -> Self {
        let settings = DeviceSettings::new(
            Volume::new(3).unwrap(),
            TimeFormat::TwentyFourHour,
            TemperatureUnit::Celsius,
            Language::English,
            Timezone::from_hours(1).unwrap(),
            ScreenLightDuration::new(10).unwrap(),
            Brightness::new(80).unwrap(),
            Brightness::new(30).unwrap(),
            ClockTime::new(22, 0).unwrap(),
            ClockTime::new(7, 0).unwrap(),
            true,
            false,
            RingtoneSignature::Unused,
        )
        .unwrap();

        Self {
            alarms: vec![None; ALARM_SLOT_COUNT],
            settings,
            battery: BatteryLevel::new(85),
            temperature: Temperature::new(22.5),
            humidity: Humidity::new(55.0),
            authenticated: false,
            token: None,
            audio_upload_active: false,
            audio_upload_total: 0,
            audio_upload_received: 0,
            audio_block_packets: 0,
            synced_time: None,
            synced_at: None,
        }
    }
}
