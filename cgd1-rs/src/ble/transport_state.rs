use std::collections::HashMap;

use btleplug::api::Characteristic;
use btleplug::platform::Peripheral;
use uuid::Uuid;

/// Per-device connection state for the btleplug transport.
///
/// Each connected device has its own peripheral, characteristics, and
/// notification stream. The scan filter is kept at the transport level
/// since scanning is global (not per-device).
pub struct DeviceConnection {
    /// The connected BLE peripheral.
    pub peripheral: Peripheral,
    /// Discovered GATT characteristics keyed by UUID.
    pub characteristics: HashMap<Uuid, Characteristic>,
}

impl DeviceConnection {
    pub fn new(peripheral: Peripheral, characteristics: HashMap<Uuid, Characteristic>) -> Self {
        Self { peripheral, characteristics }
    }
}

/// Service-data UUID to filter advertisements by, stored separately
/// from per-device connection state.
pub struct ScanState {
    /// Service-data UUID to filter advertisements by.
    pub scan_filter_uuid: Option<Uuid>,
}

impl ScanState {
    pub fn new() -> Self {
        Self { scan_filter_uuid: None }
    }
}

impl Default for ScanState {
    fn default() -> Self {
        Self::new()
    }
}
