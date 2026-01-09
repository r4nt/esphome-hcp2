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
    
    // Explicit padding to align `last_update_ts` to 4 bytes (offsets: 0..5 filled, need 2 bytes to reach 8? No. 0,1,2,3,4,5 = 6 bytes used. Need 2 bytes pad.)
    pub _pad1: [u8; 2],

    /// LP -> HP: Timestamp of last valid packet
    pub last_update_ts: u32,
    /// LP -> HP: Error code
    pub error_code: u8,
    
    pub _pad2: [u8; 3], // Pad to 16 bytes total
}

impl SharedData {
    pub fn read_owner(&self) -> u8 { unsafe { core::ptr::read_volatile(&self.owner_flag) } }
    pub fn write_owner(&mut self, val: u8) { unsafe { core::ptr::write_volatile(&mut self.owner_flag, val) } }

    pub fn read_command(&self) -> u8 { unsafe { core::ptr::read_volatile(&self.command_request) } }
    pub fn write_command(&mut self, val: u8) { unsafe { core::ptr::write_volatile(&mut self.command_request, val) } }

    pub fn read_target_pos(&self) -> u8 { unsafe { core::ptr::read_volatile(&self.target_position) } }
    pub fn write_target_pos(&mut self, val: u8) { unsafe { core::ptr::write_volatile(&mut self.target_position, val) } }

    pub fn read_state(&self) -> u8 { unsafe { core::ptr::read_volatile(&self.current_state) } }
    pub fn write_state(&mut self, val: u8) { unsafe { core::ptr::write_volatile(&mut self.current_state, val) } }

    pub fn read_current_pos(&self) -> u8 { unsafe { core::ptr::read_volatile(&self.current_position) } }
    pub fn write_current_pos(&mut self, val: u8) { unsafe { core::ptr::write_volatile(&mut self.current_position, val) } }

    pub fn read_light(&self) -> bool { unsafe { core::ptr::read_volatile(&self.light_on) } }
    pub fn write_light(&mut self, val: bool) { unsafe { core::ptr::write_volatile(&mut self.light_on, val) } }

    pub fn read_ts(&self) -> u32 { unsafe { core::ptr::read_volatile(&self.last_update_ts) } }
    pub fn write_ts(&mut self, val: u32) { unsafe { core::ptr::write_volatile(&mut self.last_update_ts, val) } }
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
