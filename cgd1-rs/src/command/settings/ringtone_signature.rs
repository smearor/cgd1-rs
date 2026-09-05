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
/// | `4d 6f 6e 6b` | [`MonkeyIsland`]       |
/// | `41 6c 53 79` | [`AlarmSynth`]         |
/// | `41 72 4d 62` | [`ArrayMbira`]         |
/// | `42 6c 69 73` | [`Bliss`]              |
/// | `43 65 6c 73` | [`Celestial`]          |
/// | `45 6e 74 72` | [`Entropy`]            |
/// | `47 6c 4d 61` | [`GlassMarimba`]       |
/// | `48 61 6c 6f` | [`HaloPentatonic`]     |
/// | `48 61 72 6d` | [`Harmonics`]          |
/// | `48 61 72 70` | [`HarpArp`]            |
/// | `4b 6f 74 6f` | [`KotoChords`]         |
/// | `53 61 6b 65` | [`Sakenointi`]         |
/// | `53 61 6d 73` | [`SamsSong`]           |
/// | `53 6f 75 6c` | [`Soul`]               |
/// | `53 70 61 72` | [`Sparkle`]            |
/// | `53 75 70 72` | [`Supreme`]            |
/// | `53 75 72 75` | [`SuruArpeggio`]       |
/// | `54 69 6d 65` | [`TimeNotLost`]        |
/// | `57 6f 6f 64` | [`WoodenDrive`]        |
///
/// # Custom ringtones
///
/// Two alternating slot signatures are available for user-uploaded audio:
///
/// - [`CustomSlotA`] - `de ad de ad`
/// - [`CustomSlotB`] - `be ef be ef`
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
/// [`MonkeyIsland`]: RingtoneSignature::MonkeyIsland
/// [`AlarmSynth`]: RingtoneSignature::AlarmSynth
/// [`ArrayMbira`]: RingtoneSignature::ArrayMbira
/// [`Bliss`]: RingtoneSignature::Bliss
/// [`Celestial`]: RingtoneSignature::Celestial
/// [`Entropy`]: RingtoneSignature::Entropy
/// [`GlassMarimba`]: RingtoneSignature::GlassMarimba
/// [`HaloPentatonic`]: RingtoneSignature::HaloPentatonic
/// [`Harmonics`]: RingtoneSignature::Harmonics
/// [`HarpArp`]: RingtoneSignature::HarpArp
/// [`KotoChords`]: RingtoneSignature::KotoChords
/// [`Sakenointi`]: RingtoneSignature::Sakenointi
/// [`SamsSong`]: RingtoneSignature::SamsSong
/// [`Soul`]: RingtoneSignature::Soul
/// [`Sparkle`]: RingtoneSignature::Sparkle
/// [`Supreme`]: RingtoneSignature::Supreme
/// [`SuruArpeggio`]: RingtoneSignature::SuruArpeggio
/// [`TimeNotLost`]: RingtoneSignature::TimeNotLost
/// [`WoodenDrive`]: RingtoneSignature::WoodenDrive
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
    /// Monkey Island 8-bit (`4d 6f 6e 6b`).
    MonkeyIsland,
    /// Alarm Synth (`41 6c 53 79`).
    AlarmSynth,
    /// Array Mbira (`41 72 4d 62`).
    ArrayMbira,
    /// Bliss (`42 6c 69 73`).
    Bliss,
    /// Celestial (`43 65 6c 73`).
    Celestial,
    /// Entropy (`45 6e 74 72`).
    Entropy,
    /// Glass Marimba (`47 6c 4d 61`).
    GlassMarimba,
    /// Halo Pentatonic (`48 61 6c 6f`).
    HaloPentatonic,
    /// Harmonics (`48 61 72 6d`).
    Harmonics,
    /// Harp Arp (`48 61 72 70`).
    HarpArp,
    /// Koto Chords (`4b 6f 74 6f`).
    KotoChords,
    /// Sakenointi (`53 61 6b 65`).
    Sakenointi,
    /// Sam's Song (`53 61 6d 73`).
    SamsSong,
    /// Soul (`53 6f 75 6c`).
    Soul,
    /// Sparkle (`53 70 61 72`).
    Sparkle,
    /// Supreme (`53 75 70 72`).
    Supreme,
    /// Suru Arpeggio (`53 75 72 75`).
    SuruArpeggio,
    /// Time Not Lost (`54 69 6d 65`).
    TimeNotLost,
    /// Wooden Drive (`57 6f 6f 64`).
    WoodenDrive,
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
            [0x4D, 0x6F, 0x6E, 0x6B] => Self::MonkeyIsland,
            [0x41, 0x6C, 0x53, 0x79] => Self::AlarmSynth,
            [0x41, 0x72, 0x4D, 0x62] => Self::ArrayMbira,
            [0x42, 0x6C, 0x69, 0x73] => Self::Bliss,
            [0x43, 0x65, 0x6C, 0x73] => Self::Celestial,
            [0x45, 0x6E, 0x74, 0x72] => Self::Entropy,
            [0x47, 0x6C, 0x4D, 0x61] => Self::GlassMarimba,
            [0x48, 0x61, 0x6C, 0x6F] => Self::HaloPentatonic,
            [0x48, 0x61, 0x72, 0x6D] => Self::Harmonics,
            [0x48, 0x61, 0x72, 0x70] => Self::HarpArp,
            [0x4B, 0x6F, 0x74, 0x6F] => Self::KotoChords,
            [0x53, 0x61, 0x6B, 0x65] => Self::Sakenointi,
            [0x53, 0x61, 0x6D, 0x73] => Self::SamsSong,
            [0x53, 0x6F, 0x75, 0x6C] => Self::Soul,
            [0x53, 0x70, 0x61, 0x72] => Self::Sparkle,
            [0x53, 0x75, 0x70, 0x72] => Self::Supreme,
            [0x53, 0x75, 0x72, 0x75] => Self::SuruArpeggio,
            [0x54, 0x69, 0x6D, 0x65] => Self::TimeNotLost,
            [0x57, 0x6F, 0x6F, 0x64] => Self::WoodenDrive,
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
            Self::MonkeyIsland => [0x4D, 0x6F, 0x6E, 0x6B],
            Self::AlarmSynth => [0x41, 0x6C, 0x53, 0x79],
            Self::ArrayMbira => [0x41, 0x72, 0x4D, 0x62],
            Self::Bliss => [0x42, 0x6C, 0x69, 0x73],
            Self::Celestial => [0x43, 0x65, 0x6C, 0x73],
            Self::Entropy => [0x45, 0x6E, 0x74, 0x72],
            Self::GlassMarimba => [0x47, 0x6C, 0x4D, 0x61],
            Self::HaloPentatonic => [0x48, 0x61, 0x6C, 0x6F],
            Self::Harmonics => [0x48, 0x61, 0x72, 0x6D],
            Self::HarpArp => [0x48, 0x61, 0x72, 0x70],
            Self::KotoChords => [0x4B, 0x6F, 0x74, 0x6F],
            Self::Sakenointi => [0x53, 0x61, 0x6B, 0x65],
            Self::SamsSong => [0x53, 0x61, 0x6D, 0x73],
            Self::Soul => [0x53, 0x6F, 0x75, 0x6C],
            Self::Sparkle => [0x53, 0x70, 0x61, 0x72],
            Self::Supreme => [0x53, 0x75, 0x70, 0x72],
            Self::SuruArpeggio => [0x53, 0x75, 0x72, 0x75],
            Self::TimeNotLost => [0x54, 0x69, 0x6D, 0x65],
            Self::WoodenDrive => [0x57, 0x6F, 0x6F, 0x64],
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
            Self::MonkeyIsland => "Monkey Island (8-bit)",
            Self::AlarmSynth => "Alarm Synth",
            Self::ArrayMbira => "Array Mbira",
            Self::Bliss => "Bliss",
            Self::Celestial => "Celestial",
            Self::Entropy => "Entropy",
            Self::GlassMarimba => "Glass Marimba",
            Self::HaloPentatonic => "Halo Pentatonic",
            Self::Harmonics => "Harmonics",
            Self::HarpArp => "Harp Arp",
            Self::KotoChords => "Koto Chords",
            Self::Sakenointi => "Sakenointi",
            Self::SamsSong => "Sam's Song",
            Self::Soul => "Soul",
            Self::Sparkle => "Sparkle",
            Self::Supreme => "Supreme",
            Self::SuruArpeggio => "Suru Arpeggio",
            Self::TimeNotLost => "Time Not Lost",
            Self::WoodenDrive => "Wooden Drive",
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
            "MonkeyIsland" => return Ok(Self::MonkeyIsland),
            "AlarmSynth" => return Ok(Self::AlarmSynth),
            "ArrayMbira" => return Ok(Self::ArrayMbira),
            "Bliss" => return Ok(Self::Bliss),
            "Celestial" => return Ok(Self::Celestial),
            "Entropy" => return Ok(Self::Entropy),
            "GlassMarimba" => return Ok(Self::GlassMarimba),
            "HaloPentatonic" => return Ok(Self::HaloPentatonic),
            "Harmonics" => return Ok(Self::Harmonics),
            "HarpArp" => return Ok(Self::HarpArp),
            "KotoChords" => return Ok(Self::KotoChords),
            "Sakenointi" => return Ok(Self::Sakenointi),
            "SamsSong" => return Ok(Self::SamsSong),
            "Soul" => return Ok(Self::Soul),
            "Sparkle" => return Ok(Self::Sparkle),
            "Supreme" => return Ok(Self::Supreme),
            "SuruArpeggio" => return Ok(Self::SuruArpeggio),
            "TimeNotLost" => return Ok(Self::TimeNotLost),
            "WoodenDrive" => return Ok(Self::WoodenDrive),
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
        assert_eq!(RingtoneSignature::from_bytes([0x4D, 0x6F, 0x6E, 0x6B]), RingtoneSignature::MonkeyIsland);
        assert_eq!(RingtoneSignature::from_bytes([0x41, 0x6C, 0x53, 0x79]), RingtoneSignature::AlarmSynth);
        assert_eq!(RingtoneSignature::from_bytes([0x41, 0x72, 0x4D, 0x62]), RingtoneSignature::ArrayMbira);
        assert_eq!(RingtoneSignature::from_bytes([0x42, 0x6C, 0x69, 0x73]), RingtoneSignature::Bliss);
        assert_eq!(RingtoneSignature::from_bytes([0x43, 0x65, 0x6C, 0x73]), RingtoneSignature::Celestial);
        assert_eq!(RingtoneSignature::from_bytes([0x45, 0x6E, 0x74, 0x72]), RingtoneSignature::Entropy);
        assert_eq!(RingtoneSignature::from_bytes([0x47, 0x6C, 0x4D, 0x61]), RingtoneSignature::GlassMarimba);
        assert_eq!(RingtoneSignature::from_bytes([0x48, 0x61, 0x6C, 0x6F]), RingtoneSignature::HaloPentatonic);
        assert_eq!(RingtoneSignature::from_bytes([0x48, 0x61, 0x72, 0x6D]), RingtoneSignature::Harmonics);
        assert_eq!(RingtoneSignature::from_bytes([0x48, 0x61, 0x72, 0x70]), RingtoneSignature::HarpArp);
        assert_eq!(RingtoneSignature::from_bytes([0x4B, 0x6F, 0x74, 0x6F]), RingtoneSignature::KotoChords);
        assert_eq!(RingtoneSignature::from_bytes([0x53, 0x61, 0x6B, 0x65]), RingtoneSignature::Sakenointi);
        assert_eq!(RingtoneSignature::from_bytes([0x53, 0x61, 0x6D, 0x73]), RingtoneSignature::SamsSong);
        assert_eq!(RingtoneSignature::from_bytes([0x53, 0x6F, 0x75, 0x6C]), RingtoneSignature::Soul);
        assert_eq!(RingtoneSignature::from_bytes([0x53, 0x70, 0x61, 0x72]), RingtoneSignature::Sparkle);
        assert_eq!(RingtoneSignature::from_bytes([0x53, 0x75, 0x70, 0x72]), RingtoneSignature::Supreme);
        assert_eq!(RingtoneSignature::from_bytes([0x53, 0x75, 0x72, 0x75]), RingtoneSignature::SuruArpeggio);
        assert_eq!(RingtoneSignature::from_bytes([0x54, 0x69, 0x6D, 0x65]), RingtoneSignature::TimeNotLost);
        assert_eq!(RingtoneSignature::from_bytes([0x57, 0x6F, 0x6F, 0x64]), RingtoneSignature::WoodenDrive);
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
        assert_eq!(RingtoneSignature::from_str("MonkeyIsland").unwrap(), RingtoneSignature::MonkeyIsland);
        assert_eq!(RingtoneSignature::from_str("AlarmSynth").unwrap(), RingtoneSignature::AlarmSynth);
        assert_eq!(RingtoneSignature::from_str("ArrayMbira").unwrap(), RingtoneSignature::ArrayMbira);
        assert_eq!(RingtoneSignature::from_str("Bliss").unwrap(), RingtoneSignature::Bliss);
        assert_eq!(RingtoneSignature::from_str("Celestial").unwrap(), RingtoneSignature::Celestial);
        assert_eq!(RingtoneSignature::from_str("Entropy").unwrap(), RingtoneSignature::Entropy);
        assert_eq!(RingtoneSignature::from_str("GlassMarimba").unwrap(), RingtoneSignature::GlassMarimba);
        assert_eq!(RingtoneSignature::from_str("HaloPentatonic").unwrap(), RingtoneSignature::HaloPentatonic);
        assert_eq!(RingtoneSignature::from_str("Harmonics").unwrap(), RingtoneSignature::Harmonics);
        assert_eq!(RingtoneSignature::from_str("HarpArp").unwrap(), RingtoneSignature::HarpArp);
        assert_eq!(RingtoneSignature::from_str("KotoChords").unwrap(), RingtoneSignature::KotoChords);
        assert_eq!(RingtoneSignature::from_str("Sakenointi").unwrap(), RingtoneSignature::Sakenointi);
        assert_eq!(RingtoneSignature::from_str("SamsSong").unwrap(), RingtoneSignature::SamsSong);
        assert_eq!(RingtoneSignature::from_str("Soul").unwrap(), RingtoneSignature::Soul);
        assert_eq!(RingtoneSignature::from_str("Sparkle").unwrap(), RingtoneSignature::Sparkle);
        assert_eq!(RingtoneSignature::from_str("Supreme").unwrap(), RingtoneSignature::Supreme);
        assert_eq!(RingtoneSignature::from_str("SuruArpeggio").unwrap(), RingtoneSignature::SuruArpeggio);
        assert_eq!(RingtoneSignature::from_str("TimeNotLost").unwrap(), RingtoneSignature::TimeNotLost);
        assert_eq!(RingtoneSignature::from_str("WoodenDrive").unwrap(), RingtoneSignature::WoodenDrive);
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
