/// The Hardware Abstraction Layer trait required by the HCP2 driver.
/// This allows the same logic to run on LP core (using esp-lp-hal) 
/// and HP core (using C function pointers).
pub trait HcpHal {
    /// Read bytes from UART into buffer. Returns number of bytes read.
    fn uart_read(&mut self, buf: &mut [u8]) -> usize;

    /// Write bytes to UART. Returns number of bytes written.
    fn uart_write(&mut self, buf: &[u8]) -> usize;

    /// Set the RS-485 Direction Pin (High = TX, Low = RX).
    fn set_tx_enable(&mut self, enable: bool);

    /// Get current timestamp in milliseconds.
    fn now_ms(&self) -> u32;

    /// Sleep for a specified duration in milliseconds.
    fn sleep_ms(&mut self, ms: u32);
}
