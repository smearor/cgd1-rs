use crate::error::ClockError;
use crate::error::Result;
use crate::types::BatteryLevel;
use crate::types::Humidity;
use crate::types::MacAddress;
use crate::types::Temperature;

use serde::Deserialize;
use serde::Serialize;

/// Parsed sensor data from a passive BLE advertisement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvertisementData {
    /// Device MAC address (reversed from advertisement).
    pub mac: MacAddress,
    /// Temperature in degrees Celsius (scaled by / 10 or / 100, firmware-dependent).
    pub temperature: Temperature,
    /// Relative humidity in percent (scaled by / 10 or / 100).
    pub humidity: Humidity,
    /// Battery level percentage (0-100, bit 7 masked).
    pub battery: BatteryLevel,
}

impl AdvertisementData {
    /// Parse a raw service-data payload into advertisement data.
    ///
    /// The payload follows the TLV format documented in `docs/BLE.md` section 8.
    /// TLV blocks may appear in any order depending on firmware revision
    /// (e.g., some revisions emit Battery before Temp/Humidity). This parser
    /// iterates over the payload and extracts each known type dynamically
    /// rather than assuming a fixed byte layout.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < 8 {
            return Err(ClockError::Parse("advertisement too short for header".to_string()));
        }
        let mac = {
            let mut mac = [0u8; 6];
            mac.copy_from_slice(&payload[2..8]);
            mac.reverse();
            MacAddress::new(mac)
        };

        let mut temperature: Option<f32> = None;
        let mut humidity: Option<f32> = None;
        let mut battery: Option<u8> = None;

        let mut offset = 8;
        while offset + 2 <= payload.len() {
            let tlv_type = payload[offset];
            let tlv_len = payload[offset + 1] as usize;
            let value_start = offset + 2;
            let value_end = value_start + tlv_len;

            if value_end > payload.len() {
                break;
            }

            match tlv_type {
                0x01 if tlv_len >= 4 => {
                    let temp_raw = i16::from_le_bytes([payload[value_start], payload[value_start + 1]]);
                    let hum_raw = u16::from_le_bytes([payload[value_start + 2], payload[value_start + 3]]);
                    temperature = Some(if temp_raw.abs() > 1000 {
                        temp_raw as f32 / 100.0
                    } else {
                        temp_raw as f32 / 10.0
                    });
                    humidity = Some(if hum_raw > 1000 { hum_raw as f32 / 100.0 } else { hum_raw as f32 / 10.0 });
                }
                0x02 if tlv_len >= 1 => {
                    battery = Some(payload[value_start] & 0x7F);
                }
                _ => {}
            }

            offset = value_end;
        }

        let temperature = temperature.ok_or_else(|| ClockError::Parse("advertisement missing temperature TLV (0x01)".to_string()))?;
        let humidity = humidity.ok_or_else(|| ClockError::Parse("advertisement missing humidity TLV (0x01)".to_string()))?;
        let battery = battery.ok_or_else(|| ClockError::Parse("advertisement missing battery TLV (0x02)".to_string()))?;

        Ok(Self {
            mac,
            temperature: Temperature::new(temperature),
            humidity: Humidity::new(humidity),
            battery: BatteryLevel::new(battery),
        })
    }

    /// Format the MAC address as a colon-separated string.
    pub fn mac_string(&self) -> String {
        self.mac.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_advertisement() {
        // [flags] [model_id] [MAC 6B reversed] [TLV: temp+hum] [TLV: battery]
        let payload = [
            0x08, 0x0C, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, // MAC (will be reversed)
            0x01, 0x04, 0x64, 0x00, // Temp = 100 → 10.0 °C
            0xC8, 0x00, // Humidity = 200 → 20.0 %
            0x02, 0x01, 0x5A, // Battery = 90
        ];
        let data = AdvertisementData::parse(&payload).unwrap();
        assert_eq!(data.mac, MacAddress::new([0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA]));
        assert_eq!(data.temperature.value(), 10.0);
        assert_eq!(data.humidity.value(), 20.0);
        assert_eq!(data.battery.value(), 90);
    }

    #[test]
    fn parse_battery_first() {
        // TLV blocks in different order: battery before temp/humidity
        let payload = [
            0x88, 0x0C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x02, 0x01, 0x63, // Battery = 99
            0x01, 0x04, 0x96, 0x00, // Temp = 150 → 15.0 °C
            0x2C, 0x01, // Humidity = 300 → 30.0 %
        ];
        let data = AdvertisementData::parse(&payload).unwrap();
        assert_eq!(data.battery.value(), 99);
        assert_eq!(data.temperature.value(), 15.0);
        assert_eq!(data.humidity.value(), 30.0);
    }

    #[test]
    fn parse_high_scale() {
        // Values > 1000 use /100 scaling
        let payload = [
            0x08, 0x0C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x01, 0x04, 0xDC, 0x05, // Temp = 1500 → 15.0 °C (high scale)
            0x10, 0x27, // Humidity = 10000 → 100.0 % (high scale)
            0x02, 0x01, 0x32, // Battery = 50
        ];
        let data = AdvertisementData::parse(&payload).unwrap();
        assert_eq!(data.temperature.value(), 15.0);
        assert_eq!(data.humidity.value(), 100.0);
        assert_eq!(data.battery.value(), 50);
    }

    #[test]
    fn parse_too_short() {
        let payload = [0x08, 0x0C, 0x01];
        assert!(AdvertisementData::parse(&payload).is_err());
    }

    #[test]
    fn parse_missing_battery() {
        let payload = [0x08, 0x0C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x01, 0x04, 0x64, 0x00, 0xC8, 0x00];
        assert!(AdvertisementData::parse(&payload).is_err());
    }

    #[test]
    fn parse_battery_mask() {
        // Bit 7 set should be masked
        let payload = [
            0x08, 0x0C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x01, 0x04, 0x64, 0x00, 0xC8, 0x00, 0x02, 0x01, 0x8A, // 0x8A & 0x7F = 0x0A = 10
        ];
        let data = AdvertisementData::parse(&payload).unwrap();
        assert_eq!(data.battery.value(), 10);
    }
}
