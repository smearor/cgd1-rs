use crate::error::ClockError;
use crate::error::Result;

use super::day_mask::DayMask;
use super::slot_index::AlarmSlotIndex;

/// A single alarm entry.
///
/// Maps to the 5-byte alarm structure used in the CGD1 BLE protocol:
/// `[Enabled] [HH] [MM] [Days] [Snooze]`.
///
/// Invariants are enforced by construction:
/// - `hour` is in range 0–23
/// - `minute` is in range 0–59
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmEntry {
    /// Hour (0–23).
    hour: u8,
    /// Minute (0–59).
    minute: u8,
    /// Day-of-week repeat bitmask.
    repeat_mask: DayMask,
    /// Whether the alarm is enabled.
    enabled: bool,
    /// Whether snooze is enabled.
    snooze: bool,
}

impl AlarmEntry {
    /// Maximum valid hour value.
    pub const MAX_HOUR: u8 = 23;

    /// Maximum valid minute value.
    pub const MAX_MINUTE: u8 = 59;

    /// Create a new alarm entry, validating hour and minute ranges.
    pub fn new(hour: u8, minute: u8, repeat_mask: DayMask, enabled: bool, snooze: bool) -> Result<Self> {
        if hour > Self::MAX_HOUR {
            return Err(ClockError::Parse(format!("invalid alarm hour: {hour}")));
        }
        if minute > Self::MAX_MINUTE {
            return Err(ClockError::Parse(format!("invalid alarm minute: {minute}")));
        }
        Ok(Self {
            hour,
            minute,
            repeat_mask,
            enabled,
            snooze,
        })
    }

    /// Get the hour (0–23).
    pub const fn hour(&self) -> u8 {
        self.hour
    }

    /// Get the minute (0–59).
    pub const fn minute(&self) -> u8 {
        self.minute
    }

    /// Get the day-of-week repeat bitmask.
    pub const fn repeat_mask(&self) -> DayMask {
        self.repeat_mask
    }

    /// Whether the alarm is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether snooze is enabled.
    pub const fn snooze(&self) -> bool {
        self.snooze
    }

    /// Encode the alarm into the 5-byte structure used in read/write frames.
    ///
    /// Format: `[Enabled] [HH] [MM] [Days] [Snooze]`
    pub fn encode(&self) -> [u8; 5] {
        [
            if self.enabled { 0x01 } else { 0x00 },
            self.hour,
            self.minute,
            self.repeat_mask.value(),
            if self.snooze { 0x01 } else { 0x00 },
        ]
    }

    /// Decode an alarm from a raw 5-byte payload.
    ///
    /// An empty/unused slot has all bytes set to `0xFF` and returns `None`.
    /// Validates hour and minute ranges from the decoded payload.
    pub fn decode(payload: &[u8]) -> Result<Option<Self>> {
        if payload.len() < 5 {
            return Err(ClockError::Parse("alarm payload too short".into()));
        }
        // Empty slots are all 0xFF.
        if payload.iter().all(|&b| b == 0xFF) {
            return Ok(None);
        }
        Self::new(payload[1], payload[2], DayMask::new(payload[3]), payload[0] != 0x00, payload[4] != 0x00).map(Some)
    }

    /// Encode the set-alarm payload (6 bytes including slot index).
    ///
    /// Format: `[ID] [Enabled] [HH] [MM] [Days] [Snooze]`
    pub fn encode_set_payload(&self, slot: AlarmSlotIndex) -> [u8; 6] {
        [
            slot.value(),
            if self.enabled { 0x01 } else { 0x00 },
            self.hour,
            self.minute,
            self.repeat_mask.value(),
            if self.snooze { 0x01 } else { 0x00 },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_alarm_entry() {
        let entry = AlarmEntry::new(7, 30, DayMask::WEEKDAYS, true, true).unwrap();
        assert_eq!(entry.encode(), [0x01, 0x07, 0x1E, 0x3E, 0x01]);
    }

    #[test]
    fn decode_alarm_entry() {
        let payload = [0x01, 0x07, 0x1E, 0x3E, 0x01];
        let entry = AlarmEntry::decode(&payload).unwrap().unwrap();
        assert_eq!(entry.hour(), 7);
        assert_eq!(entry.minute(), 30);
        assert_eq!(entry.repeat_mask(), DayMask::WEEKDAYS);
        assert!(entry.enabled());
        assert!(entry.snooze());
    }

    #[test]
    fn decode_empty_slot() {
        let payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let entry = AlarmEntry::decode(&payload).unwrap();
        assert!(entry.is_none());
    }

    #[test]
    fn decode_too_short() {
        let payload = [0x01, 0x07, 0x1E];
        assert!(AlarmEntry::decode(&payload).is_err());
    }

    #[test]
    fn encode_set_payload() {
        let entry = AlarmEntry::new(6, 0, DayMask::EVERY_DAY, true, false).unwrap();
        assert_eq!(entry.encode_set_payload(AlarmSlotIndex::new(3).unwrap()), [0x03, 0x01, 0x06, 0x00, 0x7F, 0x00]);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let entry = AlarmEntry::new(12, 45, DayMask::WEEKENDS, true, true).unwrap();
        let encoded = entry.encode();
        let decoded = AlarmEntry::decode(&encoded).unwrap().unwrap();
        assert_eq!(entry, decoded);
    }

    #[test]
    fn new_rejects_invalid_hour() {
        assert!(AlarmEntry::new(24, 0, DayMask::EVERY_DAY, true, false).is_err());
        assert!(AlarmEntry::new(255, 0, DayMask::EVERY_DAY, true, false).is_err());
    }

    #[test]
    fn new_rejects_invalid_minute() {
        assert!(AlarmEntry::new(0, 60, DayMask::EVERY_DAY, true, false).is_err());
        assert!(AlarmEntry::new(0, 255, DayMask::EVERY_DAY, true, false).is_err());
    }

    #[test]
    fn new_accepts_boundary_values() {
        assert!(AlarmEntry::new(0, 0, DayMask::ONCE, true, false).is_ok());
        assert!(AlarmEntry::new(23, 59, DayMask::EVERY_DAY, true, false).is_ok());
    }
}
