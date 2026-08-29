use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::debug;

use crate::AdvertisementData;
use crate::BleTransport;
use crate::DiscoveredDevice;
use crate::error::Result;
use crate::types::MacAddress;
use uuid::Uuid;

/// FDCD service-data UUID for Qingping advertisements.
const FDCD_UUID: Uuid = Uuid::from_fields(0x0000fdcd, 0x0000, 0x1000, &[0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb]);

/// Scans for Qingping CGD1 devices via BLE advertisements.
pub struct ClockScanner {
    transport: Arc<dyn BleTransport>,
    advertisement_sender: broadcast::Sender<AdvertisementData>,
}

impl ClockScanner {
    /// Create a new scanner with the given transport.
    pub fn new(transport: Arc<dyn BleTransport>) -> Self {
        let (advertisement_sender, _) = broadcast::channel(64);
        Self {
            transport,
            advertisement_sender,
        }
    }

    /// Start passive scanning for CGD1 advertisements.
    ///
    /// Returns a receiver that yields parsed advertisement data. The scanner
    /// continues scanning until `stop_passive` is called or the transport
    /// stops yielding advertisements.
    pub async fn scan_passive(&self) -> Result<broadcast::Receiver<AdvertisementData>> {
        self.transport.start_scan(FDCD_UUID).await?;
        let sender = self.advertisement_sender.clone();
        let transport = self.transport.clone();

        tokio::spawn(async move {
            while let Some(data) = transport.next_advertisement().await {
                debug!(
                    mac = %data.mac,
                    temp = data.temperature.value(),
                    humidity = data.humidity.value(),
                    battery = data.battery.value(),
                    "received advertisement"
                );
                let _ = sender.send(data);
            }
        });

        Ok(self.advertisement_sender.subscribe())
    }

    /// Stop passive scanning.
    pub async fn stop_passive(&self) -> Result<()> {
        self.transport.stop_scan().await
    }

    /// Start active scanning and return discovered devices.
    ///
    /// Scans for `duration` seconds, collecting unique MAC addresses from
    /// passive advertisements. Devices that broadcast CGD1 service data are
    /// included in the result.
    pub async fn scan_active(&self, duration: Duration) -> Result<Vec<DiscoveredDevice>> {
        self.transport.start_scan(FDCD_UUID).await?;

        let mut devices: HashMap<MacAddress, DiscoveredDevice> = HashMap::new();
        let deadline = duration;

        loop {
            let result = timeout(deadline, self.transport.next_advertisement()).await;
            match result {
                Ok(Some(data)) => {
                    devices.insert(
                        data.mac,
                        DiscoveredDevice {
                            address: data.mac,
                            advertisement: Some(data),
                            rssi: None,
                        },
                    );
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }

        self.transport.stop_scan().await?;
        Ok(devices.into_values().collect())
    }
}
