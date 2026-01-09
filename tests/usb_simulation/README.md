# HCP2 USB Simulation Test

This test environment allows you to verify the HCP2 protocol logic and the ESPHome component without needing a physical Hoermann garage door drive or an RS-485 adapter.

## How it Works

The test uses the **High Performance (HP) Core** mode of the HCP2 bridge. Instead of using dedicated GPIOs and an RS-485 transceiver, it redirects the protocol communication to the ESP32's primary **USB-Serial (UART0)** port.

1.  **ESP32 Side:** Runs the `hcp_bridge` component. It listens on the USB serial port for Modbus frames and responds according to the HCP2 protocol logic.
2.  **Host (PC) Side:** A Python script simulates the Master (the Garage Door Drive). It sends periodic discovery, status, and polling packets over the same USB cable used for flashing.

## Setup Instructions

### 1. Requirements
*   ESP32-C6 (or ESP32-S3) development board.
*   Python 3 with `pyserial` installed:
    ```bash
    pip install pyserial
    ```

### 2. Flash the Test Firmware
Run the following command to compile and flash the test configuration:
```bash
.venv/bin/esphome run tests/usb_simulation/test_hp_usb.yaml
```
*Note: This configuration disables standard UART logging on the TX/RX pins to prevent interference with the protocol. Logs are redirected to `USB_SERIAL_JTAG`.*

### 3. Run the Simulator
Identify your serial port (e.g., `/dev/ttyACM0` on Linux or `COM3` on Windows) and run the simulator:
```bash
python3 tests/usb_simulation/simulate_drive.py /dev/ttyACM0
```

## Expected Behavior

### On the Terminal:
You should see a loop of exchanges:
*   **[Bus Scan]:** Master sends a discovery request. The ESP should respond with identification bytes (`0430 10FF A845`).
*   **[Status Update]:** Master sends a broadcast packet setting the door to "Opening" at 25%. You should see the state update in the ESPHome web dashboard/logs.
*   **[Command Poll]:** Master checks for pending actions. If you click "Open" or "Light" in the ESPHome UI, the script will log: `>>> ESP REQUESTED ACTION: 0x...`.

### In ESPHome Logs:
Open a separate terminal to view the logs via the JTAG port:
```bash
.venv/bin/esphome logs tests/usb_simulation/test_hp_usb.yaml --device-port /dev/ttyACM1
```
*(Note: ESP32-C6 often presents two serial ports; one is the JTAG/CDC for logs, the other is the Hardware UART for the protocol).*

## Troubleshooting
*   **Timeout - No Response:** Ensure the `tx_pin` and `rx_pin` in the YAML match your board's UART0 pins.
*   **Permission Denied:** Ensure your user has permissions to access the serial port (`sudo usermod -a -G dialout $USER` on Linux).
