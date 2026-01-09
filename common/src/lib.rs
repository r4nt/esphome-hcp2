#![no_std]

mod registers;
pub mod shared;
pub mod protocol;
pub mod hal;
pub mod driver;

pub use shared::SharedData;
pub use protocol::Hcp2Protocol;
pub use hal::HcpHal;
pub use driver::Hcp2Driver;

