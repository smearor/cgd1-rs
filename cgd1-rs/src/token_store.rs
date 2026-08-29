use std::path::PathBuf;

use crate::error::ClockError;
use crate::error::Result;
use crate::token::AuthToken;
use crate::types::MacAddress;

/// Storage backend for auth tokens.
///
/// A token is only persisted after a privileged command (e.g., time sync)
/// succeeds. An Auth Confirm ACK alone does not prove the token was accepted.
pub trait TokenStore: Send + Sync {
    /// Load the token for a device address.
    fn load(&self, address: &MacAddress) -> Option<AuthToken>;

    /// Save the token for a device address.
    fn save(&self, address: &MacAddress, token: &AuthToken) -> Result<()>;
}

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

    /// Build the file path for a given MAC address.
    fn path_for(&self, address: &MacAddress) -> PathBuf {
        let filename = address.to_string().replace(':', "_");
        self.directory.join(filename)
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
}
