use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::ClockError;
use crate::error::Result;
use crate::types::MacAddress;

/// Persistent store of MAC addresses for devices that have been successfully
/// connected at least once.
///
/// The list is stored as a JSON file in the XDG data directory
/// (`~/.local/share/cgd1-rs/known_devices.json` on Linux).
/// Battery levels from advertising scans are persisted in a separate file
/// (`~/.local/share/cgd1-rs/battery_cache.json`).
pub struct KnownDeviceStore {
    /// Path to the JSON file for known device addresses.
    path: PathBuf,
    /// Path to the JSON file for cached battery levels.
    battery_path: PathBuf,
}

impl KnownDeviceStore {
    /// Create a known-device store at a specific file path.
    pub fn new(path: PathBuf) -> Self {
        let battery_path = path.parent().unwrap_or_else(|| std::path::Path::new(".")).join("battery_cache.json");
        Self { path, battery_path }
    }

    /// Create a known-device store at the default platform data directory.
    ///
    /// Uses the XDG data directory (`~/.local/share/cgd1-rs/known_devices.json` on Linux).
    pub fn default_path() -> Self {
        let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::new(data.join("cgd1-rs").join("known_devices.json"))
    }

    /// Load the list of known device addresses from disk.
    ///
    /// Returns an empty list if the file does not exist or is invalid.
    pub fn load(&self) -> Vec<MacAddress> {
        match std::fs::read_to_string(&self.path) {
            Ok(content) => {
                let strings: Vec<String> = serde_json::from_str(&content).unwrap_or_default();
                strings.iter().filter_map(|s| MacAddress::parse(s).ok()).collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Save the list of known device addresses to disk.
    pub fn save(&self, addresses: &[MacAddress]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(ClockError::from)?;
        }
        let strings: Vec<String> = addresses.iter().map(|a| a.to_string()).collect();
        let json = serde_json::to_string(&strings).map_err(ClockError::from)?;
        std::fs::write(&self.path, json).map_err(ClockError::from)?;
        Ok(())
    }

    /// Add a device address to the store if not already present.
    ///
    /// Returns `true` if the address was newly added.
    pub fn add(&self, address: &MacAddress) -> Result<bool> {
        let mut devices = self.load();
        if devices.iter().any(|a| a == address) {
            return Ok(false);
        }
        devices.push(*address);
        self.save(&devices)?;
        Ok(true)
    }

    /// Load cached battery levels from disk.
    ///
    /// Returns an empty map if the file does not exist or is invalid.
    pub fn load_battery(&self) -> HashMap<MacAddress, u8> {
        match std::fs::read_to_string(&self.battery_path) {
            Ok(content) => {
                let map: HashMap<String, u8> = serde_json::from_str(&content).unwrap_or_default();
                map.iter().filter_map(|(s, v)| MacAddress::parse(s).ok().map(|a| (a, *v))).collect()
            }
            Err(_) => HashMap::new(),
        }
    }

    /// Save a battery level for a specific device address.
    pub fn save_battery(&self, address: &MacAddress, level: u8) -> Result<()> {
        if let Some(parent) = self.battery_path.parent() {
            std::fs::create_dir_all(parent).map_err(ClockError::from)?;
        }
        let mut map = self.load_battery();
        map.insert(*address, level);
        let json_map: HashMap<String, u8> = map.iter().map(|(a, v)| (a.to_string(), *v)).collect();
        let json = serde_json::to_string(&json_map).map_err(ClockError::from)?;
        std::fs::write(&self.battery_path, json).map_err(ClockError::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn known_device_store_roundtrip() {
        let dir = std::env::temp_dir().join("cgd1_test_known_devices_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        let store = KnownDeviceStore::new(dir.join("known_devices.json"));

        let addr1 = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        let addr2 = MacAddress::parse("11:22:33:44:55:66").unwrap();

        assert!(store.add(&addr1).unwrap());
        assert!(!store.add(&addr1).unwrap());
        assert!(store.add(&addr2).unwrap());

        let loaded = store.load();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains(&addr1));
        assert!(loaded.contains(&addr2));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn known_device_store_missing_file() {
        let dir = std::env::temp_dir().join("cgd1_test_known_devices_missing");
        let _ = fs::remove_dir_all(&dir);
        let store = KnownDeviceStore::new(dir.join("known_devices.json"));

        let loaded = store.load();
        assert!(loaded.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn battery_cache_roundtrip() {
        let dir = std::env::temp_dir().join("cgd1_test_battery_cache_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        let store = KnownDeviceStore::new(dir.join("known_devices.json"));

        let addr = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();

        assert!(store.load_battery().is_empty());

        store.save_battery(&addr, 85).unwrap();
        let loaded = store.load_battery();
        assert_eq!(loaded.get(&addr), Some(&85));

        store.save_battery(&addr, 42).unwrap();
        let loaded = store.load_battery();
        assert_eq!(loaded.get(&addr), Some(&42));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn battery_cache_missing_file() {
        let dir = std::env::temp_dir().join("cgd1_test_battery_cache_missing");
        let _ = fs::remove_dir_all(&dir);
        let store = KnownDeviceStore::new(dir.join("known_devices.json"));

        assert!(store.load_battery().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }
}
