use std::collections::HashMap;

use btleplug::api::Characteristic;
use btleplug::platform::Peripheral;
use uuid::Uuid;

/// Internal state for the btleplug transport.
pub struct TransportState {
    /// Currently connected peripheral.
    pub peripheral: Option<Peripheral>,
    /// Discovered GATT characteristics keyed by UUID.
    pub characteristics: HashMap<Uuid, Characteristic>,
    /// Service-data UUID to filter advertisements by.
    pub scan_filter_uuid: Option<Uuid>,
}

impl TransportState {
    pub fn new() -> Self {
        Self {
            peripheral: None,
            characteristics: HashMap::new(),
            scan_filter_uuid: None,
        }
    }
}

impl Default for TransportState {
    fn default() -> Self {
        Self::new()
    }
}
