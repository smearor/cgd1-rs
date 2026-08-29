use crate::CharacteristicUuid;
use crate::command::CommandId;

/// CGD1 BLE protocol commands with known semantics.
///
/// The protocol reuses command byte values across different GATT
/// characteristics (Auth Write vs. Data Write), so the same byte
/// `0x01` means "Auth Init" on Auth Write but "Set Settings" on
/// Data Write. This enum disambiguates by giving each operation a
/// distinct variant.
///
/// Use [`Command::command_id`] to get the raw byte for framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Auth Init (`0x01` on Auth Write).
    AuthInit,
    /// Auth Confirm (`0x02` on Auth Write).
    AuthConfirm,
    /// Time Sync (`0x09` on Auth Write).
    TimeSync,
    /// Read Firmware (`0x0d` on Auth Write).
    ReadFirmware,
    /// Set Settings (`0x01` on Data Write).
    SetSettings,
    /// Read Settings (`0x02` on Data Write).
    ReadSettings,
    /// Set Brightness (`0x03` on Data Write).
    SetBrightness,
    /// Preview Ringtone (`0x04` on Data Write).
    PreviewRingtone,
    /// Read Alarms (`0x06` on Data Write).
    ReadAlarms,
    /// Set or Delete Alarm (`0x05` on Data Write).
    SetAlarm,
    /// Audio Init (`0x10` on Data Write).
    AudioInit,
    /// Audio Data Packet (`0x08` on Data Write).
    AudioData,
}

impl Command {
    /// Get the raw command ID byte for this command.
    pub const fn command_id(self) -> CommandId {
        match self {
            Self::AuthInit => CommandId::new(0x01),
            Self::AuthConfirm => CommandId::new(0x02),
            Self::TimeSync => CommandId::new(0x09),
            Self::ReadFirmware => CommandId::new(0x0d),
            Self::SetSettings => CommandId::new(0x01),
            Self::ReadSettings => CommandId::new(0x02),
            Self::SetBrightness => CommandId::new(0x03),
            Self::PreviewRingtone => CommandId::new(0x04),
            Self::ReadAlarms => CommandId::new(0x06),
            Self::SetAlarm => CommandId::new(0x05),
            Self::AudioInit => CommandId::new(0x10),
            Self::AudioData => CommandId::new(0x08),
        }
    }

    /// Get the GATT characteristic to write this command to.
    pub const fn characteristic(self) -> CharacteristicUuid {
        match self {
            Self::AuthInit | Self::AuthConfirm | Self::TimeSync | Self::ReadFirmware => CharacteristicUuid::AuthWrite,
            Self::SetSettings
            | Self::ReadSettings
            | Self::SetBrightness
            | Self::PreviewRingtone
            | Self::ReadAlarms
            | Self::SetAlarm
            | Self::AudioInit
            | Self::AudioData => CharacteristicUuid::DataWrite,
        }
    }
}

impl From<Command> for CommandId {
    fn from(cmd: Command) -> Self {
        cmd.command_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_init_id() {
        assert_eq!(Command::AuthInit.command_id(), CommandId::new(0x01));
    }

    #[test]
    fn auth_confirm_id() {
        assert_eq!(Command::AuthConfirm.command_id(), CommandId::new(0x02));
    }

    #[test]
    fn time_sync_id() {
        assert_eq!(Command::TimeSync.command_id(), CommandId::new(0x09));
    }

    #[test]
    fn read_firmware_id() {
        assert_eq!(Command::ReadFirmware.command_id(), CommandId::new(0x0d));
    }

    #[test]
    fn set_settings_id() {
        assert_eq!(Command::SetSettings.command_id(), CommandId::new(0x01));
    }

    #[test]
    fn read_settings_id() {
        assert_eq!(Command::ReadSettings.command_id(), CommandId::new(0x02));
    }

    #[test]
    fn set_brightness_id() {
        assert_eq!(Command::SetBrightness.command_id(), CommandId::new(0x03));
    }

    #[test]
    fn preview_ringtone_id() {
        assert_eq!(Command::PreviewRingtone.command_id(), CommandId::new(0x04));
    }

    #[test]
    fn read_alarms_id() {
        assert_eq!(Command::ReadAlarms.command_id(), CommandId::new(0x06));
    }

    #[test]
    fn set_alarm_id() {
        assert_eq!(Command::SetAlarm.command_id(), CommandId::new(0x05));
    }

    #[test]
    fn audio_init_id() {
        assert_eq!(Command::AudioInit.command_id(), CommandId::new(0x10));
    }

    #[test]
    fn audio_data_id() {
        assert_eq!(Command::AudioData.command_id(), CommandId::new(0x08));
    }

    #[test]
    fn into_command_id() {
        let id: CommandId = Command::TimeSync.into();
        assert_eq!(id, CommandId::new(0x09));
    }
}
