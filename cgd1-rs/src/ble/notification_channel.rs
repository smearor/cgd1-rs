use std::pin::Pin;
use std::sync::Arc;

use btleplug::api::ValueNotification;
use futures::Stream;
use tokio::sync::Mutex;
use tokio::sync::watch;

/// Pinned, boxed notification stream from btleplug.
///
/// This is the stream returned by `Peripheral::notifications()`, yielding
/// `ValueNotification` items as GATT notifications arrive from the device.
/// It is pinned because the underlying stream may be self-referential.
pub(crate) type NotificationStream = Pin<Box<dyn Stream<Item = ValueNotification> + Send>>;

/// Per-device notification channel: GATT value stream plus a disconnect signal.
///
/// Each connected device has one `NotificationChannel` stored in
/// `BtleplugTransport::notification_streams`. The `stream` field yields
/// incoming GATT notifications, while `disconnect_rx` is a watch channel
/// receiver that is signalled when the device disconnects (either explicitly
/// via `disconnect()` or by the event monitor detecting a `DeviceDisconnected`
/// CentralEvent).
///
/// `next_notification()` uses `tokio::select!` to race between the stream
/// and the disconnect signal, so that a silent BLE disconnect (where the
/// notification stream never ends) is still detected promptly.
pub(crate) struct NotificationChannel {
    /// The GATT notification stream from btleplug.
    ///
    /// Yields `ValueNotification` items as the device sends data on subscribed
    /// characteristics (AuthNotify, DataNotify, SensorNotify).
    pub stream: NotificationStream,

    /// Watch channel receiver for the disconnect signal.
    ///
    /// Holds `false` while connected. When the sender transmits `true`,
    /// `next_notification()` returns `None`, causing the notification task
    /// to begin reconnect logic.
    pub disconnect_rx: watch::Receiver<bool>,
}

/// Shared, thread-safe handle to a [`NotificationChannel`].
///
/// Stored in `BtleplugTransport::notification_streams` keyed by MAC address.
/// The `Arc<Mutex<...>>` allows the notification task to lock the channel
/// while polling the stream, without holding the `notification_streams` map
/// lock.
pub(crate) type SharedNotificationChannel = Arc<Mutex<NotificationChannel>>;
