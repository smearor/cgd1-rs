/// Relative humidity in percent.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Humidity(f32);

impl Humidity {
    /// Create a new humidity value.
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Get the raw value in percent.
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Humidity {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value() {
        let hum = Humidity::new(45.5);
        assert_eq!(hum.value(), 45.5);
    }

    #[test]
    fn from_f32() {
        let hum: Humidity = 50.0.into();
        assert_eq!(hum.value(), 50.0);
    }

    #[test]
    fn ordering() {
        let a = Humidity::new(30.0);
        let b = Humidity::new(60.0);
        assert!(a < b);
    }
}
