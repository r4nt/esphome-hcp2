use crate::hal::HcpHal;
use crate::protocol::{Hcp2Protocol, DispatchError};
use crate::shared::{SharedData, OWNER_LP, OWNER_FREE};

pub struct Hcp2Driver {
    protocol: Hcp2Protocol,
    rx_buf: [u8; 128],
    rx_idx: usize,
    last_rx_ms: u32,
    tx_buf: [u8; 128],
}

impl Hcp2Driver {
    pub fn new() -> Self {
        Self {
            protocol: Hcp2Protocol::new(),
            rx_buf: [0u8; 128],
            rx_idx: 0,
            last_rx_ms: 0,
            tx_buf: [0u8; 128],
        }
    }

    /// Runs a single iteration of the main loop.
    /// This should be called repeatedly.
    pub fn poll<H: HcpHal>(&mut self, hal: &mut H, shared: &mut SharedData) {
        let current_ms = hal.now_ms();

        // 1. Read available bytes
        let read_count = hal.uart_read(&mut self.rx_buf[self.rx_idx..]);
        if read_count > 0 {
            self.rx_idx += read_count;
            self.last_rx_ms = current_ms;
        }

        // 2. Check for frame timeout (Modbus RTU: 3.5 chars, ~2-10ms)
        // We trigger processing if we have data AND enough time passed since last byte
        if self.rx_idx > 0 && current_ms.wrapping_sub(self.last_rx_ms) > 10 {
            
            // Try to acquire lock (Non-blocking)
            // Note: On HP core, we might be the only writer if we own the task, 
            // but we respect the flag in case LP core is also active (unlikely but safe).
            if shared.read_owner() != 1 { 
                shared.write_owner(OWNER_LP); // Using OWNER_LP (2) as "Active Driver" ID

                match self.protocol.dispatch_frame(
                    &self.rx_buf[..self.rx_idx], 
                    &mut self.tx_buf, 
                    shared, 
                    current_ms
                ) {
                    Ok(tx_len) => {
                        if tx_len > 0 {
                            // Switch to TX
                            hal.set_tx_enable(true);
                            hal.uart_write(&self.tx_buf[..tx_len]);
                            
                            // Wait for transmission to finish is handled by HAL or caller?
                            // Usually blocking write is simplest.
                            // We add a small safety delay to ensure UART FIFO is empty before dropping DE
                            hal.sleep_ms(2); 
                            
                            // Switch back to RX
                            hal.set_tx_enable(false);
                        }
                    },
                    Err(e) => {
                        match e {
                            DispatchError::CrcMismatch => hal.log("Error: CRC Mismatch"),
                            DispatchError::InvalidAddress => hal.log("Debug: Discarding message - Invalid Address"),
                            DispatchError::FrameTooShort => hal.log("Error: Frame Too Short"),
                            DispatchError::InvalidFunction => hal.log("Debug: Invalid Function Code"),
                            DispatchError::ParsingError => hal.log("Error: Frame Parsing Failed"),
                        }
                    }
                }

                shared.write_owner(OWNER_FREE);
            }

            // Reset buffer after processing attempt
            self.rx_idx = 0;
        }
    }
}
