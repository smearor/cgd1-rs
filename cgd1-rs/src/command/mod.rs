//! Command frame types for the CGD1 BLE protocol.
//!
//! This module defines the frame encoding/decoding types used by
//! [`crate::device::ClockDevice`]. Full command implementations are
//! added in Phase 2+.

mod ack;
mod ack_status;
mod alarm;
mod command_id;
mod commands;
mod frame;
mod settings;

pub use ack::Ack;
pub use ack_status::AckStatus;
pub use alarm::AlarmEntry;
pub use alarm::AlarmSlot;
pub use alarm::AlarmSlotIndex;
pub use alarm::AlarmSlotIndexParseError;
pub use alarm::DayMask;
pub use alarm::DayMaskParseError;
pub use command_id::CommandId;
pub use commands::Command;
pub use frame::CommandFrame;
pub use settings::Brightness;
pub use settings::BrightnessParseError;
pub use settings::DeviceSettings;
pub use settings::Language;
pub use settings::LanguageParseError;
pub use settings::RingtoneSignature;
pub use settings::RingtoneSignatureParseError;
pub use settings::ScreenLightDuration;
pub use settings::ScreenLightDurationParseError;
pub use settings::TemperatureUnit;
pub use settings::TemperatureUnitParseError;
pub use settings::TimeFormat;
pub use settings::TimeFormatParseError;
pub use settings::Timezone;
pub use settings::TimezoneParseError;
pub use settings::Volume;
pub use settings::VolumeParseError;
