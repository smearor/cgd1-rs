//! Command frame types for the CGD1 BLE protocol.
//!
//! This module defines the frame encoding/decoding types used by
//! [`crate::device::ClockDevice`]. Full command implementations are
//! added in Phase 2+.

mod ack;
mod ack_status;
mod alarm;
mod command_id;
mod commands;
mod frame;

pub use ack::Ack;
pub use ack_status::AckStatus;
pub use alarm::AlarmEntry;
pub use alarm::AlarmSlot;
pub use alarm::AlarmSlotIndex;
pub use alarm::DayMask;
pub use command_id::CommandId;
pub use commands::Command;
pub use frame::CommandFrame;
