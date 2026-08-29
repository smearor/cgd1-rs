/// Temperature in degrees Celsius.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Temperature(f32);

impl Temperature {
    /// Create a new temperature value.
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Get the raw value in degrees Celsius.
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Temperature {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value() {
        let temp = Temperature::new(23.5);
        assert_eq!(temp.value(), 23.5);
    }

    #[test]
    fn from_f32() {
        let temp: Temperature = 10.0.into();
        assert_eq!(temp.value(), 10.0);
    }

    #[test]
    fn ordering() {
        let a = Temperature::new(10.0);
        let b = Temperature::new(20.0);
        assert!(a < b);
    }
}
