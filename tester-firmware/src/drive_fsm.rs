use common::registers::*;
use crate::garage_physics::GaragePhysics;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DriveFsmState {
    Scan,
    Broadcast,
    Poll,
}

pub struct DriveFsm {
    pub state: DriveFsmState,
    pub last_poll_ms: u32,
    pub sync_counter: u8,
    pub command_code: u8,
}

impl DriveFsm {
    pub fn new() -> Self {
        Self {
            state: DriveFsmState::Scan,
            last_poll_ms: 0,
            sync_counter: 0,
            command_code: 0,
        }
    }

    pub fn poll(&mut self, physics: &mut GaragePhysics, now_ms: u32, out_buf: &mut [u8]) -> usize {
        match self.state {
            DriveFsmState::Scan => {
                if now_ms - self.last_poll_ms < 1000 {
                    return 0;
                }
                self.last_poll_ms = now_ms;
                // Send Scan Request: Read 5 registers from ADDR_POLL
                // WRITE: ADDR_SYNC_COUNTER, 3 registers
                self.build_read_write_frame(out_buf, ADDRESS_HCP, 
                    ADDR_POLL, 5, 
                    ADDR_SYNC_COUNTER, 3, 
                    &[0, 0, 0])
            },
            DriveFsmState::Broadcast => {
                self.last_poll_ms = now_ms;
                self.state = DriveFsmState::Poll;

                // Send Status Broadcast (0x10) to 0x00
                // Reg 1: Target | Current
                let reg1 = ((physics.target_position as u16) << 8) | (physics.current_position as u16);
                // Reg 2: State
                let reg2 = (physics.get_drive_state() as u16) << 8;
                // Reg 6: Light (Bit 0x10)
                let reg6 = if physics.light_on { 0x0010 } else { 0x0000 };

                let regs = [
                    0x0000, reg1, reg2, 0x0000,
                    0x0000, 0x0000, reg6, 0x0000, 0x0000
                ];
                
                self.build_write_frame(out_buf, ADDRESS_BROADCAST, ADDR_STATUS_UPDATE, &regs)
            },
            DriveFsmState::Poll => {
                if now_ms - self.last_poll_ms < 100 {
                    return 0;
                }
                self.last_poll_ms = now_ms;
                // self.state = DriveFsmState::Broadcast; // Cycle back

                self.sync_counter = self.sync_counter.wrapping_add(1);
                let sync_val = ((self.sync_counter as u16) << 8) | (self.command_code as u16);

                // Send Poll Request: Read 8 registers
                self.build_read_write_frame(out_buf, ADDRESS_HCP, 
                    ADDR_POLL, 8, 
                    ADDR_SYNC_COUNTER, 1, 
                    &[sync_val])
            }
        }
    }

    pub fn handle_response(&mut self, frame: &[u8], physics: &mut GaragePhysics) {
        // Simple validation
        if frame.len() < 4 { return; }
        
        // Parse response based on state
        match self.state {
            DriveFsmState::Scan => {
                // If we got a valid response, assume it's the device
                // Could check for 0x0430, 0x10FF, 0xA845
                self.state = DriveFsmState::Broadcast;
            },
            DriveFsmState::Poll => {
                // Expecting function 0x17 response
                if frame[1] != FUNC_READ_WRITE_MULTIPLE_REGISTERS { return; }
                let byte_count = frame[2] as usize;
                if frame.len() < 3 + byte_count { return; }

                // Parse action registers (Index 2 and 3 -> Bytes 7-10)
                if byte_count >= 8 {
                    let r2 = ((frame[7] as u16) << 8) | (frame[8] as u16);
                    let r3 = ((frame[9] as u16) << 8) | (frame[10] as u16);
                    
                    let action = self.decode_action(r2, r3);
                    if action != DriveAction::None {
                        physics.handle_action(action);
                    }
                }
                self.state = DriveFsmState::Broadcast;
            },
            _ => {}
        }
    }

    fn decode_action(&self, r2: u16, r3: u16) -> DriveAction {
        // Logic from PROTOCOL.md
        // Note: The protocol sends Pressing then Release. We should trigger on Pressing.
        match (r2, r3) {
            (0x0210, 0x0000) => DriveAction::Open,
            (0x0220, 0x0000) => DriveAction::Close,
            (0x0240, 0x0000) => DriveAction::Stop,
            (0x0200, 0x0400) => DriveAction::HalfOpen,
            (0x0200, 0x4000) => DriveAction::Vent,
            (0x0100, 0x0200) => DriveAction::ToggleLight,
            _ => DriveAction::None,
        }
    }

    fn build_write_frame(&self, buf: &mut [u8], addr: u8, start: u16, regs: &[u16]) -> usize {
        buf[0] = addr;
        buf[1] = FUNC_WRITE_MULTIPLE_REGISTERS;
        buf[2] = (start >> 8) as u8;
        buf[3] = (start & 0xFF) as u8;
        let qty = regs.len() as u16;
        buf[4] = (qty >> 8) as u8;
        buf[5] = (qty & 0xFF) as u8;
        buf[6] = (qty * 2) as u8;
        
        for (i, &reg) in regs.iter().enumerate() {
            buf[7 + i*2] = (reg >> 8) as u8;
            buf[8 + i*2] = (reg & 0xFF) as u8;
        }
        
        let len = 7 + (qty * 2) as usize;
        let crc = self.crc16(&buf[..len]);
        buf[len] = (crc & 0xFF) as u8;
        buf[len+1] = (crc >> 8) as u8;
        len + 2
    }

    fn build_read_write_frame(&self, buf: &mut [u8], addr: u8, rd_start: u16, rd_qty: u16, wr_start: u16, wr_qty: u16, wr_regs: &[u16]) -> usize {
        buf[0] = addr;
        buf[1] = FUNC_READ_WRITE_MULTIPLE_REGISTERS;
        buf[2] = (rd_start >> 8) as u8;
        buf[3] = (rd_start & 0xFF) as u8;
        buf[4] = (rd_qty >> 8) as u8;
        buf[5] = (rd_qty & 0xFF) as u8;
        buf[6] = (wr_start >> 8) as u8;
        buf[7] = (wr_start & 0xFF) as u8;
        buf[8] = (wr_qty >> 8) as u8;
        buf[9] = (wr_qty & 0xFF) as u8;
        buf[10] = (wr_qty * 2) as u8;

        for (i, &reg) in wr_regs.iter().enumerate() {
            buf[11 + i*2] = (reg >> 8) as u8;
            buf[12 + i*2] = (reg & 0xFF) as u8;
        }

        let len = 11 + (wr_qty * 2) as usize;
        let crc = self.crc16(&buf[..len]);
        buf[len] = (crc & 0xFF) as u8;
        buf[len+1] = (crc >> 8) as u8;
        len + 2
    }

    fn crc16(&self, data: &[u8]) -> u16 {
        let mut crc = 0xFFFFu16;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if (crc & 1) != 0 {
                    crc >>= 1;
                    crc ^= 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        (crc << 8) | (crc >> 8)
    }
}
