# esphome-hcp2
Implementation of an esphome component for hcp2 using Rust for the communication.

## Compiling the LP Firmware

The communication with the Hörmann drive is offloaded to the ESP32-C6's Low Power (LP) core to ensure timing precision.

### Prerequisites
1.  **Rust Toolchain:** Install from [rustup.rs](https://rustup.rs/).
2.  **RISC-V Target:** Install the target for the LP core:
    ```bash
    rustup target add riscv32imac-unknown-none-elf
    ```
3.  **LLVM Tools:** Required for binary conversion:
    ```bash
    rustup component add llvm-tools-preview
    cargo install cargo-binutils
    ```

### Build Instructions
Run the following command from the project root:
```bash
cd lp-firmware && cargo build --release
```

After building, convert the ELF binary to a raw binary file and place it in the bridge component directory:
```bash
rust-objcopy -O binary target/riscv32imac-unknown-none-elf/release/hcp2-lp components/hcp_bridge/hcp2-lp.bin
```

**Note:** You **must** build in `--release` mode. The debug build is too large to fit into the limited 16KB LP RAM.

Building the firmware also automatically generates `components/hcp_bridge/shared_data.h`, which is required for the ESPHome C++ build.

## Wiring (ESP32-C6)

The LP core uses fixed pins for the LP UART:
*   **GPIO 4:** RX (Connect to HCP Bus TX)
*   **GPIO 5:** TX (Connect to HCP Bus RX)

*Note: Use an RS-485 transceiver (like MAX485) or a suitable level shifter as the HCP bus uses different voltage levels/logic than the ESP32.*

## Automated Build Integration

The provided `example.yaml` includes a Python script that automates the Rust compilation and binary conversion during the ESPHome build process. This ensures `hcp2-lp.bin` and `shared_data.h` are always up to date.

To use this, ensure your environment has the Rust prerequisites listed above.

### Running Tests
To verify the protocol logic on your host machine:
```bash
cargo test -p hcp2-common
```
