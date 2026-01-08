#![no_std]
#![no_main]

use esp_lp_hal::{
    delay::Delay,
    prelude::*,
    uart::LpUart,
};
use hcp2_common::{SharedData, Hcp2Protocol, shared::{OWNER_LP, OWNER_FREE}};
use panic_halt as _;
use embedded_hal::delay::DelayNs;

// Shared memory at fixed address for HP core to find
const SHARED_MEM_ADDR: usize = 0x50002000;

#[entry]
fn main(mut uart: LpUart) -> ! {
    let mut protocol = Hcp2Protocol::new();
    let shared_data: &mut SharedData = unsafe { &mut *(SHARED_MEM_ADDR as *mut SharedData) };

    let mut rx_buf = [0u8; 128];
    let mut rx_idx = 0;
    let mut last_rx_ms = 0u32;
    let mut current_ms = 0u32;

    loop {
        // Simple polling for UART
        if let Ok(byte) = uart.read_byte() {
            if rx_idx < rx_buf.len() {
                rx_buf[rx_idx] = byte;
                rx_idx += 1;
                last_rx_ms = current_ms;
            }
        }

        // Frame timeout (Modbus RTU: 3.5 chars, but 10ms is safe at 57600)
        if rx_idx > 0 && current_ms.wrapping_sub(last_rx_ms) > 10 {
            let mut tx_buf = [0u8; 128];
            
            // Try to acquire "lock" (very simple ownership flag)
            if shared_data.owner_flag != 1 { // 1 = HP writing
                shared_data.owner_flag = OWNER_LP;
                
                let tx_len = protocol.dispatch_frame(&rx_buf[..rx_idx], &mut tx_buf, shared_data, current_ms);
                
                if tx_len > 0 {
                    let _ = uart.write_bytes(&tx_buf[..tx_len]);
                }
                
                shared_data.owner_flag = OWNER_FREE;
            }

            rx_idx = 0;
        }

        Delay.delay_ms(1);
        current_ms = current_ms.wrapping_add(1);
    }
}
