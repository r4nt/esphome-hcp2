#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

mod garage_physics;
mod drive_protocol;

pub use garage_physics::GaragePhysics;
pub use drive_protocol::DriveProtocol;
pub use drive_protocol::DriveProtocolState;

use hcp2_common::hal::HcpHal;

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
static mut PROTOCOL: Option<DriveProtocol> = None;

#[no_mangle]
pub extern "C" fn hcp_tester_init() {
    unsafe {
        PHYSICS = Some(GaragePhysics::new());
        PROTOCOL = Some(DriveProtocol::new());
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

struct TesterHalWrapper<'a> {
    inner: &'a TesterHalC,
}

impl<'a> HcpHal for TesterHalWrapper<'a> {
    fn uart_read(&mut self, buf: &mut [u8]) -> usize {
        let res = (self.inner.read_uart)(self.inner.ctx, buf.as_mut_ptr(), buf.len());
        if res < 0 { 0 } else { res as usize }
    }

    fn uart_write(&mut self, buf: &[u8]) -> usize {
        let res = (self.inner.write_uart)(self.inner.ctx, buf.as_ptr(), buf.len());
        if res < 0 { 0 } else { res as usize }
    }

    fn set_tx_enable(&mut self, enable: bool) {
        (self.inner.set_tx_enable)(self.inner.ctx, enable);
    }

    fn now_ms(&self) -> u32 {
        (self.inner.now_ms)()
    }

    fn sleep_ms(&mut self, _ms: u32) {
    }

    fn log(&mut self, message: &str) {
        (self.inner.log)(self.inner.ctx, message.as_ptr(), message.len());
    }
}

#[no_mangle]
pub extern "C" fn hcp_tester_poll(hal: *const TesterHalC, state: *mut TesterState) {
    unsafe {
        let physics = core::ptr::addr_of_mut!(PHYSICS).as_mut().unwrap().as_mut().unwrap();
        let protocol = core::ptr::addr_of_mut!(PROTOCOL).as_mut().unwrap().as_mut().unwrap();
        let hal_c = &*hal;
        let mut hal_wrapper = TesterHalWrapper { inner: hal_c };

        // Run Physics
        physics.tick();

        // Check for incoming response
        let mut rx_buf = [0u8; 64];
        let len = hal_wrapper.uart_read(&mut rx_buf);
        if len > 0 {
            protocol.handle_response(&rx_buf[..len], physics);
        }

        // Run Protocol (Generate Request)
        protocol.poll(&mut hal_wrapper, physics);

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
