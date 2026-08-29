use crate::command::Command;
use crate::command::CommandId;

/// A command frame for the CGD1 BLE protocol.
///
/// Frame format: `[length] [command] [payload...]`.
///
/// The length byte includes itself but not the command byte, following
/// the CGD1 protocol convention documented in `docs/BLE.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFrame {
    /// The command byte (e.g., `0x01` for Auth Init, `0x09` for Time Sync).
    pub command: CommandId,
    /// The payload bytes (excluding length and command bytes).
    pub payload: Vec<u8>,
}

impl CommandFrame {
    /// Create a new command frame from a typed [`Command`] and payload.
    ///
    /// The length byte is computed automatically during [`encode`](Self::encode).
    pub fn from_command(command: Command, payload: Vec<u8>) -> Self {
        Self {
            command: command.command_id(),
            payload,
        }
    }

    /// Create a new command frame with the given command ID and payload.
    pub fn new(command: CommandId, payload: Vec<u8>) -> Self {
        Self { command, payload }
    }

    /// Encode the frame into bytes for BLE transmission.
    ///
    /// The length byte is calculated as `1 + payload.len()` (length byte
    /// itself plus payload, excluding the command byte).
    pub fn encode(&self) -> Vec<u8> {
        let mut frame = Vec::with_capacity(2 + self.payload.len());
        frame.push(1 + self.payload.len() as u8);
        frame.push(self.command.value());
        frame.extend_from_slice(&self.payload);
        frame
    }

    /// Decode a frame from raw bytes.
    ///
    /// Returns `None` if the input is too short or the length byte
    /// does not match the actual payload size.
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let length = data[0] as usize;
        let command = CommandId::new(data[1]);
        // Length byte includes itself but not the command byte.
        let expected_payload_len = length.saturating_sub(1);
        if data.len() < 2 + expected_payload_len {
            return None;
        }
        let payload = data[2..2 + expected_payload_len].to_vec();
        Some(Self { command, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_auth_init() {
        let frame = CommandFrame::new(CommandId::new(0x01), vec![0xAA; 16]);
        let encoded = frame.encode();
        // Length = 1 + 16 = 17 (0x11)
        assert_eq!(encoded[0], 0x11);
        assert_eq!(encoded[1], 0x01);
        assert_eq!(&encoded[2..], &[0xAA; 16]);
    }

    #[test]
    fn from_command_auth_init() {
        let frame = CommandFrame::from_command(Command::AuthInit, vec![0xAA; 16]);
        let encoded = frame.encode();
        assert_eq!(encoded[0], 0x11);
        assert_eq!(encoded[1], 0x01);
        assert_eq!(&encoded[2..], &[0xAA; 16]);
    }

    #[test]
    fn from_command_time_sync() {
        let frame = CommandFrame::from_command(Command::TimeSync, vec![0x78, 0x56, 0x34, 0x12]);
        let encoded = frame.encode();
        assert_eq!(encoded[0], 0x05);
        assert_eq!(encoded[1], 0x09);
        assert_eq!(&encoded[2..], &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn encode_time_sync() {
        let frame = CommandFrame::new(CommandId::new(0x09), vec![0x00, 0x00, 0x00, 0x00]);
        let encoded = frame.encode();
        // Length = 1 + 4 = 5 (0x05)
        assert_eq!(encoded[0], 0x05);
        assert_eq!(encoded[1], 0x09);
        assert_eq!(&encoded[2..6], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn decode_roundtrip() {
        let original = CommandFrame::new(CommandId::new(0x09), vec![0x78, 0x56, 0x34, 0x12]);
        let encoded = original.encode();
        let decoded = CommandFrame::decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_too_short() {
        assert!(CommandFrame::decode(&[0x05]).is_none());
    }

    #[test]
    fn decode_length_mismatch() {
        // Length says 5 (1 + 4 payload), but only 2 bytes follow
        let data = [0x05, 0x09, 0x00];
        assert!(CommandFrame::decode(&data).is_none());
    }
}
