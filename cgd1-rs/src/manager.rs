use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

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
        debug!(%address, "manager: connect requested");
        {
            let devices = self.devices.lock().await;
            if devices.contains_key(address) {
                warn!(%address, "manager: already connected");
                return Err(ClockError::AlreadyConnected);
            }
        }

        debug!(%address, "manager: calling transport.connect");
        self.transport.connect(address).await?;

        let characteristics = [CharacteristicUuid::AuthNotify, CharacteristicUuid::DataNotify, CharacteristicUuid::SensorNotify];

        for char_uuid in &characteristics {
            debug!(%address, characteristic = %char_uuid, "manager: subscribing");
            self.transport.subscribe(address, *char_uuid).await?;
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
        debug!(%address, "manager: disconnect requested");
        let device = {
            let mut devices = self.devices.lock().await;
            devices.remove(address)
        };

        if let Some(device) = device {
            device.disconnect().await?;
            info!(%address, "device disconnected");
        } else {
            warn!(%address, "manager: disconnect but device not in map");
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
        debug!(%address, "manager: connect_authenticate_and_sync starting");
        let device = self.connect(address).await?;
        debug!(%address, "manager: setting token store");
        device.set_token_store(token_store).await;
        debug!(%address, "manager: authenticating");
        if let Err(e) = device.authenticate(token).await {
            error!(%address, error = %e, "manager: authentication failed, cleaning up");
            let _ = self.disconnect(address).await;
            return Err(e);
        }
        debug!(%address, "manager: syncing timezone");
        if let Err(e) = device.sync_timezone().await {
            warn!(%address, error = %e, "manager: sync_timezone failed, continuing with sync_time");
        }
        debug!(%address, "manager: syncing time");
        if let Err(e) = device.sync_time_now().await {
            error!(%address, error = %e, "manager: sync_time failed, cleaning up");
            let _ = self.disconnect(address).await;
            return Err(e);
        }
        info!(%address, "manager: connect, auth, and sync complete");
        Ok(device)
    }
}
