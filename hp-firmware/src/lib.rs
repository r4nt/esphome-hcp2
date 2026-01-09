#![no_std]

use hcp2_common::{Hcp2Driver, HcpHal, SharedData};
use panic_halt as _;

// C-compatible struct for function pointers
#[repr(C)]
pub struct HcpHalC {
    pub ctx: *mut core::ffi::c_void,
    pub read_uart: extern "C" fn(*mut core::ffi::c_void, *mut u8, usize) -> i32,
    pub write_uart: extern "C" fn(*mut core::ffi::c_void, *const u8, usize) -> i32,
    pub set_tx_enable: extern "C" fn(*mut core::ffi::c_void, bool),
    pub now_ms: extern "C" fn() -> u32,
    pub sleep_ms: extern "C" fn(u32),
}

struct HcpHalWrapper<'a> {
    inner: &'a HcpHalC,
}

impl<'a> HcpHal for HcpHalWrapper<'a> {
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

    fn sleep_ms(&mut self, ms: u32) {
        (self.inner.sleep_ms)(ms);
    }
}

/// Main entry point for the HP core task.
/// 
/// # Safety
/// Caller must ensure `hal_ptr` and `shared_ptr` are valid and live for the duration of the call.
/// This function does NOT return.
#[no_mangle]
pub unsafe extern "C" fn hcp_run_hp_loop(hal_ptr: *const HcpHalC, shared_ptr: *mut SharedData) -> ! {
    let hal_c = &*hal_ptr;
    let mut hal = HcpHalWrapper { inner: hal_c };
    let shared = &mut *shared_ptr;
    
    let mut driver = Hcp2Driver::new();

    loop {
        driver.poll(&mut hal, shared);
        hal.sleep_ms(1);
    }
}