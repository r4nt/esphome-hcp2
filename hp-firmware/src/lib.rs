#![no_std]

use panic_halt as _;

// Re-export C functions from common so they appear in this staticlib
pub use hcp2_common::hcp2_protocol_init;
pub use hcp2_common::hcp2_protocol_dispatch;
pub use hcp2_common::hcp2_protocol_size;
