use crate::registers::*;
use crate::shared::*;

#[derive(Debug, PartialEq)]
pub enum RegisterType {
    StatusUpdate,
    SyncCounter,
    Poll,
    Unknown,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DispatchError {
    FrameTooShort,
    InvalidAddress,
    InvalidFunction,
    CrcMismatch,
    ParsingError,
}

pub struct Hcp2Protocol {
    counter: u8,
    command_code: u8,
    last_action: u8,
    action_start_ts: u32,
}

impl Hcp2Protocol {
    pub fn new() -> Self {
        Self {
            counter: 0,
            command_code: 0,
            last_action: CMD_NONE,
            action_start_ts: 0,
        }
    }

    pub(crate) fn identify_request(&self, address: u16) -> RegisterType {
        match address {
            ADDR_STATUS_UPDATE => RegisterType::StatusUpdate,
            ADDR_SYNC_COUNTER => RegisterType::SyncCounter,
            ADDR_POLL => RegisterType::Poll,
            _ => RegisterType::Unknown,
        }
    }

    pub fn handle_status_update(&mut self, regs: &[u16], shared: &mut SharedData) {
        if regs.len() < 9 {
            return;
        }
        // Reg 1: Target Position (High) | Current Position (Low)
        shared.write_target_pos((regs[1] >> 8) as u8);
        shared.write_current_pos((regs[1] & 0xFF) as u8);
        
        // Reg 2: State (High)
        let state_val = (regs[2] >> 8) as u8;
        shared.write_state(DriveState::from(state_val) as u8);
        
        // Reg 6: Light Status (Bit 0x10)
        shared.write_light((regs[6] & 0x10) != 0);
    }

    pub fn handle_sync_counter(&mut self, regs: &[u16]) {
        if regs.len() < 1 {
            return;
        }
        self.counter = (regs[0] >> 8) as u8;
        self.command_code = (regs[0] & 0xFF) as u8;
    }

    pub fn prepare_poll_response(&mut self, quantity: u16, shared: &SharedData, millis: u32) -> [u16; 8] {
        let mut resp = [0u16; 8];
        match quantity {
            2 => {
                resp[0] = ((self.counter as u16) << 8) | 0x04;
                resp[1] = ((self.command_code as u16) << 8) | 0x00;
            }
            5 => {
                resp[0] = (self.counter as u16) << 8;
                resp[1] = ((self.command_code as u16) << 8) | 0x05;
                resp[2] = 0x0430;
                resp[3] = 0x10FF;
                resp[4] = 0xA845;
            }
            8 => {
                resp[0] = (self.counter as u16) << 8;
                resp[1] = ((self.command_code as u16) << 8) | 0x01;
                
                let (reg2, reg3) = self.get_action_registers(shared, millis);
                resp[2] = reg2;
                resp[3] = reg3;
            }
            _ => {}
        }
        resp
    }

