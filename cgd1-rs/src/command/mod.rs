//! Command frame types for the CGD1 BLE protocol.
//!
//! This module defines the frame encoding/decoding types used by
//! [`crate::device::ClockDevice`]. Full command implementations are
//! added in Phase 2+.

mod ack;
mod ack_status;
mod command_id;
mod commands;
mod frame;

pub use ack::Ack;
pub use ack_status::AckStatus;
pub use command_id::CommandId;
pub use commands::Command;
pub use frame::CommandFrame;
