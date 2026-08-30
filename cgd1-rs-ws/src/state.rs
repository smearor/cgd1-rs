use std::sync::Arc;

use cgd1_rs::Backend;
use cgd1_rs::BleTransport;
use cgd1_rs::ClockDevice;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::ClockScanner;
use cgd1_rs::FileTokenStore;
use cgd1_rs::MacAddress;
use cgd1_rs::TokenStore;

use crate::error::ServerError;

/// Shared server state containing the BLE manager and transport.
///
/// The transport is kept alive for the lifetime of the server to maintain
/// BLE adapter connectivity. The manager tracks connected devices.
#[derive(Clone)]
pub struct ServerState {
    /// The BLE transport (kept alive to maintain the adapter connection).
    #[allow(dead_code)]
    transport: Arc<dyn BleTransport>,
    /// Core library BLE manager for device connections.
    manager: Arc<ClockManager>,
    /// Token store for device authentication.
    token_store: Arc<FileTokenStore>,
}

impl ServerState {
    /// Create new server state with the given backend.
    ///
    /// The `ClockManager` is created internally from the transport.
    pub async fn new(backend: Backend) -> Result<Self, ClockError> {
        let transport = backend.create_transport().await?;
        let manager = Arc::new(ClockManager::new(transport.clone()));
        let token_store = Arc::new(FileTokenStore::default_directory());
        Ok(Self {
            transport,
            manager,
            token_store,
        })
    }

    /// Get a reference to the BLE manager.
    pub fn manager(&self) -> &ClockManager {
        &self.manager
    }

    /// Create a scanner for discovering CGD1 devices.
    pub fn scanner(&self) -> ClockScanner {
        self.manager.scanner()
    }

    /// Connect to a device by MAC address and authenticate.
    ///
    /// The token is loaded from the file store if available, or generated
    /// if not. After a successful `sync_time`, the token is persisted.
    pub async fn connect(&self, mac: &MacAddress) -> Result<ClockDevice, ServerError> {
        let device = self.manager.connect(mac).await?;
        device.set_token_store(self.token_store.clone() as Arc<dyn TokenStore>);
        let token_result = self.token_store.load_or_generate(mac);
        device.authenticate(&token_result).await?;
        Ok(device)
    }

    /// Get a connected device by MAC address.
    ///
    /// Returns `ServerError::NotConnected` if the device is not connected.
    pub async fn device(&self, mac: &MacAddress) -> Result<ClockDevice, ServerError> {
        self.manager.device(mac).await.ok_or_else(|| ServerError::NotConnected { address: *mac })
    }

    /// Disconnect from a device.
    pub async fn disconnect(&self, mac: &MacAddress) -> Result<(), ServerError> {
        self.manager.disconnect(mac).await?;
        Ok(())
    }
}
