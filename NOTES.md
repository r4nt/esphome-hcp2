# Development Notes: ESPHome HCP2 Bridge on ESP32-C6

This document captures the technical learnings, architectural decisions, and workarounds discovered during the implementation of the HCP2 protocol bridge for ESP32-C6.

## 1. Architecture Overview

The goal was to offload the timing-sensitive HCP2 (Modbus RTU) polling loop to the ESP32-C6's **Low Power (LP) Core** (RISC-V), freeing up the High Performance (HP) core for WiFi and Home Assistant logic.

### Components
*   **`common` (Rust):** A `no_std` crate containing the core protocol state machine, register maps, and shared memory layout. It is designed to be agnostic of the running core.
*   **`lp-firmware` (Rust):** The firmware running on the LP core. It uses `esp-lp-hal` to drive the LP UART and GPIOs.
*   **`hp-firmware` (Rust):** A wrapper static library that exposes the `common` protocol logic to C++ via `extern "C"` functions, allowing the HP core to run the same logic if desired.
*   **`hcp_bridge` (C++):** An ESPHome custom component. It manages the LP core lifecycle, handles Inter-Processor Communication (IPC), and exposes entities (Cover, Switch) to Home Assistant.

### Inter-Processor Communication (IPC)
*   **Shared Memory:** A `SharedData` struct located at fixed address `0x50002000` in LP RAM.
*   **Synchronization:** A manual spinlock/ownership flag (`owner_flag`: `0=FREE`, `1=HP`, `2=LP`) controls access to avoid race conditions.
*   **Data Flow:**
    *   HP writes commands (Open, Close) to shared memory.
    *   LP reads commands, executes Modbus transactions, and writes status (Position, State) back to shared memory.

## 2. Protocol Implementation Details

*   **Modbus RTU:** The protocol is standard Modbus RTU over Serial (57600 baud, 8E1).
*   **Function 0x17:** The HCP2 protocol relies heavily on Function Code `0x17` (Read/Write Multiple Registers).
    *   *Issue:* The `rmodbus` crate (v0.9) does not natively support `0x17` in its server parsing logic.
    *   *Solution:* Implemented a manual fallback parser for `0x17` frames while using `rmodbus` for standard validation (CRC, Address) and other function codes.
*   **CRC:** Uses standard Modbus CRC16, but implementation required careful attention to endianness (High byte, Low byte).
*   **Command Logic:** Buttons are simulated by sending a "Pressing" value for 500ms, followed by a "Release" value. This logic is encapsulated in `common`.

## 3. ESP32-C6 Hardware Constraints

### LP Core UART
*   **Fixed Pinout:** The LP UART peripherals are physically hard-wired to specific pads in the LP IO domain.
    *   **TX:** GPIO 5
    *   **RX:** GPIO 4
    *   *Constraint:* These **cannot** be remapped via the GPIO Matrix (unlike HP UARTs). The hardware connection simply doesn't exist for other pins.
*   **RS-485 Support:** The LP UART is a "lite" peripheral and lacks automatic RS-485 hardware flow control (RTS).
    *   *Workaround:* Implemented manual software direction control.
    *   **Pin:** GPIO 2 used for DE/RE.

### LP Core Jumper Guide (Seeed Xiao RS485)
Since the Seeed Xiao RS485 shield hard-wires the RS485 transceiver to GPIO 22 (TX) and GPIO 23 (RX), the LP core cannot use them. To use LP mode, you must:
1.  **Intercept the signals:** Do not allow Xiao pins D6 (GPIO 22) and D7 (GPIO 23) to connect to the shield.
2.  **Jumper TX:** Connect Xiao pin **D5 (GPIO 5)** to the shield's **TX/D6** input.
3.  **Jumper RX:** Connect Xiao pin **D4 (GPIO 4)** to the shield's **RX/D7** output.
4.  **DE/RE:** Pin **D0 (GPIO 2)** is already shared by both HP and LP, so no modification is needed for flow control.

## 4. Build System & Integration

Integrating Rust builds into the ESPHome/PlatformIO workflow required several custom steps.

### Binary Embedding (LP Mode)
*   *Issue:* Standard PlatformIO `board_build.embed_files` and CMake `target_add_binary_data` failed to reliably link the binary blob due to symbol visibility and path issues in the ESPHome generated project structure.
*   *Solution:* **C Header Generation**.
    *   Created `build_hooks.py` which runs `cargo build`.
    *   It converts the resulting `hcp2-lp.bin` into a C header file (`hcp2_lp_bin.h`) containing a `const uint8_t[]` array.
    *   The C++ code simply `#include`s this header, bypassing all linker complexity.

### Static Linking (HP Mode)
*   To allow the HP core to run the Rust logic, we needed to link `common` as a static library.
*   *Config:* Added `crate-type = ["staticlib"]` to a wrapper crate (`hp-firmware`) and set `panic = "abort"` in `Cargo.toml` to avoid linking `libunwind` (which `no_std` lacks).
*   *Linking:* Used `target_link_libraries` in `CMakeLists.txt` to link `libhcp2_hp_lib.a` to the component.

### Automation
*   `components/hcp_bridge/build_hooks.py` hooks into the ESPHome build process.
*   It automatically installs the `riscv32imac-unknown-none-elf` target and `cargo-binutils` if missing.
*   It builds both the LP binary and the HP static library before the C++ build starts.

## 5. ESPHome Configuration Tips

*   **Secrets:** Used `!secret` for WiFi credentials to keep the main config shareable.
*   **Typed Schemas:** Used `cv.typed_schema` for the Switch platform to cleanly handle different switch classes (`HCPLightSwitch` vs `HCPVentSwitch`) within a single platform definition.
*   **Logging:** The bridge component is excessively chatty in `DEBUG` mode if the loop isn't throttled; production use should stick to `INFO` or handle logging carefully.

## 6. Testing Strategy

*   **Unit Tests:** `cargo test -p hcp2-common` validates the protocol logic (parsing, state machine, CRC) on the host machine without hardware.
*   **Hardware Loopback:** Connecting TX to RX on the chip verified the UART driver (though logic requires a master/slave pairing).
*   **PC Simulation:** A Python script using `pyserial` can act as the "Drive", sending polls to the ESP32 to verify end-to-end functionality before connecting to the real garage door.

## 7. Hybrid HP/LP Mode

The project now supports running the protocol logic on either the Low Power (LP) core or the High Performance (HP) core, configurable via YAML.

### Architecture
*   **LP Mode:** Loads `hcp2-lp.bin` into the LP core. The HP core communicates via shared memory. This provides the best timing guarantees and lowest power consumption.
*   **HP Mode:** Spawns a high-priority FreeRTOS task on the main core. It uses the standard ESP-IDF UART driver but calls the *exact same* Rust protocol logic (`common` crate) via C bindings.

### Implementation
*   **`hp-firmware` crate:** A wrapper that builds `common` as a `staticlib` (`libhcp2_hp_lib.a`) and exposes `extern "C"` functions (`hcp2_protocol_init`, `dispatch`).
*   **Linking:** The static library is cross-compiled for `riscv32imac-unknown-none-elf` by `build_hooks.py` and linked into the ESPHome component using `target_link_libraries` in `CMakeLists.txt`.
*   **Configuration:**
    ```yaml
    hcp_bridge:
      id: hcp_hub
      core: hp  # or 'lp' (default)
      tx_pin: 5
      rx_pin: 4
      flow_control_pin: 2
    ```
