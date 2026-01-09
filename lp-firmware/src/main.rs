#![no_std]
#![no_main]

use esp_lp_hal::{
    delay::Delay,
    prelude::*,
};
use hcp2_common::{SharedData, Hcp2Driver, HcpHal};
use panic_halt as _;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

// Shared memory at fixed address for HP core to find
const SHARED_MEM_ADDR: usize = 0x50003000;

struct LpHal {
    uart: esp_lp_hal::uart::LpUart,
    dir_pin: esp_lp_hal::gpio::Output<2>,
    current_ms: u32,
}

impl HcpHal for LpHal {
    fn uart_read(&mut self, buf: &mut [u8]) -> usize {
        let mut count = 0;
        // Simple non-blocking read until empty or full
        while count < buf.len() {
             if let Ok(byte) = self.uart.read_byte() {
                 buf[count] = byte;
                 count += 1;
             } else {
                 break;
             }
        }
        count
    }

    fn uart_write(&mut self, buf: &[u8]) -> usize {
        let _ = self.uart.write_bytes(buf);
        buf.len()
    }

    fn set_tx_enable(&mut self, enable: bool) {
        if enable {
            let _ = self.dir_pin.set_high();
        } else {
            let _ = self.dir_pin.set_low();
        }
    }

    fn now_ms(&self) -> u32 {
        self.current_ms
    }

    fn sleep_ms(&mut self, ms: u32) {
        Delay.delay_ms(ms);
        self.current_ms = self.current_ms.wrapping_add(ms);
    }
}

#[entry]
fn main() -> ! {
    let uart = unsafe { esp_lp_hal::uart::conjure() };
    let mut dir_pin = unsafe { esp_lp_hal::gpio::conjure_output::<2>().unwrap() };
    let _ = dir_pin.set_low();

    let shared_data: &mut SharedData = unsafe { &mut *(SHARED_MEM_ADDR as *mut SharedData) };

    let mut hal = LpHal {
        uart,
        dir_pin,
        current_ms: 0,
    };

    let mut driver = Hcp2Driver::new();

    loop {
        driver.poll(&mut hal, shared_data);
        // Short sleep to yield/save power, matching the logic of sleep_ms updating time
        hal.sleep_ms(1);
    }
}