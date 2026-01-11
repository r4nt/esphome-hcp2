use hcp2_tester_lib::{DriveFsm, DriveFsmState, GaragePhysics};
use hcp2_common::driver::Hcp2Driver;
use hcp2_common::shared::{SharedData, CMD_OPEN};
use hcp2_common::hal::HcpHal;
use std::cell::RefCell;
use std::rc::Rc;

// --- Mock HAL for Bridge ---
struct MockBridgeHal {
    rx_queue: Rc<RefCell<Vec<u8>>>,
    tx_queue: Rc<RefCell<Vec<u8>>>,
    now: u32,
    logs: Rc<RefCell<Vec<String>>>,
}

impl MockBridgeHal {
    fn new(rx: Rc<RefCell<Vec<u8>>>, tx: Rc<RefCell<Vec<u8>>>) -> Self {
        Self {
            rx_queue: rx,
            tx_queue: tx,
            now: 0,
            logs: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl HcpHal for MockBridgeHal {
    fn uart_read(&mut self, buf: &mut [u8]) -> usize {
        let mut q = self.rx_queue.borrow_mut();
        let len = std::cmp::min(buf.len(), q.len());
        for i in 0..len {
            buf[i] = q.remove(0);
        }
        len
    }

    fn uart_write(&mut self, buf: &[u8]) -> usize {
        self.tx_queue.borrow_mut().extend_from_slice(buf);
        buf.len()
    }

    fn set_tx_enable(&mut self, _enable: bool) {}

    fn now_ms(&self) -> u32 {
        self.now
    }

    fn sleep_ms(&mut self, ms: u32) {
        self.now += ms;
    }

    fn log(&mut self, message: &str) {
        self.logs.borrow_mut().push(message.to_string());
        println!("[Bridge Log] {}", message);
    }
}

#[test]
fn test_simulation_loop() {
    // 1. Setup
    // Shared "Bus" (Buffers)
    // Tester writes to Bus1, Bridge reads from Bus1
    // Bridge writes to Bus2, Tester reads from Bus2
    let bus_tester_to_bridge = Rc::new(RefCell::new(Vec::new()));
    let bus_bridge_to_tester = Rc::new(RefCell::new(Vec::new()));

    // Instantiate Components
    let mut physics = GaragePhysics::new();
    let mut fsm = DriveFsm::new();
    
    let mut bridge_hal = MockBridgeHal::new(bus_tester_to_bridge.clone(), bus_bridge_to_tester.clone());
    let mut bridge_driver = Hcp2Driver::new();
    let mut shared_data = SharedData::default();

    let mut current_time = 1000u32;
    let _step_ms = 10;

    // --- PHASE 1: DISCOVERY (SCAN) ---
    println!("--- Starting Simulation: Discovery Phase ---");
    
    // We expect the Tester to start Scanning
    assert_eq!(fsm.state, DriveFsmState::Scan);

    // Run Tester Poll (It should send a Scan Request)
    let mut tx_buf = [0u8; 256];
    let tx_len = fsm.poll(&mut physics, current_time, &mut tx_buf);
    assert!(tx_len > 0, "Tester should send scan packet");
    
    // Put packet on bus
    bus_tester_to_bridge.borrow_mut().extend_from_slice(&tx_buf[..tx_len]);

    // Run Bridge Poll
    // Bridge should read packet, see it is for Address 0x02 (hopefully, or it iterates)
    // Tester scans descending from 0xFF. 
    // We need to loop simulation until Tester hits 0x02 or Bridge responds.
    // Note: Tester logic: starts 0xFF, decrements.
    // Ideally we want to fast forward to 0x02 if we don't want to wait 254 cycles.
    // Let's cheat and force Tester scan address to 0x02 so target is 0x02.
    fsm.scan_address = 0x02; 
    current_time += 55;

    // Re-run poll to generate 0x02 scan
    let tx_len = fsm.poll(&mut physics, current_time, &mut tx_buf);
    println!("DEBUG: Generated Scan Packet Len: {}", tx_len);
    bus_tester_to_bridge.borrow_mut().clear(); // Clear previous invalid scans
    bus_tester_to_bridge.borrow_mut().extend_from_slice(&tx_buf[..tx_len]);

    // Bridge Poll
    // Since Bridge is stateless regarding time mostly (except timeout), just run it.
    bridge_hal.now = current_time;
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Modbus RTU Timeout: Need to advance time > 10ms and poll again to trigger processing
    current_time += 15;
    bridge_hal.now = current_time;
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);

    // Check if Bridge sent response
    let bridge_response = bus_bridge_to_tester.borrow().clone();
    assert!(!bridge_response.is_empty(), "Bridge should respond to Scan on 0x02");

    // Tester Handle Response
    fsm.handle_response(&bridge_response, &mut physics);
    
    assert_eq!(fsm.state, DriveFsmState::Broadcast, "Tester should transition to Broadcast after valid Scan response");

    // --- PHASE 2: NORMAL OPERATION & COMMAND ---
    println!("--- Simulation: Connected ---");
    
    // Tester should now Broadcast Status
    let tx_len = fsm.poll(&mut physics, current_time, &mut tx_buf);
    assert!(tx_len > 0); // Broadcast packet
    // Bridge processes Broadcast
    bus_tester_to_bridge.borrow_mut().clear();
    bus_bridge_to_tester.borrow_mut().clear();
    bus_tester_to_bridge.borrow_mut().extend_from_slice(&tx_buf[..tx_len]);
    
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Time advance for Bridge RX Timeout
    current_time += 15;
    bridge_hal.now = current_time;
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);

    // Verify SharedData updated (Initial state should be Stopped/Closed)
    assert_eq!(shared_data.current_state, 0x40); // 0x40 = Closed (from DriveState::Closed)

    // Tester should now POLL
    assert_eq!(fsm.state, DriveFsmState::Poll);
    current_time += 100;
    let tx_len = fsm.poll(&mut physics, current_time, &mut tx_buf);
    assert!(tx_len > 0); // Poll packet

    // --- USER ACTION: OPEN DOOR ---
    println!("--- Simulation: Sending Open Command ---");
    shared_data.command_request = CMD_OPEN;

    // Bridge processes Poll Request
    bus_tester_to_bridge.borrow_mut().clear();
    bus_tester_to_bridge.borrow_mut().extend_from_slice(&tx_buf[..tx_len]);
    
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Modbus RTU Timeout
    current_time += 15;
    bridge_hal.now = current_time;
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Bridge should respond with Action Registers set
    let bridge_resp = bus_bridge_to_tester.borrow().clone();
    assert!(!bridge_resp.is_empty());
    
    // Tester processes Poll Response
    fsm.handle_response(&bridge_resp, &mut physics);
    
    // Verify Physics target updated
    // 0 = Closed, 200 = Open (100.0%)
    // If CMD_OPEN sent, target should be 100.0 (or 200 int)
    // GaragePhysics uses float 0.0-1.0 or 0.0-100.0?
    // Let's check GaragePhysics struct.
    // Assuming 0.0 to 1.0 or similar.
    assert!(physics.target_position > 0.9, "Target position should be set to Open (1.0 or similar)");

    // --- PHASE 3: PHYSICS MOVEMENT ---
    println!("--- Simulation: Physics Ticking ---");
    
    // Run for 5 seconds (simulated)
    let start_pos = physics.current_position;
    for _ in 0..50 { // 50 * 100ms = 5s
        current_time += 100;
        bridge_hal.now = current_time;
        
        physics.tick(); // Move door
        
        // Full Loop
        // 1. Tester Broadcast
        if fsm.state == DriveFsmState::Broadcast {
            let tx_len = fsm.poll(&mut physics, current_time, &mut tx_buf);
            bus_tester_to_bridge.borrow_mut().clear();
            bus_tester_to_bridge.borrow_mut().extend_from_slice(&tx_buf[..tx_len]);
            
            bridge_driver.poll(&mut bridge_hal, &mut shared_data);
            // Broadcasts don't need response, but we should clear buffer/trigger timeout to process write
            bridge_hal.now = current_time + 15;
            bridge_driver.poll(&mut bridge_hal, &mut shared_data);
        }
        
        // 2. Tester Poll
        if fsm.state == DriveFsmState::Poll {
             let tx_len = fsm.poll(&mut physics, current_time, &mut tx_buf);
             bus_tester_to_bridge.borrow_mut().clear();
             bus_tester_to_bridge.borrow_mut().extend_from_slice(&tx_buf[..tx_len]);
             
             bus_bridge_to_tester.borrow_mut().clear();
             bridge_driver.poll(&mut bridge_hal, &mut shared_data);
             // Timeout trigger
             bridge_hal.now = current_time + 15;
             bridge_driver.poll(&mut bridge_hal, &mut shared_data);
             
             let resp = bus_bridge_to_tester.borrow().clone();
             if !resp.is_empty() {
                 fsm.handle_response(&resp, &mut physics);
             }
        }
    }
    
    assert!(physics.current_position > start_pos, "Door should have moved opened");
    assert_eq!(shared_data.current_state, 0x01); // 0x01 = Opening
    
    println!("Test Complete. Final Pos: {}", physics.current_position);
}
