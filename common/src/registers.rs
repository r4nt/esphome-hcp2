pub const ADDRESS_BROADCAST: u8 = 0x00;
pub const ADDRESS_HCP: u8 = 0x02;

pub const FUNC_WRITE_MULTIPLE_REGISTERS: u8 = 0x10;
pub const FUNC_READ_WRITE_MULTIPLE_REGISTERS: u8 = 0x17;

pub const ADDR_STATUS_UPDATE: u16 = 0x9D31;
pub const ADDR_SYNC_COUNTER: u16 = 0x9C41;
pub const ADDR_POLL: u16 = 0x9CB9;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveState {
    Stopped = 0x00,
    Opening = 0x01,
    Closing = 0x02,
    MoveHalf = 0x05,
    MoveVenting = 0x09,
    VentReached = 0x0A,
    Open = 0x20,
    Closed = 0x40,
    HalfOpenReached = 0x80,
}

impl From<u8> for DriveState {
    fn from(val: u8) -> Self {
        match val {
            0x01 => DriveState::Opening,
            0x02 => DriveState::Closing,
            0x05 => DriveState::MoveHalf,
            0x09 => DriveState::MoveVenting,
            0x0A => DriveState::VentReached,
            0x20 => DriveState::Open,
            0x40 => DriveState::Closed,
            0x80 => DriveState::HalfOpenReached,
            _ => DriveState::Stopped,
        }
    }
}