    /// Dispatches a raw byte frame to the appropriate handler.
    pub fn dispatch_frame(&mut self, frame: &[u8], out_buffer: &mut [u8], shared: &mut SharedData, millis: u32) -> Result<usize, DispatchError> {
        if frame.len() < 4 {
            return Err(DispatchError::FrameTooShort);
        }

        let address = frame[0];
        let func = frame[1];

        if address != ADDRESS_HCP && address != ADDRESS_BROADCAST {
            return Err(DispatchError::InvalidAddress);
        }

        if func != FUNC_WRITE_MULTIPLE_REGISTERS && func != FUNC_READ_WRITE_MULTIPLE_REGISTERS {
            return Err(DispatchError::InvalidFunction);
        }

        // Validate CRC (Modbus RTU: last 2 bytes are LSB, MSB)
        let len = frame.len();
        let received_crc = (frame[len - 2] as u16) | ((frame[len - 1] as u16) << 8);
        let computed_crc = crc16(&frame[..len - 2]);
        if received_crc != computed_crc {
            return Err(DispatchError::CrcMismatch);
        }

        match func {
            FUNC_WRITE_MULTIPLE_REGISTERS => {
                if frame.len() < 9 { return Err(DispatchError::ParsingError); }
                let start_addr = ((frame[2] as u16) << 8) | (frame[3] as u16);
                let qty = ((frame[4] as u16) << 8) | (frame[5] as u16);
                let byte_count = frame[6] as usize;
                if frame.len() < 9 + byte_count { return Err(DispatchError::ParsingError); }
                
                let mut regs = [0u16; 16];
                for i in 0..(byte_count / 2).min(16) {
                    regs[i] = ((frame[7 + i * 2] as u16) << 8) | (frame[8 + i * 2] as u16);
                }

                match self.identify_request(start_addr) {
                    RegisterType::StatusUpdate => self.handle_status_update(&regs[..qty as usize], shared),
                    RegisterType::SyncCounter => self.handle_sync_counter(&regs[..qty as usize]),
                    _ => {}
                }
                Ok(0) 
            }
            FUNC_READ_WRITE_MULTIPLE_REGISTERS => {
                if frame.len() < 13 { return Err(DispatchError::ParsingError); }
                let rd_addr = ((frame[2] as u16) << 8) | (frame[3] as u16);
                let rd_qty = ((frame[4] as u16) << 8) | (frame[5] as u16);
                let wr_addr = ((frame[6] as u16) << 8) | (frame[7] as u16);
                let wr_qty = ((frame[8] as u16) << 8) | (frame[9] as u16);
                let byte_count = frame[10] as usize;
                
                if frame.len() < 13 + byte_count { return Err(DispatchError::ParsingError); }

                let mut wr_regs = [0u16; 16];
                for i in 0..(byte_count / 2).min(16) {
                    wr_regs[i] = ((frame[11 + i * 2] as u16) << 8) | (frame[12 + i * 2] as u16);
                }

                if self.identify_request(wr_addr) == RegisterType::SyncCounter {
                    self.handle_sync_counter(&wr_regs[..wr_qty as usize]);
                }

                if self.identify_request(rd_addr) == RegisterType::Poll {
                    let resp_regs = self.prepare_poll_response(rd_qty, shared, millis);
                    let resp_byte_count = (rd_qty * 2) as u8;
                    
                    if out_buffer.len() < 5 + resp_byte_count as usize { return Err(DispatchError::ParsingError); }

                    out_buffer[0] = ADDRESS_HCP;
                    out_buffer[1] = FUNC_READ_WRITE_MULTIPLE_REGISTERS;
                    out_buffer[2] = resp_byte_count;
                    for i in 0..rd_qty as usize {
                        out_buffer[3 + i * 2] = (resp_regs[i] >> 8) as u8;
                        out_buffer[4 + i * 2] = (resp_regs[i] & 0xFF) as u8;
                    }
                    let out_len = 3 + resp_byte_count as usize;
                    let crc = crc16(&out_buffer[..out_len]);
                    out_buffer[out_len] = (crc & 0xFF) as u8;
                    out_buffer[out_len + 1] = (crc >> 8) as u8;
                    return Ok(out_len + 2);
                }
                Ok(0)
            }
            _ => Err(DispatchError::InvalidFunction)
        }
    }

    fn get_action_registers(&mut self, shared: &SharedData, millis: u32) -> (u16, u16) {
        let action = shared.read_command();
        if action == CMD_NONE {
            self.last_action = CMD_NONE;
            return (0, 0);
        }

        if self.last_action != action {
            self.last_action = action;
            self.action_start_ts = millis;
        }

        let is_pressing = millis.wrapping_sub(self.action_start_ts) < 500;

        match action {
            CMD_OPEN => if is_pressing { (0x0210, 0x0000) } else { (0x0110, 0x0000) },
            CMD_CLOSE => if is_pressing { (0x0220, 0x0000) } else { (0x0120, 0x0000) },
            CMD_STOP => if is_pressing { (0x0240, 0x0000) } else { (0x0140, 0x0000) },
            CMD_HALF_OPEN => if is_pressing { (0x0200, 0x0400) } else { (0x0100, 0x0400) },
            CMD_VENT => if is_pressing { (0x0200, 0x4000) } else { (0x0100, 0x4000) },
            CMD_TOGGLE_LIGHT => if is_pressing { (0x0100, 0x0200) } else { (0x0800, 0x0200) },
            _ => (0, 0),
        }
    }
}

fn crc16(data: &[u8]) -> u16 {
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
    crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::DriveState;

    #[test]
    fn test_handle_status_update_parsing() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();

        // Verify constants usage via validation
        // NOTE: validate_frame is now internal/inline. Testing dispatch directly.
        
        let mut buf = [0u8; 32];
        
        // Invalid Address
        let invalid_addr = [0x99, FUNC_WRITE_MULTIPLE_REGISTERS, 0x00, 0x00];
        assert_eq!(proto.dispatch_frame(&invalid_addr, &mut buf, &mut shared, 0), Err(DispatchError::InvalidAddress));

        // Invalid Func
        let invalid_func = [ADDRESS_HCP, 0x88, 0x00, 0x00];
        assert_eq!(proto.dispatch_frame(&invalid_func, &mut buf, &mut shared, 0), Err(DispatchError::InvalidFunction));

        // Example from PROTOCOL.md: 0x1635 (target 0x16, current 0x35), state 0x01 (Opening), light bit 0x10
        let regs1 = [0x0000, 0x1635, 0x0100, 0x0000, 0x0000, 0x0000, 0x0010, 0x0000, 0x0000];
        proto.handle_status_update(&regs1, &mut shared);
        assert_eq!(shared.target_position, 0x16);
        assert_eq!(shared.current_position, 0x35);
        assert_eq!(shared.current_state, DriveState::Opening as u8);
        assert!(shared.light_on);

        // Test Case 2: Closed, Light Off
        let regs2 = [0x0000, 0x0000, 0x4000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000];
        proto.handle_status_update(&regs2, &mut shared);
        assert_eq!(shared.current_state, DriveState::Closed as u8);
        assert!(!shared.light_on);
    }

