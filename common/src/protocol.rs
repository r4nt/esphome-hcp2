use crate::registers::*;
use crate::shared::*;

#[derive(Debug, PartialEq)]
pub enum RegisterType {
    StatusUpdate,
    SyncCounter,
    Poll,
    Unknown,
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

    /// Validates if a frame is addressed to us or broadcast, and if the function code is supported.
    pub fn validate_frame(&self, address: u8, function: u8) -> bool {
        let valid_addr = address == ADDRESS_HCP || address == ADDRESS_BROADCAST;
        let valid_func = function == FUNC_WRITE_MULTIPLE_REGISTERS || function == FUNC_READ_WRITE_MULTIPLE_REGISTERS;
        valid_addr && valid_func
    }

    pub fn identify_request(&self, address: u16) -> RegisterType {
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
        shared.target_position = (regs[1] >> 8) as u8;
        shared.current_position = (regs[1] & 0xFF) as u8;
        
        // Reg 2: State (High)
        let state_val = (regs[2] >> 8) as u8;
        shared.current_state = DriveState::from(state_val) as u8;
        
        // Reg 6: Light Status (Bit 0x10)
        shared.light_on = (regs[6] & 0x10) != 0;
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

    fn get_action_registers(&mut self, shared: &SharedData, millis: u32) -> (u16, u16) {
        let action = shared.command_request;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::DriveState;

    #[test]
    fn test_handle_status_update_parsing() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();

        // Verify constants usage via validation
        assert!(proto.validate_frame(ADDRESS_HCP, FUNC_WRITE_MULTIPLE_REGISTERS));
        assert!(proto.validate_frame(ADDRESS_BROADCAST, FUNC_READ_WRITE_MULTIPLE_REGISTERS));
        assert!(!proto.validate_frame(0x99, FUNC_WRITE_MULTIPLE_REGISTERS)); // Invalid addr
        assert!(!proto.validate_frame(ADDRESS_HCP, 0x88)); // Invalid func

        // Example from PROTOCOL.md: 0x1635 (target 0x16, current 0x35), state 0x01 (Opening), light bit 0x10

        
        // Test Case 1: Opening, Light On
        // Reg 1: Target=0x16, Current=0x35
        // Reg 2: State=0x01 (Opening)
        // Reg 6: Light=0x10 (On)
        let regs1 = [0x0000, 0x1635, 0x0100, 0x0000, 0x0000, 0x0000, 0x0010, 0x0000, 0x0000];
        proto.handle_status_update(&regs1, &mut shared);
        assert_eq!(shared.target_position, 0x16);
        assert_eq!(shared.current_position, 0x35);
        assert_eq!(shared.current_state, DriveState::Opening as u8);
        assert!(shared.light_on);

        // Test Case 2: Closed, Light Off
        // Reg 2: State=0x40 (Closed)
        // Reg 6: Light=0x00 (Off)
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
        // Counter=0x12, Command=0x34
        let sync_regs = [0x1234];
        proto.handle_sync_counter(&sync_regs);

        // Test Length 2 (Idle Poll)
        let resp2 = proto.prepare_poll_response(2, &shared, 0);
        assert_eq!(resp2[0], 0x1204); // Counter | 0x04
        assert_eq!(resp2[1], 0x3400); // Command | 0x00

        // Test Length 5 (Bus Scan)
        let resp5 = proto.prepare_poll_response(5, &shared, 0);
        assert_eq!(resp5[0], 0x1200); // Counter | 0x00
        assert_eq!(resp5[1], 0x3405); // Command | 0x05
        assert_eq!(resp5[2], 0x0430);
        assert_eq!(resp5[3], 0x10FF);
        assert_eq!(resp5[4], 0xA845);

        // Test Length 8 (Command Action - None)
        let resp8 = proto.prepare_poll_response(8, &shared, 0);
        assert_eq!(resp8[0], 0x1200); // Counter | 0x00
        assert_eq!(resp8[1], 0x3401); // Command | 0x01
        assert_eq!(resp8[2], 0x0000); // Action Reg 1
        assert_eq!(resp8[3], 0x0000); // Action Reg 2
    }

    #[test]
    fn test_all_commands_press_release_logic() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();

        // Helper to check action registers at specific times
        let check_action = |proto: &mut Hcp2Protocol, shared: &SharedData, time: u32, expected_r2, expected_r3| {
            let resp = proto.prepare_poll_response(8, shared, time);
            assert_eq!(resp[2], expected_r2, "Reg2 mismatch at time {}", time);
            assert_eq!(resp[3], expected_r3, "Reg3 mismatch at time {}", time);
        };

        // Table of commands and their expected Pressing (0-500ms) and Release (>500ms) values
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
            
            // Time = Start (0ms elapsed) -> Expect Pressing
            check_action(&mut proto, &shared, current_time, *press_r2, *press_r3);
            
            // Time = Start + 499ms -> Expect Pressing
            check_action(&mut proto, &shared, current_time + 499, *press_r2, *press_r3);
            
            // Time = Start + 500ms -> Expect Release
            check_action(&mut proto, &shared, current_time + 500, *rel_r2, *rel_r3);

            // Move time forward for next test
            current_time += 2000;
        }

        // Test Reset to CMD_NONE
        shared.command_request = CMD_NONE;
        check_action(&mut proto, &shared, current_time, 0x0000, 0x0000);
    }

    #[test]
    fn test_identify_request() {
        let proto = Hcp2Protocol::new();
        assert_eq!(proto.identify_request(0x9D31), RegisterType::StatusUpdate);
        assert_eq!(proto.identify_request(0x9C41), RegisterType::SyncCounter);
        assert_eq!(proto.identify_request(0x9CB9), RegisterType::Poll);
        assert_eq!(proto.identify_request(0xFFFF), RegisterType::Unknown);
    }
}
