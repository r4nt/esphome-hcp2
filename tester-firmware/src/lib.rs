#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

mod garage_physics;
mod drive_fsm;

pub use garage_physics::GaragePhysics;
pub use drive_fsm::DriveFsm;
pub use drive_fsm::DriveFsmState;

// FFI Interface
#[repr(C)]
pub struct TesterState {
    pub current_pos: f32,
    pub target_pos: f32,
    pub light_on: bool,
    pub vent_on: bool,
    pub last_action: u8,
}

static mut PHYSICS: Option<GaragePhysics> = None;
static mut FSM: Option<DriveFsm> = None;

#[no_mangle]
pub extern "C" fn hcp_tester_init() {
    unsafe {
        PHYSICS = Some(GaragePhysics::new());
        FSM = Some(DriveFsm::new());
    }
}

#[repr(C)]
pub struct TesterHalC {
    pub ctx: *mut core::ffi::c_void,
    pub read_uart: extern "C" fn(*mut core::ffi::c_void, *mut u8, usize) -> i32,
    pub write_uart: extern "C" fn(*mut core::ffi::c_void, *const u8, usize) -> i32,
    pub set_tx_enable: extern "C" fn(*mut core::ffi::c_void, bool),
    pub now_ms: extern "C" fn() -> u32,
    pub log: extern "C" fn(*mut core::ffi::c_void, *const u8, usize),
}

impl TesterHalC {
    pub fn log(&self, message: &str) {
        (self.log)(self.ctx, message.as_ptr(), message.len());
    }
}

#[no_mangle]
pub extern "C" fn hcp_tester_poll(hal: *const TesterHalC, state: *mut TesterState) {
    unsafe {
        let physics = core::ptr::addr_of_mut!(PHYSICS).as_mut().unwrap().as_mut().unwrap();
        let fsm = core::ptr::addr_of_mut!(FSM).as_mut().unwrap().as_mut().unwrap();
        let hal = &*hal;

        let now = (hal.now_ms)();
        
        // Run Physics
        physics.tick();

        // Check for incoming response
        let mut rx_buf = [0u8; 64];
        let len = (hal.read_uart)(hal.ctx, rx_buf.as_mut_ptr(), rx_buf.len());
        if len > 0 {
            fsm.handle_response(&rx_buf[..len as usize], physics);
        }

        // Run FSM (Generate Request)
        let mut tx_buf = [0u8; 64];
        let old_state = fsm.state;
        let tx_len = fsm.poll(physics, now, &mut tx_buf);
        
        if fsm.state != old_state {
            let s = match fsm.state {
                drive_fsm::DriveFsmState::Scan => "Transition to State: Scan",
                drive_fsm::DriveFsmState::Broadcast => "Transition to State: Broadcast",
                drive_fsm::DriveFsmState::Poll => "Transition to State: Poll",
            };
            hal.log(s);
        }

        if tx_len > 0 {
            if old_state == drive_fsm::DriveFsmState::Scan {
                hal.log("Scanning...");
            } else if old_state == drive_fsm::DriveFsmState::Poll {
                hal.log("Polling...");
            } else if old_state == drive_fsm::DriveFsmState::Broadcast {
                hal.log("Broadcasting status...");
            }

            (hal.set_tx_enable)(hal.ctx, true);
            (hal.write_uart)(hal.ctx, tx_buf.as_ptr(), tx_len);
            (hal.set_tx_enable)(hal.ctx, false);
            
            if old_state == drive_fsm::DriveFsmState::Scan || old_state == drive_fsm::DriveFsmState::Poll {
                hal.log("Waiting for response...");
            }
        }

        // Update State Struct for C++
        if !state.is_null() {
            (*state).current_pos = physics.current_position;
            (*state).target_pos = physics.target_position;
            (*state).light_on = physics.light_on;
            (*state).vent_on = physics.vent_on;
        }
    }
}

#[no_mangle]
pub extern "C" fn hcp_tester_set_control(target_pos: f32, toggle_light: bool) {
    unsafe {
        if let Some(physics) = core::ptr::addr_of_mut!(PHYSICS).as_mut().unwrap().as_mut() {
            physics.target_position = target_pos;
            if toggle_light {
                physics.light_on = !physics.light_on;
            }
        }
    }
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