    #[test]
    fn test_poll_responses() {
        let mut proto = Hcp2Protocol::new();
        let shared = SharedData::default();

        // Simulate sync counter update from drive
        let sync_regs = [0x1234];
        proto.handle_sync_counter(&sync_regs);

        // Test Length 2 (Idle Poll)
        let resp2 = proto.prepare_poll_response(2, &shared, 0);
        assert_eq!(resp2[0], 0x1204); 
        assert_eq!(resp2[1], 0x3400); 

        // Test Length 5 (Bus Scan)
        let resp5 = proto.prepare_poll_response(5, &shared, 0);
        assert_eq!(resp5[0], 0x1200); 
        assert_eq!(resp5[1], 0x3405); 
        assert_eq!(resp5[2], 0x0430);
        assert_eq!(resp5[3], 0x10FF);
        assert_eq!(resp5[4], 0xA845);

        // Test Length 8 (Command Action - None)
        let resp8 = proto.prepare_poll_response(8, &shared, 0);
        assert_eq!(resp8[0], 0x1200); 
        assert_eq!(resp8[1], 0x3401); 
        assert_eq!(resp8[2], 0x0000); 
        assert_eq!(resp8[3], 0x0000); 
    }

    #[test]
    fn test_all_commands_press_release_logic() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();

        let check_action = |proto: &mut Hcp2Protocol, shared: &SharedData, time: u32, expected_r2, expected_r3| {
            let resp = proto.prepare_poll_response(8, shared, time);
            assert_eq!(resp[2], expected_r2, "Reg2 mismatch at time {}", time);
            assert_eq!(resp[3], expected_r3, "Reg3 mismatch at time {}", time);
        };

        let test_cases = [
            (CMD_OPEN,         0x0210, 0x0000, 0x0110, 0x0000),
            (CMD_CLOSE,        0x0220, 0x0000, 0x0120, 0x0000),
            (CMD_STOP,         0x0240, 0x0000, 0x0140, 0x0000),
            (CMD_HALF_OPEN,    0x0200, 0x0400, 0x0100, 0x0400),
            (CMD_VENT,         0x0200, 0x4000, 0x0100, 0x4000),
            (CMD_TOGGLE_LIGHT, 0x0100, 0x0200, 0x0800, 0x0200),
        ];

        let mut current_time = 1000;

        for (cmd, press_r2, press_r3, rel_r2, rel_r3) in test_cases.iter() {
            shared.command_request = *cmd;
            check_action(&mut proto, &shared, current_time, *press_r2, *press_r3);
            check_action(&mut proto, &shared, current_time + 499, *press_r2, *press_r3);
            check_action(&mut proto, &shared, current_time + 500, *rel_r2, *rel_r3);
            current_time += 2000;
        }

        shared.command_request = CMD_NONE;
        check_action(&mut proto, &shared, current_time, 0x0000, 0x0000);
    }

    #[test]
    fn test_dispatch_frame_busscan() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();
        // Byte sequence for a valid bus scan request
        let request = [
            0x02, 0x17, 0x9C, 0xB9, 0x00, 0x05, 0x9C, 0x41, 0x00, 0x03, 
            0x06, 0x00, 0x02, 0x00, 0x00, 0x01, 0x02, 0xF8, 0x35
        ];
        let mut response = [0u8; 32];
        
        // Note: We use 0 as current time for the test
        let result = proto.dispatch_frame(&request, &mut response, &mut shared, 0);
        
        assert!(result.is_ok(), "Bus scan should be parsed successfully");
        let len = result.unwrap();
        assert!(len > 0, "Response should be generated");
        
        // Check identifying features of the response (Bus Scan Response)
        assert_eq!(response[0], ADDRESS_HCP);
        assert_eq!(response[1], FUNC_READ_WRITE_MULTIPLE_REGISTERS);
        assert_eq!(response[2], 10, "Response should contain 5 registers (10 bytes)");
        
        // Device ID constants check (0x0430, 0x10FF, 0xA845)
        // Offset 3 is start of data
        assert_eq!(response[7], 0x04);
        assert_eq!(response[8], 0x30);
        assert_eq!(response[9], 0x10);
        assert_eq!(response[10], 0xFF);
        assert_eq!(response[11], 0xA8);
        assert_eq!(response[12], 0x45);
    }

    #[test]
    fn test_crc() {
        let request = [0x02, 0x17, 0x9C, 0xB9, 0x00, 0x05, 0x9C, 0x41, 0x00, 0x03, 0x06, 0x00, 0x02, 0x00, 0x00, 0x01, 0x02];
        let crc = crc16(&request);
        assert_eq!(crc, 0x35F8); 
    }
}
