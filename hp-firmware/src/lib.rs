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
    pub log: extern "C" fn(*mut core::ffi::c_void, *const u8, usize),
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

    fn log(&mut self, message: &str) {
        (self.inner.log)(self.inner.ctx, message.as_ptr(), message.len());
    }
}

static mut DRIVER: Option<Hcp2Driver> = None;

#[no_mangle]
pub unsafe extern "C" fn hcp_hp_init() {
    DRIVER = Some(Hcp2Driver::new());
}

#[no_mangle]
pub unsafe extern "C" fn hcp_hp_poll(hal_ptr: *const HcpHalC, shared_ptr: *mut SharedData) {
    if let Some(driver) = &mut DRIVER {
        let hal_c = &*hal_ptr;
        let mut hal = HcpHalWrapper { inner: hal_c };
        let shared = &mut *shared_ptr;
        driver.poll(&mut hal, shared);
    }
}
