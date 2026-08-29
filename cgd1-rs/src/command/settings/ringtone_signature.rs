/// 4-byte ringtone signature used in the settings payload.
///
/// Identifies the active ringtone. Set to `0xFF` bytes when unused.
/// Custom ringtones use alternating signatures (`de ad de ad` / `be ef be ef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingtoneSignature([u8; 4]);

impl RingtoneSignature {
    /// Unused/empty signature (all `0xFF`).
    pub const UNUSED: RingtoneSignature = RingtoneSignature([0xFF, 0xFF, 0xFF, 0xFF]);

    /// Custom ringtone slot A signature.
    pub const CUSTOM_SLOT_A: RingtoneSignature = RingtoneSignature([0xDE, 0xAD, 0xDE, 0xAD]);

    /// Custom ringtone slot B signature.
    pub const CUSTOM_SLOT_B: RingtoneSignature = RingtoneSignature([0xBE, 0xEF, 0xBE, 0xEF]);

    /// Create a signature from raw bytes.
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Get the raw signature bytes.
    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }

    /// Whether this signature is the unused sentinel.
    pub fn is_unused(self) -> bool {
        self.0 == [0xFF, 0xFF, 0xFF, 0xFF]
    }
}

impl From<[u8; 4]> for RingtoneSignature {
    fn from(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }
}

impl From<RingtoneSignature> for [u8; 4] {
    fn from(sig: RingtoneSignature) -> Self {
        sig.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unused_signature() {
        assert!(RingtoneSignature::UNUSED.is_unused());
        assert!(!RingtoneSignature::CUSTOM_SLOT_A.is_unused());
    }

    #[test]
    fn bytes_roundtrip() {
        let sig = RingtoneSignature::new([0xFD, 0xC3, 0x66, 0xA5]);
        assert_eq!(sig.bytes(), [0xFD, 0xC3, 0x66, 0xA5]);
    }

    #[test]
    fn from_array() {
        let sig: RingtoneSignature = [0x09, 0x61, 0xBB, 0x77].into();
        assert_eq!(sig.bytes(), [0x09, 0x61, 0xBB, 0x77]);
    }

    #[test]
    fn into_array() {
        let sig = RingtoneSignature::new([0xBA, 0x2C, 0x2C, 0x8C]);
        let arr: [u8; 4] = sig.into();
        assert_eq!(arr, [0xBA, 0x2C, 0x2C, 0x8C]);
    }
}
