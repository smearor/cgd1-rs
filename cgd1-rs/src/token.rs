use rand::Rng;

/// 16-byte authentication token for CGD1 BLE protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthToken([u8; 16]);

impl AuthToken {
    /// Generate a new random token.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Create a token from raw bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Get the raw token bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Get the token bytes as a payload slice for command frames.
    pub fn payload(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_random_tokens() {
        let token1 = AuthToken::generate();
        let token2 = AuthToken::generate();
        assert_ne!(token1, token2);
    }

    #[test]
    fn from_bytes_roundtrip() {
        let bytes = [0u8; 16];
        let token = AuthToken::from_bytes(bytes);
        assert_eq!(token.as_bytes(), &bytes);
    }
}
