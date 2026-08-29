use crate::error::ClockError;
use crate::error::Result;
use crate::types::Humidity;
use crate::types::Temperature;

/// Parsed sensor notification from the Sensor Notify characteristic.
///
/// Format: `[0x00] [TempLo] [TempHi] [HumLo] [HumHi]` (5 bytes).
/// Temperature is a signed 16-bit little-endian value, scaled by / 100.
/// Humidity is an unsigned 16-bit little-endian value, scaled by / 100.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorNotification {
    /// Temperature in degrees Celsius.
    pub temperature: Temperature,
    /// Relative humidity in percent.
    pub humidity: Humidity,
}

impl SensorNotification {
    /// Parse a raw sensor notification payload.
    ///
    /// The first byte must be `0x00` (sensor data header). Temperature and
    /// humidity follow as little-endian 16-bit values, each divided by 100.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < 5 {
            return Err(ClockError::Parse("sensor notification too short".to_string()));
        }

        if payload[0] != 0x00 {
            return Err(ClockError::Parse(format!("unexpected sensor header byte: 0x{:02X}", payload[0])));
        }

        let temp_raw = i16::from_le_bytes([payload[1], payload[2]]);
        let humidity_raw = u16::from_le_bytes([payload[3], payload[4]]);

        Ok(Self {
            temperature: Temperature::new(temp_raw as f32 / 100.0),
            humidity: Humidity::new(humidity_raw as f32 / 100.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_notification() {
        // [0x00] [TempLo] [TempHi] [HumLo] [HumHi]
        // Temp = 2345 → 23.45 °C, Hum = 5600 → 56.00 %
        let payload = [0x00, 0x29, 0x09, 0xE0, 0x15];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), 23.45);
        assert_eq!(sensor.humidity.value(), 56.0);
    }

    #[test]
    fn parse_negative_temperature() {
        // Temp = -550 → -5.50 °C, LE bytes: [0xDA, 0xFD]
        let payload = [0x00, 0xDA, 0xFD, 0x64, 0x00];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), -5.5);
        assert_eq!(sensor.humidity.value(), 1.0);
    }

    #[test]
    fn parse_zero_values() {
        let payload = [0x00, 0x00, 0x00, 0x00, 0x00];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), 0.0);
        assert_eq!(sensor.humidity.value(), 0.0);
    }

    #[test]
    fn parse_too_short() {
        let payload = [0x00, 0x29, 0x09];
        assert!(SensorNotification::parse(&payload).is_err());
    }

    #[test]
    fn parse_wrong_header() {
        let payload = [0x01, 0x29, 0x09, 0xE0, 0x15];
        assert!(SensorNotification::parse(&payload).is_err());
    }
}
