use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Error parsing a [`RingtoneSignature`] from a string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid ringtone signature '{input}': {reason}")]
pub struct RingtoneSignatureParseError {
    /// The raw input string.
    pub input: String,
    /// The parse error reason.
    pub reason: String,
}

/// 4-byte ringtone signature that identifies a ringtone on the CGD1 device.
///
/// The signature serves two purposes in the BLE protocol:
///
/// - **Audio upload** (`08 10 [Size 3B] [Sig 4B]`): tells the device which
///   slot to store the uploaded audio under.
/// - **Settings payload** (bytes 14–17): selects the active ringtone by
///   writing the matching signature.
///
/// # Built-in ringtones
///
/// The official Qingping firmware ships with these ringtones, each identified
/// by a fixed 4-byte signature:
///
/// | Signature     | Variant                |
/// |---------------|------------------------|
/// | `fd c3 66 a5` | [`Beep`]               |
/// | `09 61 bb 77` | [`Digital`]            |
/// | `ba 2c 2c 8c` | [`Digital2`]           |
/// | `ea 2d 4c 02` | [`Cuckoo`]             |
/// | `79 1b ac b3` | [`Telephone`]          |
/// | `1d 01 9f d6` | [`ExoticGuitar`]       |
/// | `6e 70 b6 59` | [`LivelyPiano`]        |
/// | `8f 00 48 86` | [`StoryPiano`]         |
/// | `26 52 25 19` | [`ForestPiano`]        |
///
/// # Custom ringtones
///
/// Two alternating slot signatures are available for user-uploaded audio:
///
/// - [`CustomSlotA`] — `de ad de ad`
/// - [`CustomSlotB`] — `be ef be ef`
///
/// Always alternate between slots when uploading new custom audio. The device
/// may reject uploads if the target signature matches the currently active
/// ringtone.
///
/// # Unused
///
/// [`Unused`] (`ff ff ff ff`) indicates that no custom ringtone is selected.
///
/// For signatures not covered by the built-in variants, use
/// [`Custom`] with the raw 4-byte value.
///
/// [`Beep`]: RingtoneSignature::Beep
/// [`Digital`]: RingtoneSignature::Digital
/// [`Digital2`]: RingtoneSignature::Digital2
/// [`Cuckoo`]: RingtoneSignature::Cuckoo
/// [`Telephone`]: RingtoneSignature::Telephone
/// [`ExoticGuitar`]: RingtoneSignature::ExoticGuitar
/// [`LivelyPiano`]: RingtoneSignature::LivelyPiano
/// [`StoryPiano`]: RingtoneSignature::StoryPiano
/// [`ForestPiano`]: RingtoneSignature::ForestPiano
/// [`CustomSlotA`]: RingtoneSignature::CustomSlotA
/// [`CustomSlotB`]: RingtoneSignature::CustomSlotB
/// [`Unused`]: RingtoneSignature::Unused
/// [`Custom`]: RingtoneSignature::Custom
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingtoneSignature {
    /// Beep (`fd c3 66 a5`).
    Beep,
    /// Digital Ringtone (`09 61 bb 77`).
    Digital,
    /// Digital Ringtone 2 (`ba 2c 2c 8c`).
    Digital2,
    /// Cuckoo (`ea 2d 4c 02`).
    Cuckoo,
    /// Telephone Ringtone (`79 1b ac b3`).
    Telephone,
    /// Exotic Guitar (`1d 01 9f d6`).
    ExoticGuitar,
    /// Lively Piano (`6e 70 b6 59`).
    LivelyPiano,
    /// Story Piano (`8f 00 48 86`).
    StoryPiano,
    /// Forest Piano (`26 52 25 19`).
    ForestPiano,
    /// Custom ringtone slot A (`de ad de ad`).
    CustomSlotA,
    /// Custom ringtone slot B (`be ef be ef`).
    CustomSlotB,
    /// No custom ringtone selected (`ff ff ff ff`).
    Unused,
    /// Any other 4-byte signature not covered by the built-in variants.
    Custom([u8; 4]),
}

