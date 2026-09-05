use std::collections::HashMap;

use btleplug::api::Characteristic;
use btleplug::platform::Peripheral;
use uuid::Uuid;

/// Per-device connection entry holding the peripheral and its discovered GATT characteristics.
///
/// Stored in `BtleplugTransport::connections` keyed by MAC address. The
/// `peripheral` is the btleplug handle used for all BLE operations (read,
/// write, subscribe, disconnect). The `characteristics` map allows looking
/// up a `Characteristic` by its UUID so that callers can address
/// characteristics via the ergonomic [`CharacteristicUuid`] enum.
///
/// [`CharacteristicUuid`]: crate::ble::characteristic::CharacteristicUuid
pub(crate) struct DeviceEntry {
    /// The btleplug peripheral handle for this connected device.
    ///
    /// Cloned cheaply (internally `Arc`-backed) for use in read/write/subscribe
    /// operations without holding the `connections` lock.
    pub peripheral: Peripheral,

    /// Map of GATT characteristic UUID to the discovered `Characteristic`.
    ///
    /// Populated during `connect()` after `discover_services()`. Looked up by
    /// `lookup_peripheral_and_char()` and `lookup_peripheral_by_uuid()` to
    /// resolve a [`CharacteristicUuid`] or raw `Uuid` to the btleplug
    /// `Characteristic` needed for read/write/subscribe calls.
    pub characteristics: HashMap<Uuid, Characteristic>,
}
