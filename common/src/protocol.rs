use crate::shared::*;

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

    pub fn process_write_9d31(&mut self, regs: &[u16], shared: &mut SharedData) {
        if regs.len() < 9 {
            return;
        }
        // Reg 1: Target Position (High) | Current Position (Low)
        shared.target_position = (regs[1] >> 8) as u8;
        shared.current_position = (regs[1] & 0xFF) as u8;
        
        // Reg 2: State (High)
        shared.current_state = (regs[2] >> 8) as u8;
        
        // Reg 6: Light Status (Bit 0x10)
        shared.light_on = (regs[6] & 0x10) != 0;
    }

    pub fn process_write_9c41(&mut self, regs: &[u16]) {
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
                resp[0] = ((self.counter as u16) << 8) | 0x0004;
                resp[1] = ((self.command_code as u16) << 8) | 0x0000;
            }
            5 => {
                resp[0] = (self.counter as u16) << 8;
                resp[1] = ((self.command_code as u16) << 8) | 0x0005;
                resp[2] = 0x0430;
                resp[3] = 0x10FF;
                resp[4] = 0xA845;
            }
            8 => {
                resp[0] = (self.counter as u16) << 8;
                resp[1] = ((self.command_code as u16) << 8) | 0x0001;
                
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

    #[test]
    fn test_process_write_9d31() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();
        
        // Example from PROTOCOL.md: 0x1635 (target 0x16, current 0x35), state 0x01 (Opening), light bit 0x10
        let regs = [0x0000, 0x1635, 0x0100, 0x0000, 0x0000, 0x0000, 0x0010, 0x0000, 0x0000];
        proto.process_write_9d31(&regs, &mut shared);
        
        assert_eq!(shared.target_position, 0x16);
        assert_eq!(shared.current_position, 0x35);
        assert_eq!(shared.current_state, 0x01);
        assert!(shared.light_on);
    }

    #[test]
    fn test_action_sequence() {
        let mut proto = Hcp2Protocol::new();
        let mut shared = SharedData::default();
        
        shared.command_request = CMD_CLOSE;
        
        // At t=0, should be "Pressing"
        let (r2, r3) = proto.get_action_registers(&shared, 0);
        assert_eq!(r2, 0x0220);
        assert_eq!(r3, 0x0000);
        
        // At t=499, should still be "Pressing"
        let (r2, r3) = proto.get_action_registers(&shared, 499);
        assert_eq!(r2, 0x0220);
        
        // At t=501, should be "Release"
        let (r2, r3) = proto.get_action_registers(&shared, 501);
        assert_eq!(r2, 0x0120);
        assert_eq!(r3, 0x0000);
    }
}
