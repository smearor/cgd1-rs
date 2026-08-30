use std::sync::Arc;

use cgd1_rs::AuthFailedError;
use cgd1_rs::Backend;
use cgd1_rs::BleTransport;
use cgd1_rs::ClockDevice;
use cgd1_rs::ClockError;
use cgd1_rs::ClockManager;
use cgd1_rs::FileTokenStore;
use cgd1_rs::MacAddress;
use cgd1_rs::TokenStore;

use crate::error::CliError;

/// An authenticated connection to a CGD1 device.
pub struct DeviceConnection {
    /// The BLE transport (kept alive to maintain the connection).
    #[allow(dead_code)]
    transport: Arc<dyn BleTransport>,
    /// The authenticated device handle.
    device: ClockDevice,
}

impl DeviceConnection {
    /// Get the device handle.
    pub fn device(&self) -> &ClockDevice {
        &self.device
    }

    /// Connect to a device by MAC address and authenticate.
    ///
    /// The token is loaded from the file store if available, or generated
    /// if not. The token is persisted after `sync_time` succeeds.
    pub async fn connect(mac: &MacAddress, backend: Backend) -> Result<Self, CliError> {
        let transport = backend.create_transport().await?;
        let manager = ClockManager::new(transport.clone());
        let device = manager.connect(mac).await?;

        let store = Arc::new(FileTokenStore::default_directory());
        let token_result = store.load_or_generate(mac);

        device.set_token_store(store.clone() as Arc<dyn TokenStore>).await;

        match device.authenticate(&token_result).await {
            Ok(()) => Ok(Self { transport, device }),
            Err(ClockError::AuthFailed(err)) => Err(ClockError::AuthFailed(AuthFailedError {
                is_new_token: token_result.is_new(),
                token_path: Some(store.directory().join(mac.to_string().replace(':', "_")).display().to_string()),
                ..err
            })
            .into()),
            Err(e) => Err(e.into()),
        }
    }

    /// Connect with a token store reference for manual token management.
    ///
    /// Used by `sync_time` which needs to persist the token after a
    /// successful time sync.
    pub async fn connect_with_store(mac: &MacAddress, backend: Backend) -> Result<(Self, Arc<FileTokenStore>), CliError> {
        let transport = backend.create_transport().await?;
        let manager = ClockManager::new(transport.clone());
        let device = manager.connect(mac).await?;

        let store = Arc::new(FileTokenStore::default_directory());
        let token_result = store.load_or_generate(mac);

        device.set_token_store(store.clone() as Arc<dyn TokenStore>).await;

        match device.authenticate(&token_result).await {
            Ok(()) => Ok((Self { transport, device }, store)),
            Err(ClockError::AuthFailed(err)) => Err(ClockError::AuthFailed(AuthFailedError {
                is_new_token: token_result.is_new(),
                token_path: Some(store.directory().join(mac.to_string().replace(':', "_")).display().to_string()),
                ..err
            })
            .into()),
            Err(e) => Err(e.into()),
        }
    }
}
