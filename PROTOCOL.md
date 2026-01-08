# Hoermann HCP2 Protocol

This document describes the HCP2 protocol implementation. The protocol is based on Modbus RTU over a serial connection (RS-485 logic).

## Physical Layer
*   **Interface:** Serial (UART)
*   **Baud Rate:** 57600
*   **Data Bits:** 8
*   **Stop Bits:** 1
*   **Parity:** Even

## General Flow

The communication is master-slave, where the Drive (Master) initiates all exchanges. The HCP device (Slave) responds to requests.

### 1. Bus Scan (Discovery)
Upon startup or periodically, the Drive scans for connected devices.
*   **Method:** Read/Write (0x17)
*   **Target:** Address `0x02`
*   **Read Quantity:** 5 Registers
*   **Action:** The device responds with identification constants (`0x0430`, `0x10FF`, `0xA845`) to announce its presence.

### 2. Status Broadcast
The Drive broadcasts its current state to all devices.
*   **Method:** Write Multiple Registers (0x10)
*   **Target:** Address `0x00` (Broadcast)
*   **Write Address:** `0x9D31`
*   **Action:** The device updates its internal state (Door Open/Closed, Light On/Off, etc.) but **does not reply**.

### 3. Polling / Command Loop
The Drive polls the device to check if it has any commands to execute (e.g., button pressed).
*   **Method:** Read/Write (0x17)
*   **Target:** Address `0x02`
*   **Read Quantity:** 8 Registers (Active Poll) or 2 Registers (Idle Poll)
*   **Action:** The device responds. If a specific action (Open, Close, Light) is requested, the device populates the specific registers in the response.

## Frame Structure

The protocol uses a Modbus-like frame structure.

### Request Frame (Master -> Slave)
| Byte Offset | Field | Size | Description |
| :--- | :--- | :--- | :--- |
| 0 | Address | 1 Byte | `0x00` (Broadcast), `0x02` (HCP) |
| 1 | Function Code | 1 Byte | `0x10` (Write), `0x17` (Read/Write) |
| ... | Data | ... | Depends on Function Code |
| N-2 | CRC Low | 1 Byte | Modbus CRC16 |
| N-1 | CRC High | 1 Byte | Modbus CRC16 |

### Response Frame (Slave -> Master)
| Byte Offset | Field | Size | Description |
| :--- | :--- | :--- | :--- |
| 0 | Address | 1 Byte | `0x02` |
| 1 | Function Code | 1 Byte | `0x10` or `0x17` |
| 2 | Byte Count | 1 Byte | Number of data bytes following |
| 3 | Data | N Bytes | Register Data |
| 3+N | CRC Low | 1 Byte | Modbus CRC16 |
| 3+N+1 | CRC High | 1 Byte | Modbus CRC16 |

## Function Codes

### 0x10: Write Multiple Registers
Used by the master (drive) to send status updates.

**Request:**
*   Address
*   Function Code (0x10)
*   Starting Address (2 Bytes)
*   Quantity of Registers (2 Bytes)
*   Byte Count (1 Byte)
*   Registers Value (N * 2 Bytes)
*   CRC

### 0x17: Read/Write Multiple Registers
Used by the master to read status/commands from the slave while writing synchronization data.

**Request:**
*   Address
*   Function Code (0x17)
*   Read Starting Address (2 Bytes)
*   Read Quantity of Registers (2 Bytes)
*   Write Starting Address (2 Bytes)
*   Write Quantity of Registers (2 Bytes)
*   Write Byte Count (1 Byte)
*   Write Registers Value (N * 2 Bytes)
*   CRC

## Registers and Commands

### Master Write (Drive -> Device)

#### Status Update (Write Address `0x9D31`)
The drive writes 9 registers to address `0x9D31`.

| Register Index | High Byte | Low Byte | Description |
| :--- | :--- | :--- | :--- |
| 0 | - | - | Unknown / Unused in code |
| 1 | Target Position | Current Position | Position (0-200, where 200 = 100% Open) |
| 2 | State | - | [Drive State](#drive-states) |
| 3 | - | - | Unknown |
| 4 | - | - | Unknown |
| 5 | - | - | Unknown |
| 6 | - | Light Status | Bit `0x10` indicates Light On/Off |
| 7 | - | - | Unknown |
| 8 | - | - | Unknown |

#### Sync/Counter (Write Address `0x9C41`)
The drive writes to address `0x9C41`.

| Register Index | High Byte | Low Byte | Description |
| :--- | :--- | :--- | :--- |
| 0 | Counter | Command Code | Sync counter and command code |

### Master Read (Device -> Drive)

The drive reads from address `0x9CB9`. The device responds based on the requested quantity.

#### Length 2 (Idle Poll)
| Register Index | High Byte | Low Byte | Description |
| :--- | :--- | :--- | :--- |
| 0 | Counter | `0x04` | Echo Counter |
| 1 | Command Code | `0x00` | Echo Command Code |

#### Length 5 (Bus Scan / Identification)
| Register Index | High Byte | Low Byte | Description |
| :--- | :--- | :--- | :--- |
| 0 | Counter | `0x00` | Echo Counter |
| 1 | Command Code | `0x05` | Echo Command Code |
| 2 | `0x04` | `0x30` | Device Type / ID |
| 3 | `0x10` | `0xFF` | Device Type / ID |
| 4 | `0xA8` | `0x45` | Device Type / ID |

#### Length 8 (Command Action)
Used to send commands to the drive.

| Register Index | High Byte | Low Byte | Description |
| :--- | :--- | :--- | :--- |
| 0 | Counter | `0x00` | Echo Counter |
| 1 | Command Code | `0x01` | Echo Command Code |
| 2 | Action Reg 1 High | Action Reg 1 Low | See [Actions](#actions) |
| 3 | Action Reg 2 High | Action Reg 2 Low | See [Actions](#actions) |
| 4 | `0x00` | `0x00` | Padding |
| 5 | `0x00` | `0x00` | Padding |
| 6 | `0x00` | `0x00` | Padding |
| 7 | `0x00` | `0x00` | Padding |

## Drive States
Defined in `State` enum (from Register 2 High Byte of `0x9D31`):

| Value | State |
| :--- | :--- |
| `0x00` | Stopped |
| `0x01` | Opening |
| `0x02` | Closing |
| `0x05` | Move Half |
| `0x09` | Move Venting |
| `0x0A` | Vent Reached |
| `0x20` | Open |
| `0x40` | Closed |
| `0x80` | Half Open Reached |

## Actions
The device sends commands by setting Register 2 and 3 in the Length 8 response.
The protocol simulates a button press by sending a "Pressing" value for `500ms`, followed by a "Release" value once.

| Action | Reg 2 (Pressing) | Reg 3 (Pressing) | Reg 2 (Release) | Reg 3 (Release) |
| :--- | :--- | :--- | :--- | :--- |
| **None** | `0x0000` | `0x0000` | - | - |
| **Open Door** | `0x0210` | `0x0000` | `0x0110` | `0x0000` |
| **Close Door** | `0x0220` | `0x0000` | `0x0120` | `0x0000` |
| **Stop Door** | `0x0240` | `0x0000` | `0x0140` | `0x0000` |
| **Half Open** | `0x0200` | `0x0400` | `0x0100` | `0x0400` |
| **Vent** | `0x0200` | `0x4000` | `0x0100` | `0x4000` |
| **Toggle Light** | `0x0100` | `0x0200` | `0x0800` | `0x0200` |

## Checksum (CRC)
Standard Modbus CRC16.
*   Polynomial: `0xA001`
*   Initial Value: `0xFFFF`