use ppe_can::{well_known, CanFilter, CanId, VirtualCanBus};
use ppe_subsystems::{BatteryManagementSystem, BmsConfig, Subsystem};
use std::time::Duration;

#[test]
fn bms_tick_100_times_publishes_correct_can() {
    let bus = VirtualCanBus::new(512);
    let bms_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
    let monitor = bus.connect(
        CanFilter::Range {
            low: CanId::new(0x100).unwrap(),
            high: CanId::new(0x10F).unwrap(),
        },
        512,
    );

    let config = BmsConfig::default();
    let (mut bms, handles) = BatteryManagementSystem::new(config, bms_node);
    bms.init().unwrap();

    // Simulate moderate discharge
    handles.pack_current.set(50.0);

    for _ in 0..100 {
        bms.tick(Duration::from_millis(10)).unwrap();
    }

    std::thread::sleep(Duration::from_millis(100));
    let frames = monitor.drain();

    // Should have 100 ticks * 4 messages = 400 frames
    assert!(
        frames.len() >= 100,
        "Expected many CAN frames, got {}",
        frames.len()
    );

    // Verify we got SOC, voltage, current, and temperature frames
    let soc_frames: Vec<_> = frames
        .iter()
        .filter(|f| f.id == well_known::BMS_SOC)
        .collect();
    let voltage_frames: Vec<_> = frames
        .iter()
        .filter(|f| f.id == well_known::BMS_VOLTAGE)
        .collect();
    let current_frames: Vec<_> = frames
        .iter()
        .filter(|f| f.id == well_known::BMS_CURRENT)
        .collect();
    let temp_frames: Vec<_> = frames
        .iter()
        .filter(|f| f.id == well_known::BMS_TEMPERATURE)
        .collect();

    assert!(!soc_frames.is_empty(), "Should have SOC frames");
    assert!(!voltage_frames.is_empty(), "Should have voltage frames");
    assert!(!current_frames.is_empty(), "Should have current frames");
    assert!(!temp_frames.is_empty(), "Should have temperature frames");

    // Verify last SOC is close to initial (100 ticks at 10ms = 1s total, minor discharge)
    let last_soc = soc_frames.last().unwrap();
    let soc_raw = u16::from_le_bytes([last_soc.data[0], last_soc.data[1]]);
    let soc_pct = soc_raw as f64 / 100.0;
    assert!(
        soc_pct > 70.0 && soc_pct <= 80.0,
        "SOC should be between 70-80% after brief discharge: got {soc_pct}"
    );
}
