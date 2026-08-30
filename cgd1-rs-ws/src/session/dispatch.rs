use cgd1_rs::MacAddress;
use futures::SinkExt;
use serde::Serialize;
use tracing::debug;
use tracing::warn;

use crate::command;
use crate::error::ServerError;
use crate::protocol::SubscribeEventsResponse;
use crate::protocol::WsCommand;
use crate::session::WsSession;
use crate::session::event::convert_event;

/// Serialize a result to `serde_json::Value`, mapping errors to `ServerError`.
fn to_value(result: impl Serialize) -> Result<serde_json::Value, ServerError> {
    Ok(serde_json::to_value(result)?)
}

impl WsSession {
    /// Dispatch a WebSocket command to the appropriate handler.
    pub(crate) async fn dispatch_command(&self, command: WsCommand) -> Result<serde_json::Value, ServerError> {
        let value = match command {
            WsCommand::Scan { duration_secs } => to_value(command::scan(&self.state, duration_secs).await?)?,
            WsCommand::Connect { address } => to_value(command::connect(&self.state, address).await?)?,
            WsCommand::Disconnect { address } => to_value(command::disconnect(&self.state, address).await?)?,
            WsCommand::SyncTime { address } => to_value(command::sync_time(&self.state, address).await?)?,
            WsCommand::ReadAlarms { address } => to_value(command::alarm_list(&self.state, address).await?)?,
            WsCommand::SetAlarm {
                address,
                slot,
                time,
                repeat_mask,
                enabled,
                snooze,
            } => to_value(command::alarm_set(&self.state, address, slot, time, repeat_mask, enabled, snooze).await?)?,
            WsCommand::DeleteAlarm { address, slot } => to_value(command::alarm_delete(&self.state, address, slot).await?)?,
            WsCommand::ReadSettings { address } => to_value(command::read_settings(&self.state, address).await?)?,
            WsCommand::WriteSettings { address, settings } => to_value(command::write_settings(&self.state, address, settings).await?)?,
            WsCommand::SetBrightness { address, value } => to_value(command::set_brightness(&self.state, address, value).await?)?,
            WsCommand::PreviewRingtone { address, volume } => to_value(command::preview_ringtone(&self.state, address, volume).await?)?,
            WsCommand::ReadFirmware { address } => to_value(command::read_firmware(&self.state, address).await?)?,
            WsCommand::ReadBattery { address } => to_value(command::read_battery(&self.state, address).await?)?,
            WsCommand::SubscribeEvents { address } => to_value(self.subscribe_events(address).await?)?,
        };
        Ok(value)
    }

    /// Subscribe to device events and forward them to the WebSocket client.
    async fn subscribe_events(&self, address: MacAddress) -> Result<SubscribeEventsResponse, ServerError> {
        let device = self.state.device(&address).await?;
        let mut receiver = device.subscribe();

        let sender_clone = self.sender.clone();
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        if let Some(ws_event) = convert_event(&event) {
                            let json = match serde_json::to_string(&ws_event) {
                                Ok(j) => j,
                                Err(e) => {
                                    warn!("Failed to serialize event: {e}");
                                    continue;
                                }
                            };
                            let mut tx = sender_clone.lock().await;
                            if tx.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                                debug!("WebSocket client disconnected, stopping event forwarding");
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Skipped {n} events");
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(SubscribeEventsResponse { address, subscribed: true })
    }
}
