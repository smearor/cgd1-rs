use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

use crate::BleTransport;
use crate::CharacteristicUuid;
use crate::device::ClockDevice;
use crate::error::ClockError;
use crate::error::Result;
use crate::scanner::ClockScanner;
use crate::token::AuthToken;
use crate::token::TokenStore;
use crate::types::MacAddress;

/// Manages BLE connections to CGD1 alarm clocks.
///
/// Owns the BLE transport and tracks all connected devices by MAC address.
/// Provides methods for scanning, connecting, disconnecting, and retrieving
/// devices.
pub struct ClockManager {
    transport: Arc<dyn BleTransport>,
    devices: Mutex<HashMap<MacAddress, ClockDevice>>,
}

impl ClockManager {
    /// Create a new manager with the given transport.
    pub fn new(transport: Arc<dyn BleTransport>) -> Self {
        Self {
            transport,
            devices: Mutex::new(HashMap::new()),
        }
    }

    /// Create a scanner for discovering CGD1 devices.
    pub fn scanner(&self) -> ClockScanner {
        ClockScanner::new(self.transport.clone())
    }

    /// Connect to a device by MAC address.
    ///
    /// Performs the full connection lifecycle:
    /// 1. BLE connect
    /// 2. Subscribe to Auth Notify, Data Notify, and Sensor Notify
    /// 3. Spawn the notification task
    ///
    /// Authentication is performed separately via [`ClockDevice::authenticate`].
    pub async fn connect(&self, address: &MacAddress) -> Result<ClockDevice> {
        {
            let devices = self.devices.lock().await;
            if devices.contains_key(address) {
                return Err(ClockError::AlreadyConnected);
            }
        }

        self.transport.connect(address).await?;

        let characteristics = [CharacteristicUuid::AuthNotify, CharacteristicUuid::DataNotify, CharacteristicUuid::SensorNotify];

        for char_uuid in &characteristics {
            self.transport.subscribe(*char_uuid).await?;
        }

        let device = ClockDevice::new(self.transport.clone(), *address);
        device.spawn_notification_task();

        {
            let mut devices = self.devices.lock().await;
            devices.insert(*address, device.clone());
        }

        info!(%address, "device connected and subscribed");
        Ok(device)
    }

    /// Disconnect from a device and remove it from the manager.
    pub async fn disconnect(&self, address: &MacAddress) -> Result<()> {
        let device = {
            let mut devices = self.devices.lock().await;
            devices.remove(address)
        };

        if let Some(device) = device {
            device.disconnect().await?;
            info!(%address, "device disconnected");
        }
        Ok(())
    }

    /// Disconnect all managed devices.
    pub async fn disconnect_all(&self) -> Result<()> {
        let addresses: Vec<MacAddress> = {
            let devices = self.devices.lock().await;
            devices.keys().copied().collect()
        };

        for address in &addresses {
            let _ = self.disconnect(address).await;
        }
        Ok(())
    }

    /// Get a connected device by MAC address.
    pub async fn device(&self, address: &MacAddress) -> Option<ClockDevice> {
        let devices = self.devices.lock().await;
        devices.get(address).cloned()
    }

    /// List all connected device addresses.
    pub async fn connected_devices(&self) -> Vec<MacAddress> {
        let devices = self.devices.lock().await;
        devices.keys().copied().collect()
    }

    /// Connect and authenticate in one step.
    pub async fn connect_and_authenticate(&self, address: &MacAddress, token: &AuthToken) -> Result<ClockDevice> {
        let device = self.connect(address).await?;
        device.authenticate(token).await?;
        Ok(device)
    }

    /// Connect, set a token store, authenticate, and sync time.
    ///
    /// This is the recommended full connection flow: the token is persisted
    /// only after `sync_time` succeeds, confirming the device accepted it.
    pub async fn connect_authenticate_and_sync(&self, address: &MacAddress, token: &AuthToken, token_store: Arc<dyn TokenStore>) -> Result<ClockDevice> {
        let device = self.connect(address).await?;
        device.set_token_store(token_store).await;
        device.authenticate(token).await?;
        device.sync_time_now().await?;
        Ok(device)
    }
}
