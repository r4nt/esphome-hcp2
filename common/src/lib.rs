#![no_std]

mod registers;
pub mod shared;
pub mod protocol;

pub use shared::SharedData;
pub use protocol::Hcp2Protocol;
