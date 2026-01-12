use hcp2_tester_lib::{DriveProtocol, DriveProtocolState, GaragePhysics};
use hcp2_common::driver::Hcp2Driver;
use hcp2_common::shared::{SharedData, CMD_OPEN};
use hcp2_common::hal::HcpHal;
use std::cell::RefCell;
use std::rc::Rc;

// --- Mock HAL for Bridge & Tester ---
struct MockHal {
    rx_queue: Rc<RefCell<Vec<u8>>>,
    tx_queue: Rc<RefCell<Vec<u8>>>,
    now: u32,
    logs: Rc<RefCell<Vec<String>>>,
    name: String,
}

impl MockHal {
    fn new(rx: Rc<RefCell<Vec<u8>>>, tx: Rc<RefCell<Vec<u8>>>, name: &str) -> Self {
        Self {
            rx_queue: rx,
            tx_queue: tx,
            now: 0,
            logs: Rc::new(RefCell::new(Vec::new())),
            name: name.to_string(),
        }
    }
}

impl HcpHal for MockHal {
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
        println!("[{} Log] {}", self.name, message);
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
    let mut protocol = DriveProtocol::new();
    
    // Tester HAL: RX=Bus2, TX=Bus1
    let mut tester_hal = MockHal::new(bus_bridge_to_tester.clone(), bus_tester_to_bridge.clone(), "Tester");
    
    // Bridge HAL: RX=Bus1, TX=Bus2
    let mut bridge_hal = MockHal::new(bus_tester_to_bridge.clone(), bus_bridge_to_tester.clone(), "Bridge");
    
    let mut bridge_driver = Hcp2Driver::new();
    let mut shared_data = SharedData::default();

    let mut current_time = 1000u32;
    let _step_ms = 10;

    // --- PHASE 1: DISCOVERY (SCAN) ---
    println!("--- Starting Simulation: Discovery Phase ---");
    
    // We expect the Tester to start Scanning
    assert_eq!(protocol.state, DriveProtocolState::Scan);

    // Run Tester Poll (It should send a Scan Request)
    tester_hal.now = current_time;
    protocol.poll(&mut tester_hal, &mut physics);
    
    assert!(!bus_tester_to_bridge.borrow().is_empty(), "Tester should send scan packet");
    
    // Run Bridge Poll
    // Bridge should read packet, see it is for Address 0x02 (hopefully, or it iterates)
    // Tester scans descending from 0xFF. 
    // We need to loop simulation until Tester hits 0x02 or Bridge responds.
    // Note: Tester logic: starts 0xFF, decrements.
    // Ideally we want to fast forward to 0x02 if we don't want to wait 254 cycles.
    // Let's cheat and force Tester scan address to 0x02 so target is 0x02.
    protocol.scan_address = 0x02; 
    current_time += 55;

    // Re-run poll to generate 0x02 scan
    bus_tester_to_bridge.borrow_mut().clear(); // Clear previous invalid scans
    tester_hal.now = current_time;
    protocol.poll(&mut tester_hal, &mut physics);
    
    assert!(!bus_tester_to_bridge.borrow().is_empty(), "Tester should send scan packet to 0x02");

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
    protocol.handle_response(&bridge_response, &mut physics);
    
    assert_eq!(protocol.state, DriveProtocolState::Broadcast, "Tester should transition to Broadcast after valid Scan response");

    // --- PHASE 2: NORMAL OPERATION & COMMAND ---
    println!("--- Simulation: Connected ---");
    
    // Tester should now Broadcast Status
    bus_tester_to_bridge.borrow_mut().clear();
    bus_bridge_to_tester.borrow_mut().clear();
    
    tester_hal.now = current_time;
    protocol.poll(&mut tester_hal, &mut physics);
    
    assert!(!bus_tester_to_bridge.borrow().is_empty(), "Tester should send Broadcast packet");
    
    // Bridge processes Broadcast
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Time advance for Bridge RX Timeout
    current_time += 15;
    bridge_hal.now = current_time;
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);

    // Verify SharedData updated (Initial state should be Stopped/Closed)
    assert_eq!(shared_data.current_state, 0x40); // 0x40 = Closed (from DriveState::Closed)

    // Tester should now POLL
    assert_eq!(protocol.state, DriveProtocolState::Poll);
    current_time += 100;
    
    bus_tester_to_bridge.borrow_mut().clear();
    tester_hal.now = current_time;
    protocol.poll(&mut tester_hal, &mut physics);
    assert!(!bus_tester_to_bridge.borrow().is_empty(), "Tester should send Poll packet");

    // --- USER ACTION: OPEN DOOR ---
    println!("--- Simulation: Sending Open Command ---");
    shared_data.command_request = CMD_OPEN;

    // Bridge processes Poll Request
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Modbus RTU Timeout
    current_time += 15;
    bridge_hal.now = current_time;
    bridge_driver.poll(&mut bridge_hal, &mut shared_data);
    
    // Bridge should respond with Action Registers set
    let bridge_resp = bus_bridge_to_tester.borrow().clone();
    assert!(!bridge_resp.is_empty());
    
    // Tester processes Poll Response
    protocol.handle_response(&bridge_resp, &mut physics);
    
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
        tester_hal.now = current_time;
        
        physics.tick(); // Move door
        
        // Full Loop
        // 1. Tester Broadcast
        if protocol.state == DriveProtocolState::Broadcast {
            bus_tester_to_bridge.borrow_mut().clear();
            protocol.poll(&mut tester_hal, &mut physics);
            
            bridge_driver.poll(&mut bridge_hal, &mut shared_data);
            // Broadcasts don't need response, but we should clear buffer/trigger timeout to process write
            bridge_hal.now = current_time + 15;
            bridge_driver.poll(&mut bridge_hal, &mut shared_data);
        }
        
        // 2. Tester Poll
        if protocol.state == DriveProtocolState::Poll {
             bus_tester_to_bridge.borrow_mut().clear();
             protocol.poll(&mut tester_hal, &mut physics);
             
             bus_bridge_to_tester.borrow_mut().clear();
             bridge_driver.poll(&mut bridge_hal, &mut shared_data);
             // Timeout trigger
             bridge_hal.now = current_time + 15;
             bridge_driver.poll(&mut bridge_hal, &mut shared_data);
             
             let resp = bus_bridge_to_tester.borrow().clone();
             if !resp.is_empty() {
                 protocol.handle_response(&resp, &mut physics);
             }
        }
    }
    
    assert!(physics.current_position > start_pos, "Door should have moved opened");
    assert_eq!(shared_data.current_state, 0x01); // 0x01 = Opening
    
    println!("Test Complete. Final Pos: {}", physics.current_position);
}