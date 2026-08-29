/// A CGD1 BLE protocol command identifier (single byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(u8);

impl CommandId {
    /// Create a new command ID from a raw byte.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Get the raw byte value.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for CommandId {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<CommandId> for u8 {
    fn from(cmd: CommandId) -> Self {
        cmd.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value() {
        let cmd = CommandId::new(0x09);
        assert_eq!(cmd.value(), 0x09);
    }

    #[test]
    fn from_u8() {
        let cmd: CommandId = 0x01.into();
        assert_eq!(cmd.value(), 0x01);
    }

    #[test]
    fn into_u8() {
        let cmd = CommandId::new(0x05);
        let raw: u8 = cmd.into();
        assert_eq!(raw, 0x05);
    }

    #[test]
    fn equality() {
        assert_eq!(CommandId::new(0x01), CommandId::new(0x01));
        assert_ne!(CommandId::new(0x01), CommandId::new(0x02));
    }
}
