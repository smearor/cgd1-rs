use tracing::debug;

use crate::error::ClockError;
use crate::error::Result;
use crate::types::BatteryLevel;
use crate::types::Humidity;
use crate::types::Temperature;

/// Parsed sensor notification from the Sensor Notify characteristic.
///
/// Base format: `[0x00] [TempLo] [TempHi] [HumLo] [HumHi]` (5 bytes).
/// Temperature is a signed 16-bit little-endian value, scaled by / 100.
/// Humidity is an unsigned 16-bit little-endian value, scaled by / 100.
///
/// Some firmware revisions append an extra battery byte after the humidity
/// field, making the payload 6 bytes. When present, it is interpreted as a
/// battery percentage (0–100).
#[derive(Debug, Clone, PartialEq)]
pub struct SensorNotification {
    /// Temperature in degrees Celsius.
    pub temperature: Temperature,
    /// Relative humidity in percent.
    pub humidity: Humidity,
    /// Battery level percentage, if included in the notification payload.
    pub battery: Option<BatteryLevel>,
}

impl SensorNotification {
    /// Create a new sensor notification from temperature and humidity values.
    pub const fn new(temperature: Temperature, humidity: Humidity) -> Self {
        Self {
            temperature,
            humidity,
            battery: None,
        }
    }

    /// Create a new sensor notification with a battery level.
    pub const fn with_battery(temperature: Temperature, humidity: Humidity, battery: BatteryLevel) -> Self {
        Self {
            temperature,
            humidity,
            battery: Some(battery),
        }
    }

    /// Encode the sensor notification into the payload format.
    ///
    /// Without battery: `[0x00] [TempLo] [TempHi] [HumLo] [HumHi]` (5 bytes).
    /// With battery: `[0x00] [TempLo] [TempHi] [HumLo] [HumHi] [Battery]` (6 bytes).
    /// Temperature is a signed 16-bit little-endian value, scaled by * 100.
    /// Humidity is an unsigned 16-bit little-endian value, scaled by * 100.
    pub fn encode(&self) -> Vec<u8> {
        let temp_raw = (self.temperature.value() * 100.0) as i16;
        let hum_raw = (self.humidity.value() * 100.0) as u16;
        let mut buf = vec![
            0x00,
            (temp_raw & 0xFF) as u8,
            ((temp_raw >> 8) & 0xFF) as u8,
            (hum_raw & 0xFF) as u8,
            ((hum_raw >> 8) & 0xFF) as u8,
        ];
        if let Some(battery) = self.battery {
            buf.push(battery.value());
        }
        buf
    }

    /// Parse a raw sensor notification payload.
    ///
    /// The first byte must be `0x00` (sensor data header). Temperature and
    /// humidity follow as little-endian 16-bit values, each divided by 100.
    /// If a sixth byte is present, it is interpreted as a battery percentage.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < 5 {
            return Err(ClockError::Parse("sensor notification too short".to_string()));
        }

        if payload[0] != 0x00 {
            return Err(ClockError::Parse(format!("unexpected sensor header byte: 0x{:02X}", payload[0])));
        }

        let temp_raw = i16::from_le_bytes([payload[1], payload[2]]);
        let humidity_raw = u16::from_le_bytes([payload[3], payload[4]]);

        let battery = if payload.len() >= 6 {
            let raw = payload[5];
            if raw <= 100 {
                Some(BatteryLevel::new(raw))
            } else {
                debug!("sensor notification battery byte {} > 100, ignoring", raw);
                None
            }
        } else {
            None
        };

        Ok(Self {
            temperature: Temperature::new(temp_raw as f32 / 100.0),
            humidity: Humidity::new(humidity_raw as f32 / 100.0),
            battery,
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
        assert!(sensor.battery.is_none());
    }

    #[test]
    fn parse_negative_temperature() {
        // Temp = -550 → -5.50 °C, LE bytes: [0xDA, 0xFD]
        let payload = [0x00, 0xDA, 0xFD, 0x64, 0x00];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), -5.5);
        assert_eq!(sensor.humidity.value(), 1.0);
        assert!(sensor.battery.is_none());
    }

    #[test]
    fn parse_zero_values() {
        let payload = [0x00, 0x00, 0x00, 0x00, 0x00];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), 0.0);
        assert_eq!(sensor.humidity.value(), 0.0);
        assert!(sensor.battery.is_none());
    }

    #[test]
    fn parse_with_battery() {
        // [0x00] [TempLo] [TempHi] [HumLo] [HumHi] [Battery=85]
        let payload = [0x00, 0x29, 0x09, 0xE0, 0x15, 0x55];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), 23.45);
        assert_eq!(sensor.humidity.value(), 56.0);
        assert_eq!(sensor.battery.map(|b| b.value()), Some(85));
    }

    #[test]
    fn parse_with_invalid_battery_ignores() {
        // Battery byte = 150 (>100) should be ignored
        let payload = [0x00, 0x29, 0x09, 0xE0, 0x15, 0x96];
        let sensor = SensorNotification::parse(&payload).unwrap();
        assert_eq!(sensor.temperature.value(), 23.45);
        assert_eq!(sensor.humidity.value(), 56.0);
        assert!(sensor.battery.is_none());
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

    #[test]
    fn encode_round_trip() {
        let sensor = SensorNotification::new(Temperature::new(23.45), Humidity::new(56.0));
        let encoded = sensor.encode();
        let parsed = SensorNotification::parse(&encoded).unwrap();
        assert_eq!(parsed, sensor);
    }

    #[test]
    fn encode_round_trip_with_battery() {
        let sensor = SensorNotification::with_battery(Temperature::new(23.45), Humidity::new(56.0), BatteryLevel::new(85));
        let encoded = sensor.encode();
        assert_eq!(encoded.len(), 6);
        let parsed = SensorNotification::parse(&encoded).unwrap();
        assert_eq!(parsed, sensor);
    }

    #[test]
    fn encode_negative_temperature() {
        let sensor = SensorNotification::new(Temperature::new(-5.5), Humidity::new(1.0));
        let encoded = sensor.encode();
        let parsed = SensorNotification::parse(&encoded).unwrap();
        assert_eq!(parsed.temperature.value(), -5.5);
        assert_eq!(parsed.humidity.value(), 1.0);
    }
}
