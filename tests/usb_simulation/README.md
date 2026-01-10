# HCP2 USB Simulation Test

This test environment allows you to verify the HCP2 protocol logic and the ESPHome component without needing a physical Hoermann garage door drive or an RS-485 adapter.

## How it Works

The test uses the **High Performance (HP) Core** mode of the HCP2 bridge. Instead of using dedicated GPIOs and an RS-485 transceiver, it redirects the protocol communication to the ESP32's primary **USB-Serial (UART0)** port.

1.  **ESP32 Side:** Runs the `hcp_bridge` component. It listens on the USB serial port for Modbus frames and responds according to the HCP2 protocol logic.
2.  **Host (PC) Side:** A Python script simulates the Drive (the Garage Door motor). It sends periodic discovery, status, and polling packets over the same USB cable used for flashing.

## Setup & Run

1.  **Flash the ESP:** Use `test_hp_usb.yaml` to flash your ESP.
2.  **Run Simulator:**
    ```bash
    python3 simulate_drive.py /dev/ttyACM0
    ```
3.  **Observe:**
    *   **[Bus Scan]:** Drive sends a discovery request. The ESP should respond with identification bytes (`0430 10FF A845`).
    *   **[Status Update]:** Drive sends a broadcast packet setting the door to "Opening" at 25%. You should see the state update in the ESPHome web dashboard/logs.
    *   **[Command Poll]:** Drive checks for pending actions. If you click "Open" or "Light" in the ESPHome UI, the script will log: `>>> ESP REQUESTED ACTION: 0x...`.

### In ESPHome Logs:
Open a separate terminal to view the logs via the JTAG port:
```bash
.venv/bin/esphome logs tests/usb_simulation/test_hp_usb.yaml --device-port /dev/ttyACM1
```
*(Note: ESP32-C6 often presents two serial ports; one is the JTAG/CDC for logs, the other is the Hardware UART for the protocol).*

## Troubleshooting
*   **Timeout - No Response:** Ensure the `tx_pin` and `rx_pin` in the YAML match your board's UART0 pins.
*   **Permission Denied:** Ensure your user has permissions to access the serial port (`sudo usermod -a -G dialout $USER` on Linux).
