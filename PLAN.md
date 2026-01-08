# HCP2 on ESP32-C6 LP Core Implementation Plan

This plan outlines the architecture and steps to implement the Hoermann HCP2 protocol on the ESP32-C6 Low Power (LP) RISC-V processor using Rust (`no_std`) and `rmodbus`.

## Goal
Offload the timing-sensitive and continuous polling HCP2 communication to the LP core to save power and free up the High Performance (HP) core for WiFi/Home Assistant tasks.

## Architecture

### 1. Hardware & Environment
*   **Target:** ESP32-C6 LP Core (RISC-V).
*   **Language:** Rust (`no_std`).
*   **Libraries:** `rmodbus` (for Modbus RTU framing and parsing).
*   **Communication:**
    *   **LP UART:** Direct register access or HAL wrapper to communicate with the HCP bus.
    *   **HP <-> LP:** Shared memory in `RTC_SLOW_MEM` (LP RAM) protected by a Spinlock.

### 2. Directory Structure

```text
esphome-hcp2/
├── lp-firmware/                # Rust crate for the LP Core
│   ├── Cargo.toml              # Dependencies: rmodbus, riscv, critical-section
│   ├── .cargo/
│   │   └── config.toml         # Target configuration (riscv32imac-unknown-none-elf)
│   ├── build.rs                # Linker script generation
│   ├── memory.x                # Linker script for LP RAM layout
│   └── src/
│       ├── main.rs             # Entry point, main loop
│       ├── uart.rs             # Minimal LP UART driver (RX/TX ring buffers)
│       ├── protocol.rs         # HCP2 state machine using rmodbus
│       ├── shared.rs           # Shared memory structs (repr(C)) & Spinlock
│       └── registers.rs        # Register definitions from PROTOCOL.md
└── components/                 # ESPHome C++ components (HP Core)
    └── hcp_bridge/             # Reads/Writes shared memory to talk to LP
        ├── hcp_bridge.h
        └── hcp_bridge.cpp
```

## Component Implementation Details

### A. Shared Memory (`src/shared.rs`)
Defines the interface between HP and LP cores. Must be `#[repr(C)]` and aligned.

*   **Lock:** A simple atomic spinlock or flag (e.g., `AtomicBool`) to ensure only one core writes at a time.
*   **Data Structure:**
    ```rust
    #[repr(C)]
    struct SharedData {
        // Sync
        lock: AtomicBool,
        
        // HP -> LP (Command)
        command_request: u8,      // e.g., OPEN, CLOSE, LIGHT
        target_position: u8,      // 0-200
        
        // LP -> HP (Status)
        current_state: u8,        // STOPPED, OPENING, etc.
        current_position: u8,     // 0-200
        light_on: bool,
        last_update_ts: u32,      // Timestamp of last valid packet
        error_code: u8,
    }
    ```

### B. LP UART Driver (`src/uart.rs`)
Since standard HALs might be too heavy or unavailable for the LP core context:
*   Direct access to LP_UART registers.
*   **Polling Mode:** For simplicity in the initial implementation, or ISR-based if efficient enough.
*   **Functions:** `init()`, `read_byte()`, `write_bytes()`, `flush()`.

### C. Protocol Logic (`src/protocol.rs`)
Uses `rmodbus` in `no_std` mode.

1.  **Modbus Server context:** The HCP device acts as a Slave.
2.  **State Machine:**
    *   **Idle:** Waiting for frames.
    *   **Frame Received:** Pass buffer to `rmodbus`.
    *   **Process Request:**
        *   If `0x10 (Write)` to `0x9D31`: Update Status in Shared Memory.
        *   If `0x17 (Read/Write)`:
            *   Read Sync data.
            *   Prepare Response: Check Shared Memory for pending commands (Button Presses) and populate the "Length 8" response registers.
    *   **Respond:** Send frame via UART.

### D. Main Loop (`src/main.rs`)
1.  Initialize UART and Shared Memory.
2.  Enter infinite loop:
    *   Check UART for incoming data.
    *   Run Protocol State Machine.
    *   Update Shared Memory (acquire lock -> update -> release).
    *   (Optional) Deep Sleep yield if waiting.

## Implementation Steps

1.  **Project Setup:** Initialize the `lp-firmware` Rust crate with correct target and linker scripts.
2.  **Shared Memory:** Define the struct and verify memory layout.
3.  **UART Driver:** Implement basic echo test on LP UART.
4.  **Rmodbus Integration:** Compile `rmodbus` with `no_std` and verify frame parsing.
5.  **HCP2 Logic:** Implement the handlers for `0x10` and `0x17` function codes based on `PROTOCOL.md`.
6.  **Integration:** Create the ESPHome C++ component to read the shared memory and expose it as entities (Cover, Light, Switch).

## Testing Plan

### 1. Unit Tests (Rust - Host Machine)
Since the protocol logic is platform-independent `no_std` Rust, we can test it thoroughly on the host machine before flashing.

*   **Protocol Logic:**
    *   **Mock UART:** Create a mock implementation of the UART trait to inject byte streams (e.g., recorded Modbus frames from `PROTOCOL.md`) and capture responses.
    *   **State Machine Verification:** Test transitions (e.g., receiving a 'Status Update' should update the internal state struct).
    *   **Command Generation:** Verify that setting a flag in `SharedMemory` correctly generates the corresponding Modbus response (e.g., "Open Door" action registers).
    *   **CRC Validation:** Ensure `rmodbus` or internal CRC logic correctly accepts valid frames and rejects corrupt ones.

*   **Shared Memory Layout:**
    *   **Alignment/Padding:** Use `memoffset` or standard `size_of`/`align_of` tests to ensure the C-struct layout matches exactly what the C++ HP core expects, preventing misalignment issues.

### 2. Integration & System Tests

*   **LP UART Loopback (Hardware):**
    *   Connect LP UART TX to RX on the dev board.
    *   Send a known string/frame and verify it reads back correctly. This validates the low-level UART driver and clock configuration.

*   **HP <-> LP Shared Memory Test:**
    *   **Write Test:** HP writes a pattern (e.g., `0xDEADBEEF`) to shared memory. LP reads it and blinks an LED or echoes it via UART.
    *   **Read Test:** LP increments a counter in shared memory. HP reads and logs it to the console.
    *   **Locking:** Stress test concurrent access (if possible) or verify the spinlock mechanism prevents data corruption.

*   **Real Hardware / Simulator:**
    *   **Device Simulator:** If a real HCP drive is unavailable, use a USB-RS485 adapter on a PC running a Python script (using `pymodbus`) to act as the Master (Drive).
        *   The PC sends "Bus Scan" -> LP should reply `0x0430...`.
        *   The PC sends "Status Update" -> LP should update Shared Memory (verify via ESPHome logs).
    *   **Real Drive:** Connect to the actual garage door opener. Verify:
        *   Discovery works.
        *   State changes (manual door movement) are reflected in ESPHome.
        *   Commands (Open/Light) from ESPHome work.
