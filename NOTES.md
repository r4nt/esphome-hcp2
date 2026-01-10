# Development Notes: ESPHome HCP2 Bridge on ESP32-C6

This document captures the technical learnings, architectural decisions, and workarounds discovered during the implementation of the HCP2 protocol bridge for ESP32-C6.

## 1. Architecture Overview

The goal was to offload the timing-sensitive HCP2 (Modbus RTU) polling loop to the ESP32-C6's **Low Power (LP) Core** (RISC-V), freeing up the High Performance (HP) core for WiFi and Home Assistant logic.

### Components
*   **`common` (Rust):** A `no_std` crate containing the core protocol state machine, register maps, and shared memory layout.
*   **Unified Driver Architecture:** Both LP and HP cores use the exact same `Hcp2Driver` logic from the `common` crate, abstracted via the `HcpHal` trait.
*   **`lp-firmware` (Rust):** The firmware running on the LP core. It implements `HcpHal` using `esp-lp-hal` to drive the LP UART and GPIOs.
*   **`hp-firmware` (Rust):** A static library (`staticlib`) that implements `HcpHal` using function pointers to C proxy functions. This allows the HP core to drive the hardware while Rust owns the protocol state and timing.
*   **`hcp_bridge` (C++):** An ESPHome custom component. It manages the LP core lifecycle, handles Inter-Processor Communication (IPC), and exposes entities (Cover, Switch) to Home Assistant.

### Inter-Processor Communication (IPC)
*   **Shared Memory:** A `SharedData` struct located at fixed address `0x50003000` in LP RAM.
    *   *Note:* Address was moved from `0x50002000` to `0x50003000` to avoid overlap with the LP firmware binary/stack.
*   **Synchronization:** A manual ownership flag (`owner_flag`: `0=FREE`, `1=HP`, `2=LP`) controls access.
*   **Data Flow:**
    *   HP writes commands (Open, Close) to shared memory.
    *   LP/HP Driver reads commands, executes Modbus transactions, and writes status back to shared memory.

## 2. Protocol Implementation Details

*   **Modbus RTU:** Standard Modbus RTU (57600 baud, 8E1).
*   **Custom Parsing:** The protocol relies heavily on Function Code `0x17` (Read/Write Multiple Registers). Since common Modbus libraries (like `rmodbus`) often lack native `0x17` support in slave mode, a 100% manual frame parser was implemented in `protocol.rs`.
*   **CRC:** Uses standard Modbus CRC16 (Polynomial `0xA001`).
*   **Command Logic:** Buttons are simulated by sending a "Pressing" value for 500ms, followed by a "Release" value. This timing is managed by the unified `Hcp2Driver`.

## 3. ESP32-C6 Hardware Constraints

### LP Core UART
*   **Fixed Pinout:** The LP UART peripherals are physically hard-wired to specific pads.
    *   **TX:** GPIO 5
    *   **RX:** GPIO 4
    *   *Constraint:* These **cannot** be remapped via the GPIO Matrix.
*   **RS-485 Support:** The LP UART lacks automatic hardware direction control. Manual software control of the DE/RE pin (GPIO 2) was implemented in the HAL.

## 4. Build System & Integration

### Binary Embedding (LP Mode)
*   *Issue:* ESPHome's build process periodically synchronizes/prunes the build directory, deleting any manually placed headers or binaries. This makes traditional `-I` include paths unreliable for generated files.
*   *Solution:* **Global RawStatement Injection**.
    *   `build_hooks.py` compiles the Rust firmware and reads the resulting `.bin`.
    *   It uses `cg.add_global(RawStatement(...))` to inject the binary data as a C-array directly into the generated `main.cpp`.
    *   This ensures the binary is always available to the C++ component without needing external files or complex include paths.

### Static Linking (HP Mode)
*   **The "Shim Crate" Pattern:** `hp-firmware` acts as a wrapper around `common` to provide the required `#[panic_handler]` and `staticlib` configuration.
*   **Config:** Set `panic = "abort"` in the workspace to avoid `libunwind` dependencies.
*   **Linking:** The component's `__init__.py` adds the library path to the linker flags.

## 5. Hybrid HP/LP Mode

Configurable via YAML:
*   **LP Mode:** Provides the best timing guarantees and lowest power consumption.
*   **HP Mode:** Uses a high-priority FreeRTOS task on the main core. Useful for debugging or for ESP32 variants without an LP core.

## 6. Testing Strategy

*   **Unit Tests:** `cargo test -p hcp2-common` validates logic on the host machine.
*   **USB Simulation:** A dedicated test configuration (`tests/usb_simulation/`) allows testing the entire ESPHome component via a standard USB cable. A Python script simulates the drive, allowing verification of the UART logic and entity states without RS-485 hardware.