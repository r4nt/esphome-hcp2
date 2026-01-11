# HCP Tester Plan

This document outlines the architecture and implementation plan for the `hcp_tester`, a device designed to simulate a Hörmann garage door motor (Drive). It acts as the **Bus Drive** to drive the protocol and validate the `hcp_bridge`.

## 1. Objectives

1.  **Role:** Act as the **RS485 Drive**.
    *   Initiate Bus Scan.
    *   Broadcast Status updates (Position, Light).
    *   Poll the Bridge for commands.
2.  **Simulation:** Simulate the physical mechanics of a garage door.
    *   Physics: Interpolate `current_position` towards `target_position` over time.
    *   Logic: Handle Light/Vent toggles.
3.  **Interactivity:** Provide ESPHome UI controls to manipulate the *simulated* door state directly, forcing the Bridge to react.

## 2. Architecture

### A. Directory Structure

```text
/home/klimek/src/esphome-hcp2/
├── common/                     # Existing shared protocol definitions (Protocol, CRC, etc.)
├── tester-firmware/            # [NEW] Rust library for the Simulator logic
│   ├── Cargo.toml              # Defines dependencies
│   └── src/
│       ├── lib.rs              # FFI exports
│       ├── drive_fsm.rs        # Drive Protocol State Machine (Scan -> Broadcast -> Poll)
│       └── garage_physics.rs   # Simulation logic (Movement, State transitions)
├── components/
│   └── hcp_tester/             # [NEW] ESPHome C++ Component
│       ├── __init__.py         # Codegen and Configuration Schema
│       ├── hcp_tester.h        # Header
│       └── hcp_tester.cpp      # Implementation (calls Rust FFI)
└── example_tester_s3.yaml      # [NEW] ESPHome config for the Tester Device
```

### B. Rust Logic (`tester-firmware`)

The firmware acts as the **Drive**.

**State Machine (`drive_fsm.rs`):**
1.  **State: Scanning**
    *   Action: Send `Function 0x17` to Address `0x02` (Read 5 Registers).
    *   Expectation: Response with Device IDs (`0x0430`, etc.).
    *   Transition: If valid response, go to `Broadcast`.
2.  **State: Broadcast**
    *   Action: Send `Function 0x10` to Address `0x00` (Write 9 Registers).
    *   Data: Current Position, State (Opening/Closing), Light status.
    *   Transition: Immediate -> `Polling`.
3.  **State: Polling**
    *   Action: Send `Function 0x17` to Address `0x02` (Write Sync / Read 8 Registers).
    *   Expectation: Response containing Command requests (Open/Close/Light).
    *   Logic: If command received (e.g., `0x0210` Open), update `garage_physics` target.
    *   Transition: Wait delay -> `Broadcast`.

**Physics (`garage_physics.rs`):**
*   `tick()`: Moves `current_pos` towards `target_pos`. Updates `state` (Stopped, Opening, Closing, Open, Closed).

### C. C++ Component (`components/hcp_tester`)

Wraps the Rust Drive FSM.

**Entities:**
*   **Cover:** "Simulated Door" (Controls the physics `target_pos` manually).
*   **Switch:** "Simulated Light" (Toggles the physics `light_state`).
*   **Sensor:** "Bridge Command" (Shows what the Bridge asked for, e.g. "Request: Open").

### D. FFI (Rust <-> C++)

*   `tester_init()`: Setup FSM.
*   `tester_poll(hal, &state)`: Run FSM. Returns the current simulated state to C++ for UI updates.
*   `tester_set_control(...)`: Allow C++ (User UI) to override simulation (e.g. force door open).

## 3. Implementation Steps

1.  **Scaffold:** Create directories and files.
2.  **Rust Implementation:**
    *   Implement `garage_physics` struct.
    *   Implement `drive_fsm` using `common::protocol` to frame packets.
    *   Expose FFI.
3.  **ESPHome Component:**
    *   Create `hcp_tester` component that drives the Rust poll loop.
    *   Map UART read/write.
4.  **Integration:**
    *   Create `example_tester_s3.yaml`.
    *   Compile and verify.

## 4. Hardware Setup

*   **Tester (ESP32-S3):**
    *   TX -> RS485 DI
    *   RX -> RS485 RO
    *   Pin 4 -> RS485 DE/RE
*   **Target (HCP Bridge):** Connected to same RS485 bus.