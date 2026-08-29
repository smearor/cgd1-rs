use crate::command::AckStatus;
use crate::command::CommandId;

/// Parsed ACK frame from a BLE notification.
///
/// ACK format: `04 ff [command] [status] [payload]` (5 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ack {
    /// The command byte that was acknowledged.
    pub command: CommandId,
    /// The status of the acknowledged command.
    pub status: AckStatus,
    /// The single payload byte.
    pub payload: u8,
}

impl Ack {
    /// Parse an ACK from a raw notification value.
    ///
    /// Returns `None` if the value is too short or does not have the
    /// ACK prefix (`04 ff`).
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() >= 5 && value[0] == 0x04 && value[1] == 0xff {
            Some(Self {
                command: CommandId::new(value[2]),
                status: AckStatus::from_byte(value[3]),
                payload: value[4],
            })
        } else {
            None
        }
    }

    /// Whether the ACK indicates success.
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_ack() {
        let value = [0x04, 0xff, 0x01, 0x00, 0x06];
        let ack = Ack::parse(&value).unwrap();
        assert_eq!(ack.command, CommandId::new(0x01));
        assert_eq!(ack.status, AckStatus::Success);
        assert_eq!(ack.payload, 0x06);
        assert!(ack.is_success());
    }

    #[test]
    fn parse_ack_failure_status() {
        let value = [0x04, 0xff, 0x09, 0x01, 0x00];
        let ack = Ack::parse(&value).unwrap();
        assert_eq!(ack.status, AckStatus::Failure(0x01));
        assert!(!ack.is_success());
    }

    #[test]
    fn parse_too_short() {
        let value = [0x04, 0xff, 0x01];
        assert!(Ack::parse(&value).is_none());
    }

    #[test]
    fn parse_wrong_prefix() {
        let value = [0x05, 0xff, 0x01, 0x00, 0x06];
        assert!(Ack::parse(&value).is_none());
    }
}
