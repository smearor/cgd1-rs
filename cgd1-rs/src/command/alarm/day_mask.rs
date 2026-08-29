/// Day-of-week bitmask for alarm repeat.
///
/// Bit 0 = Monday, bit 1 = Tuesday, ..., bit 5 = Saturday, bit 6 = Sunday.
/// `0x00` means one-shot (fires once, then auto-disables).
/// `0x7F` means every day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayMask(u8);

impl DayMask {
    /// Every day (Mon–Sun).
    pub const EVERY_DAY: Self = Self(0x7F);

    /// Weekdays only (Mon–Fri).
    pub const WEEKDAYS: Self = Self(0x3E);

    /// Weekends only (Sat, Sun).
    pub const WEEKENDS: Self = Self(0x41);

    /// One-shot (no repeat, fires once then auto-disables).
    pub const ONCE: Self = Self(0x00);

    /// Create a day mask from a raw bitmask.
    pub const fn new(mask: u8) -> Self {
        Self(mask)
    }

    /// Get the raw bitmask value.
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for DayMask {
    fn from(mask: u8) -> Self {
        Self(mask)
    }
}

impl From<DayMask> for u8 {
    fn from(mask: DayMask) -> u8 {
        mask.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_mask_constants() {
        assert_eq!(DayMask::EVERY_DAY.value(), 0x7F);
        assert_eq!(DayMask::WEEKDAYS.value(), 0x3E);
        assert_eq!(DayMask::WEEKENDS.value(), 0x41);
        assert_eq!(DayMask::ONCE.value(), 0x00);
    }

    #[test]
    fn day_mask_from_u8() {
        let mask: DayMask = 0x1F.into();
        assert_eq!(mask.value(), 0x1F);
        let raw: u8 = mask.into();
        assert_eq!(raw, 0x1F);
    }
}