impl RingtoneSignature {
    /// Create a signature from raw bytes, mapping known values to their
    /// named variants and falling back to [`Custom`](Self::Custom) for
    /// unknown signatures.
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        match bytes {
            [0xFD, 0xC3, 0x66, 0xA5] => Self::Beep,
            [0x09, 0x61, 0xBB, 0x77] => Self::Digital,
            [0xBA, 0x2C, 0x2C, 0x8C] => Self::Digital2,
            [0xEA, 0x2D, 0x4C, 0x02] => Self::Cuckoo,
            [0x79, 0x1B, 0xAC, 0xB3] => Self::Telephone,
            [0x1D, 0x01, 0x9F, 0xD6] => Self::ExoticGuitar,
            [0x6E, 0x70, 0xB6, 0x59] => Self::LivelyPiano,
            [0x8F, 0x00, 0x48, 0x86] => Self::StoryPiano,
            [0x26, 0x52, 0x25, 0x19] => Self::ForestPiano,
            [0xDE, 0xAD, 0xDE, 0xAD] => Self::CustomSlotA,
            [0xBE, 0xEF, 0xBE, 0xEF] => Self::CustomSlotB,
            [0xFF, 0xFF, 0xFF, 0xFF] => Self::Unused,
            other => Self::Custom(other),
        }
    }

    /// Get the raw 4-byte signature.
    pub const fn bytes(self) -> [u8; 4] {
        match self {
            Self::Beep => [0xFD, 0xC3, 0x66, 0xA5],
            Self::Digital => [0x09, 0x61, 0xBB, 0x77],
            Self::Digital2 => [0xBA, 0x2C, 0x2C, 0x8C],
            Self::Cuckoo => [0xEA, 0x2D, 0x4C, 0x02],
            Self::Telephone => [0x79, 0x1B, 0xAC, 0xB3],
            Self::ExoticGuitar => [0x1D, 0x01, 0x9F, 0xD6],
            Self::LivelyPiano => [0x6E, 0x70, 0xB6, 0x59],
            Self::StoryPiano => [0x8F, 0x00, 0x48, 0x86],
            Self::ForestPiano => [0x26, 0x52, 0x25, 0x19],
            Self::CustomSlotA => [0xDE, 0xAD, 0xDE, 0xAD],
            Self::CustomSlotB => [0xBE, 0xEF, 0xBE, 0xEF],
            Self::Unused => [0xFF, 0xFF, 0xFF, 0xFF],
            Self::Custom(bytes) => bytes,
        }
    }

    /// Whether this signature is the unused sentinel.
    pub fn is_unused(self) -> bool {
        matches!(self, Self::Unused)
    }

    /// Human-readable name of the ringtone.
    pub fn name(self) -> &'static str {
        match self {
            Self::Beep => "Beep",
            Self::Digital => "Digital",
            Self::Digital2 => "Digital2",
            Self::Cuckoo => "Cuckoo",
            Self::Telephone => "Telephone",
            Self::ExoticGuitar => "ExoticGuitar",
            Self::LivelyPiano => "LivelyPiano",
            Self::StoryPiano => "StoryPiano",
            Self::ForestPiano => "ForestPiano",
            Self::CustomSlotA => "CustomSlotA",
            Self::CustomSlotB => "CustomSlotB",
            Self::Unused => "Unused",
            Self::Custom(_) => "Custom",
        }
    }
}

