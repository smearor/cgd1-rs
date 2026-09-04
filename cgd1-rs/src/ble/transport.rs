use async_trait::async_trait;
use uuid::Uuid;

use crate::ble::advertisement::AdvertisementData;
use crate::ble::characteristic::CharacteristicUuid;
use crate::ble::notification::BleNotification;
use crate::command::Command;
use crate::command::CommandFrame;
use crate::error::Result;
use crate::types::MacAddress;

/// BLE transport abstraction for CGD1 communication.
///
/// This trait keeps the door open for alternative backends (e.g., `bluer`).
/// The implementation manages multiple simultaneous BLE connections, keyed
/// by MAC address. Scan-related methods are global (not per-device).
#[async_trait]
pub trait BleTransport: Send + Sync {
    /// Start scanning for devices with the given service-data UUID filter.
    async fn start_scan(&self, filter_uuid: Uuid) -> Result<()>;

    /// Stop an active scan.
    async fn stop_scan(&self) -> Result<()>;

    /// Receive the next advertisement event during scanning.
    async fn next_advertisement(&self) -> Option<AdvertisementData>;

    /// Connect to a peripheral by MAC address.
    async fn connect(&self, address: &MacAddress) -> Result<()>;

    /// Disconnect from the peripheral identified by MAC address.
    async fn disconnect(&self, address: &MacAddress) -> Result<()>;

    /// Write data to a characteristic on the device identified by MAC address.
    async fn write(&self, address: &MacAddress, characteristic: CharacteristicUuid, data: &[u8]) -> Result<()>;

    /// Write a command frame to the characteristic associated with the command.
    ///
    /// This is a convenience method that derives the correct GATT characteristic
    /// from the [`Command`] variant, eliminating the risk of sending a command
    /// on the wrong characteristic.
    async fn write_command(&self, address: &MacAddress, command: Command, data: &[u8]) -> Result<()> {
        self.write(address, command.characteristic(), data).await
    }

    /// Encode and write a command frame to the appropriate characteristic.
    ///
    /// Builds a [`CommandFrame`] from the given [`Command`] and payload,
    /// encodes it as `[length] [command] [payload...]`, and sends it to
    /// the characteristic associated with the command.
    async fn write_frame(&self, address: &MacAddress, command: Command, payload: &[u8]) -> Result<()> {
        let frame = CommandFrame::from_command(command, payload.to_vec());
        let encoded = frame.encode();
        self.write(address, command.characteristic(), &encoded).await
    }

    /// Subscribe to notifications from a characteristic on the device
    /// identified by MAC address.
    async fn subscribe(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<()>;

    /// Receive the next notification value from the device identified by
    /// MAC address.
    ///
    /// Returns a [`BleNotification`] containing the characteristic UUID and
    /// raw value bytes, enabling the notification task to route frames
    /// correctly.
    async fn next_notification(&self, address: &MacAddress) -> Option<BleNotification>;

    /// Read a characteristic value from the device identified by MAC address.
    async fn read(&self, address: &MacAddress, characteristic: CharacteristicUuid) -> Result<Vec<u8>>;

    /// Request a larger ATT MTU via the BLE MTU exchange procedure.
    ///
    /// The default ATT MTU is 23 bytes (20 bytes usable payload). Audio
    /// data packets require 130 bytes, so an MTU exchange must succeed
    /// before uploading ringtones. Returns the negotiated MTU on success.
    async fn request_mtu(&self, address: &MacAddress, mtu: u16) -> Result<u16>;

    /// Check if the device identified by MAC address is currently connected.
    fn is_connected(&self, address: &MacAddress) -> bool;
}
