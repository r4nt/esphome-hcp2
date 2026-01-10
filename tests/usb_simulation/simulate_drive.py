#!/usr/bin/env python3
import serial
import time
import struct
import sys

# CRC16-Modbus Implementation
def crc16(data: bytes) -> bytes:
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return struct.pack('<H', crc)

def send_packet(ser, frame_hex):
    data = bytes.fromhex(frame_hex)
    crc = crc16(data)
    full_frame = data + crc
    print(f"Master  -> ESP: {full_frame.hex().upper()}")
    ser.write(full_frame)
    return full_frame

def read_response(ser, timeout=0.2):
    start = time.time()
    while (time.time() - start) < timeout:
        if ser.in_waiting >= 5: # Min modbus response size
            resp = ser.read(ser.in_waiting)
            print(f"Master <-  ESP: {resp.hex().upper()}")
            return resp
        time.sleep(0.01)
    print("Master <-  ESP: (Timeout - No Response)")
    return None

def main():
    # Adjust port as needed (ttyUSB0 or ttyACM0 usually)
    port = '/dev/ttyACM0' if len(sys.argv) < 2 else sys.argv[1]
    baud = 57600
    
    try:
        ser = serial.Serial(port, baud, parity=serial.PARITY_EVEN, timeout=0.05)
        print(ser)
    except Exception as e:
        print(f"Error: Could not open port {port}: {e}")
        return

    print(f"Connected to {port} at {baud} baud.")
    print("Simulating Hoermann Drive... (Press Ctrl+C to stop)")

    try:
        while True:
            # 1. Bus Scan (Read 5 Registers from address 0x9CB9)
            print("\n[Bus Scan]")
            # 02 (Addr), 17 (Func), 9C B9 (Read Addr), 00 05 (Read Qty), 9C 41 (Write Addr), 00 01 (Write Qty), 02 (ByteCnt), 00 00 (Write Data)
            send_packet(ser, "02 17 9C B9 00 05 9C 41 00 01 02 00 00")
            read_response(ser)
            time.sleep(0.5)

            # 2. Status Update (Broadcast address 0x00, Func 0x10, Addr 0x9D31)
            # This sets the door state to "Opening" (01) at 25% (50/200 = 0x32)
            print("\n[Status Update - Opening 25%]")
            # Reg 1: 00 32 (Pos), Reg 2: 01 00 (State)
            # 00 (Addr), 10 (Func), 9D 31 (Addr), 00 09 (Qty), 12 (ByteCnt) + 18 bytes of data
            send_packet(ser, "00 10 9D 31 00 09 12 00 00 00 32 01 00 00 00 00 00 00 00 00 10 00 00 00 00")
            # No response expected for broadcast
            time.sleep(0.5)

            # 3. Poll (Read 8 Registers from address 0x9CB9)
            print("\n[Command Poll]")
            # 02 (Addr), 17 (Func), 9C B9 (Read Addr), 00 08 (Read Qty), 9C 41 (Write Addr), 00 01 (Write Qty), 02 (ByteCnt), 00 00 (Write Data)
            send_packet(ser, "02 17 9C B9 00 08 9C 41 00 01 02 00 00")
            resp = read_response(ser)
            
            if resp and len(resp) > 8:
                # Reg 2 is at byte 7,8? (Addr, Func, Len, Data...)
                # Response for length 8 has 16 bytes of data.
                # Reg 2 is bytes 7 and 8 of the response frame.
                reg2 = (resp[7] << 8) | resp[8]
                if reg2 != 0:
                    print(f">>> ESP REQUESTED ACTION: {hex(reg2)}")

            time.sleep(1.0)

    except KeyboardInterrupt:
        print("\nStopping simulation.")
    finally:
        ser.close()

if __name__ == "__main__":
    main()