impl From<[u8; 4]> for RingtoneSignature {
    fn from(bytes: [u8; 4]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<RingtoneSignature> for [u8; 4] {
    fn from(sig: RingtoneSignature) -> Self {
        sig.bytes()
    }
}

impl Serialize for RingtoneSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RingtoneSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Display for RingtoneSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let bytes = self.bytes();
        write!(f, "{} ({:02x}{:02x}{:02x}{:02x})", self.name(), bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

impl FromStr for RingtoneSignature {
    type Err = RingtoneSignatureParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Beep" => return Ok(Self::Beep),
            "Digital" => return Ok(Self::Digital),
            "Digital2" => return Ok(Self::Digital2),
            "Cuckoo" => return Ok(Self::Cuckoo),
            "Telephone" => return Ok(Self::Telephone),
            "ExoticGuitar" => return Ok(Self::ExoticGuitar),
            "LivelyPiano" => return Ok(Self::LivelyPiano),
            "StoryPiano" => return Ok(Self::StoryPiano),
            "ForestPiano" => return Ok(Self::ForestPiano),
            "CustomSlotA" => return Ok(Self::CustomSlotA),
            "CustomSlotB" => return Ok(Self::CustomSlotB),
            "Unused" => return Ok(Self::Unused),
            _ => {}
        }
        let bytes = hex::decode(s.as_bytes()).map_err(|e| RingtoneSignatureParseError {
            input: s.to_string(),
            reason: e.to_string(),
        })?;
        if bytes.len() != 4 {
            return Err(RingtoneSignatureParseError {
                input: s.to_string(),
                reason: format!("signature must be 4 bytes, got {}", bytes.len()),
            });
        }
        let arr: [u8; 4] = bytes.as_slice().try_into().map_err(|_| RingtoneSignatureParseError {
            input: s.to_string(),
            reason: "internal error: length mismatch".to_string(),
        })?;
        Ok(Self::from_bytes(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_known_signatures() {
        assert_eq!(RingtoneSignature::from_bytes([0xFD, 0xC3, 0x66, 0xA5]), RingtoneSignature::Beep);
        assert_eq!(RingtoneSignature::from_bytes([0x09, 0x61, 0xBB, 0x77]), RingtoneSignature::Digital);
        assert_eq!(RingtoneSignature::from_bytes([0xBA, 0x2C, 0x2C, 0x8C]), RingtoneSignature::Digital2);
        assert_eq!(RingtoneSignature::from_bytes([0xEA, 0x2D, 0x4C, 0x02]), RingtoneSignature::Cuckoo);
        assert_eq!(RingtoneSignature::from_bytes([0x79, 0x1B, 0xAC, 0xB3]), RingtoneSignature::Telephone);
        assert_eq!(RingtoneSignature::from_bytes([0x1D, 0x01, 0x9F, 0xD6]), RingtoneSignature::ExoticGuitar);
        assert_eq!(RingtoneSignature::from_bytes([0x6E, 0x70, 0xB6, 0x59]), RingtoneSignature::LivelyPiano);
        assert_eq!(RingtoneSignature::from_bytes([0x8F, 0x00, 0x48, 0x86]), RingtoneSignature::StoryPiano);
        assert_eq!(RingtoneSignature::from_bytes([0x26, 0x52, 0x25, 0x19]), RingtoneSignature::ForestPiano);
        assert_eq!(RingtoneSignature::from_bytes([0xDE, 0xAD, 0xDE, 0xAD]), RingtoneSignature::CustomSlotA);
        assert_eq!(RingtoneSignature::from_bytes([0xBE, 0xEF, 0xBE, 0xEF]), RingtoneSignature::CustomSlotB);
        assert_eq!(RingtoneSignature::from_bytes([0xFF, 0xFF, 0xFF, 0xFF]), RingtoneSignature::Unused);
    }

    #[test]
    fn from_bytes_unknown_signature() {
        let sig = RingtoneSignature::from_bytes([0x01, 0x02, 0x03, 0x04]);
        assert_eq!(sig, RingtoneSignature::Custom([0x01, 0x02, 0x03, 0x04]));
    }

    #[test]
    fn bytes_roundtrip() {
        assert_eq!(RingtoneSignature::Beep.bytes(), [0xFD, 0xC3, 0x66, 0xA5]);
        assert_eq!(RingtoneSignature::Digital.bytes(), [0x09, 0x61, 0xBB, 0x77]);
        assert_eq!(RingtoneSignature::CustomSlotA.bytes(), [0xDE, 0xAD, 0xDE, 0xAD]);
        assert_eq!(RingtoneSignature::Unused.bytes(), [0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(RingtoneSignature::Custom([0x01, 0x02, 0x03, 0x04]).bytes(), [0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn from_array() {
        let sig: RingtoneSignature = [0x09, 0x61, 0xBB, 0x77].into();
        assert_eq!(sig, RingtoneSignature::Digital);
    }

    #[test]
    fn into_array() {
        let arr: [u8; 4] = RingtoneSignature::Digital2.into();
        assert_eq!(arr, [0xBA, 0x2C, 0x2C, 0x8C]);
    }

    #[test]
    fn is_unused() {
        assert!(RingtoneSignature::Unused.is_unused());
        assert!(!RingtoneSignature::Beep.is_unused());
        assert!(!RingtoneSignature::CustomSlotA.is_unused());
    }

    #[test]
    fn from_str_known_signatures() {
        assert_eq!(RingtoneSignature::from_str("fdc366a5").unwrap(), RingtoneSignature::Beep);
        assert_eq!(RingtoneSignature::from_str("0961bb77").unwrap(), RingtoneSignature::Digital);
        assert_eq!(RingtoneSignature::from_str("deaddead").unwrap(), RingtoneSignature::CustomSlotA);
        assert_eq!(RingtoneSignature::from_str("beefbeef").unwrap(), RingtoneSignature::CustomSlotB);
        assert_eq!(RingtoneSignature::from_str("ffffffff").unwrap(), RingtoneSignature::Unused);
    }

    #[test]
    fn from_str_by_name() {
        assert_eq!(RingtoneSignature::from_str("Beep").unwrap(), RingtoneSignature::Beep);
        assert_eq!(RingtoneSignature::from_str("Digital").unwrap(), RingtoneSignature::Digital);
        assert_eq!(RingtoneSignature::from_str("Digital2").unwrap(), RingtoneSignature::Digital2);
        assert_eq!(RingtoneSignature::from_str("Cuckoo").unwrap(), RingtoneSignature::Cuckoo);
        assert_eq!(RingtoneSignature::from_str("Telephone").unwrap(), RingtoneSignature::Telephone);
        assert_eq!(RingtoneSignature::from_str("ExoticGuitar").unwrap(), RingtoneSignature::ExoticGuitar);
        assert_eq!(RingtoneSignature::from_str("LivelyPiano").unwrap(), RingtoneSignature::LivelyPiano);
        assert_eq!(RingtoneSignature::from_str("StoryPiano").unwrap(), RingtoneSignature::StoryPiano);
        assert_eq!(RingtoneSignature::from_str("ForestPiano").unwrap(), RingtoneSignature::ForestPiano);
        assert_eq!(RingtoneSignature::from_str("CustomSlotA").unwrap(), RingtoneSignature::CustomSlotA);
        assert_eq!(RingtoneSignature::from_str("CustomSlotB").unwrap(), RingtoneSignature::CustomSlotB);
        assert_eq!(RingtoneSignature::from_str("Unused").unwrap(), RingtoneSignature::Unused);
    }

    #[test]
    fn from_str_custom_signature() {
        let sig = RingtoneSignature::from_str("01234567").unwrap();
        assert_eq!(sig, RingtoneSignature::Custom([0x01, 0x23, 0x45, 0x67]));
    }

    #[test]
    fn from_str_invalid() {
        assert!(RingtoneSignature::from_str("xyz").is_err());
        assert!(RingtoneSignature::from_str("deadbee").is_err());
        assert!(RingtoneSignature::from_str("deadbeef00").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", RingtoneSignature::Beep), "Beep (fdc366a5)");
        assert_eq!(format!("{}", RingtoneSignature::CustomSlotA), "CustomSlotA (deaddead)");
        assert_eq!(format!("{}", RingtoneSignature::Unused), "Unused (ffffffff)");
        assert_eq!(format!("{}", RingtoneSignature::Custom([0xDE, 0xAD, 0xBE, 0xEF])), "Custom (deadbeef)");
    }
}
