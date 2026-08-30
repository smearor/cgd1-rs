use crate::error::ClockError;
use crate::error::Result;
use crate::types::ClockTime;

use super::day_mask::DayMask;
use super::slot_index::AlarmSlotIndex;

/// A single alarm entry.
///
/// Maps to the 5-byte alarm structure used in the CGD1 BLE protocol:
/// `[Enabled] [HH] [MM] [Days] [Snooze]`.
///
/// Invariants are enforced by construction via the [`ClockTime`] field:
/// - `time` hour is in range 0–23, minute 0–59
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmEntry {
    /// Alarm time (hour 0–23, minute 0–59).
    time: ClockTime,
    /// Day-of-week repeat bitmask.
    repeat_mask: DayMask,
    /// Whether the alarm is enabled.
    enabled: bool,
    /// Whether snooze is enabled.
    snooze: bool,
}

impl AlarmEntry {
    /// Create a new alarm entry with a validated clock time.
    pub fn new(time: ClockTime, repeat_mask: DayMask, enabled: bool, snooze: bool) -> Self {
        Self {
            time,
            repeat_mask,
            enabled,
            snooze,
        }
    }

    /// Get the alarm time.
    pub const fn time(&self) -> ClockTime {
        self.time
    }

    /// Get the hour (0–23).
    pub const fn hour(&self) -> u8 {
        self.time.hour()
    }

    /// Get the minute (0–59).
    pub const fn minute(&self) -> u8 {
        self.time.minute()
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
            self.time.hour(),
            self.time.minute(),
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
        let time = ClockTime::new(payload[1], payload[2])?;
        Ok(Some(Self::new(time, DayMask::new(payload[3]), payload[0] != 0x00, payload[4] != 0x00)))
    }

    /// Encode the set-alarm payload (6 bytes including slot index).
    ///
    /// Format: `[ID] [Enabled] [HH] [MM] [Days] [Snooze]`
    pub fn encode_set_payload(&self, slot: AlarmSlotIndex) -> [u8; 6] {
        [
            slot.value(),
            if self.enabled { 0x01 } else { 0x00 },
            self.time.hour(),
            self.time.minute(),
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
        let entry = AlarmEntry::new(ClockTime::new(7, 30).unwrap(), DayMask::WEEKDAYS, true, true);
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
        let entry = AlarmEntry::new(ClockTime::new(6, 0).unwrap(), DayMask::EVERY_DAY, true, false);
        assert_eq!(entry.encode_set_payload(AlarmSlotIndex::new(3).unwrap()), [0x03, 0x01, 0x06, 0x00, 0x7F, 0x00]);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let entry = AlarmEntry::new(ClockTime::new(12, 45).unwrap(), DayMask::WEEKENDS, true, true);
        let encoded = entry.encode();
        let decoded = AlarmEntry::decode(&encoded).unwrap().unwrap();
        assert_eq!(entry, decoded);
    }

    #[test]
    fn new_rejects_invalid_hour() {
        assert!(ClockTime::new(24, 0).is_err());
        assert!(ClockTime::new(255, 0).is_err());
    }

    #[test]
    fn new_rejects_invalid_minute() {
        assert!(ClockTime::new(0, 60).is_err());
        assert!(ClockTime::new(0, 255).is_err());
    }

    #[test]
    fn new_accepts_boundary_values() {
        assert!(ClockTime::new(0, 0).is_ok());
        assert!(ClockTime::new(23, 59).is_ok());
    }
}
