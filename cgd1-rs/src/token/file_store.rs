use std::path::PathBuf;

use crate::error::ClockError;
use crate::error::Result;
use crate::token::AuthToken;
use crate::token::TokenResult;
use crate::token::TokenStore;
use crate::types::MacAddress;

/// File-based token store using a simple directory of files keyed by MAC.
///
/// Each token is stored as a 16-byte binary file named after the MAC address
/// (with colons replaced by underscores for filesystem compatibility).
pub struct FileTokenStore {
    /// Directory path for token files.
    directory: PathBuf,
}

impl FileTokenStore {
    /// Create a new file token store at the given directory.
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    /// Create a file token store at the default platform data directory.
    ///
    /// Uses the XDG data directory (`~/.local/share/cgd1-rs` on Linux).
    /// Tokens are persistent runtime data, not configuration.
    pub fn default_directory() -> Self {
        let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::new(data.join("cgd1-rs"))
    }

    /// Get the directory path used by this token store.
    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    /// Build the file path for a given MAC address.
    fn path_for(&self, address: &MacAddress) -> PathBuf {
        let filename = address.to_string().replace(':', "_");
        self.directory.join(filename)
    }

    /// Load an existing token or generate a new one.
    ///
    /// Returns a [`TokenResult`] indicating whether the token was loaded
    /// from storage or freshly generated.
    ///
    /// **Warning**: Generating a new token when the device already has a paired
    /// token will cause authentication to fail. The device requires an explicit
    /// unpairing (factory reset) before accepting a new token.
    pub fn load_or_generate(&self, address: &MacAddress) -> TokenResult {
        if let Some(token) = self.load(address) {
            TokenResult::existing(token)
        } else {
            TokenResult::generated(AuthToken::generate())
        }
    }
}

impl TokenStore for FileTokenStore {
    fn load(&self, address: &MacAddress) -> Option<AuthToken> {
        let path = self.path_for(address);
        let bytes = std::fs::read(path).ok()?;
        if bytes.len() == 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes);
            Some(AuthToken::from_bytes(arr))
        } else {
            None
        }
    }

    fn save(&self, address: &MacAddress, token: &AuthToken) -> Result<()> {
        std::fs::create_dir_all(&self.directory).map_err(ClockError::from)?;
        let path = self.path_for(address);
        std::fs::write(path, token.as_bytes()).map_err(ClockError::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn file_token_store_roundtrip() {
        let dir = std::env::temp_dir().join("cgd1_test_token_store_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        let store = FileTokenStore::new(dir.clone());

        let address = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let token = AuthToken::generate();

        store.save(&address, &token).unwrap();
        let loaded = store.load(&address).unwrap();
        assert_eq!(token, loaded);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_token_store_missing_token() {
        let dir = std::env::temp_dir().join("cgd1_test_token_store_missing");
        let _ = fs::remove_dir_all(&dir);
        let store = FileTokenStore::new(dir.clone());

        let address = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        assert!(store.load(&address).is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_generate_returns_new_when_missing() {
        let dir = std::env::temp_dir().join("cgd1_test_load_or_generate_new");
        let _ = fs::remove_dir_all(&dir);
        let store = FileTokenStore::new(dir.clone());

        let address = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let result = store.load_or_generate(&address);
        assert!(result.is_new());
        assert_eq!(result.as_bytes().len(), 16);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_generate_returns_existing_when_present() {
        let dir = std::env::temp_dir().join("cgd1_test_load_or_generate_existing");
        let _ = fs::remove_dir_all(&dir);
        let store = FileTokenStore::new(dir.clone());

        let address = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let token = AuthToken::generate();
        store.save(&address, &token).unwrap();

        let result = store.load_or_generate(&address);
        assert!(!result.is_new());
        assert_eq!(*result, token);

        let _ = fs::remove_dir_all(&dir);
    }
}
