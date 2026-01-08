#[repr(C)]
#[derive(Debug, Default)]
pub struct SharedData {
    /// 0 = Free, 1 = HP core writing, 2 = LP core writing
    pub owner_flag: u8,

    /// HP -> LP: Requested action (0 = None, 1 = Open, 2 = Close, 3 = Stop, 4 = HalfOpen, 5 = Vent, 6 = Light)
    pub command_request: u8,
    /// HP -> LP: Target position (0-200)
    pub target_position: u8,

    /// LP -> HP: Current state of the drive
    pub current_state: u8,
    /// LP -> HP: Current position (0-200)
    pub current_position: u8,
    /// LP -> HP: Light status
    pub light_on: bool,
    /// LP -> HP: Timestamp of last valid packet
    pub last_update_ts: u32,
    /// LP -> HP: Error code
    pub error_code: u8,
}

pub const OWNER_FREE: u8 = 0;
pub const OWNER_HP: u8 = 1;
pub const OWNER_LP: u8 = 2;

pub const CMD_NONE: u8 = 0;
pub const CMD_OPEN: u8 = 1;
pub const CMD_CLOSE: u8 = 2;
pub const CMD_STOP: u8 = 3;
pub const CMD_HALF_OPEN: u8 = 4;
pub const CMD_VENT: u8 = 5;
pub const CMD_TOGGLE_LIGHT: u8 = 6;
