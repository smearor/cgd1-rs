use cgd1_rs::MacAddress;

use serde::Serialize;

/// Response: subscribed to events.
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeEventsResponse {
    /// Device MAC address.
    pub address: MacAddress,
    /// Whether the subscription is active.
    pub subscribed: bool,
}
