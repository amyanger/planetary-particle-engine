use ppe_can::{well_known, CanFilter, VirtualCanBus};
use ppe_scheduler::{ScheduledTask, Scheduler};
use ppe_subsystems::{
    BatteryManagementSystem, BmsConfig, MotorController, Subsystem, ThermalManagement,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn scheduler_runs_subsystems_at_correct_rates() {
    let bus = VirtualCanBus::new(4096);

    // BMS
    let bms_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
    let (mut bms, _bms_handles) = BatteryManagementSystem::new(BmsConfig::default(), bms_node);
    bms.init().unwrap();

    // Motor
    let motor_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
    let (mut motor, _motor_handles) = MotorController::new(motor_node);
    motor.init().unwrap();

    // Thermal
    let thermal_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 64);
    let (mut thermal, _thermal_handles) = ThermalManagement::new(thermal_node);
    thermal.init().unwrap();

    let bms_ticks = Arc::new(AtomicU64::new(0));
    let motor_ticks = Arc::new(AtomicU64::new(0));
    let thermal_ticks = Arc::new(AtomicU64::new(0));

    let bms_ticks_c = bms_ticks.clone();
    let motor_ticks_c = motor_ticks.clone();
    let thermal_ticks_c = thermal_ticks.clone();

    let mut scheduler = Scheduler::new();

    // BMS at 10ms
    scheduler.add_task(ScheduledTask::new(
        1,
        "BMS",
        Duration::from_millis(10),
        move |dt| {
            bms_ticks_c.fetch_add(1, Ordering::Relaxed);
            bms.tick(dt)
        },
    ));

    // Motor at 10ms
    scheduler.add_task(ScheduledTask::new(
        2,
        "Motor",
        Duration::from_millis(10),
        move |dt| {
            motor_ticks_c.fetch_add(1, Ordering::Relaxed);
            motor.tick(dt)
        },
    ));

    // Thermal at 100ms
    scheduler.add_task(ScheduledTask::new(
        3,
        "Thermal",
        Duration::from_millis(100),
        move |dt| {
            thermal_ticks_c.fetch_add(1, Ordering::Relaxed);
            thermal.tick(dt)
        },
    ));

    let stop = scheduler.stop_handle();

    // Run for ~500ms
    let handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    tokio::time::sleep(Duration::from_millis(500)).await;
    stop.store(false, std::sync::atomic::Ordering::SeqCst);

    let _ = tokio::time::timeout(Duration::from_millis(100), handle).await;

    let bms_count = bms_ticks.load(Ordering::Relaxed);
    let motor_count = motor_ticks.load(Ordering::Relaxed);
    let thermal_count = thermal_ticks.load(Ordering::Relaxed);

    // BMS and Motor should have ~50 ticks each (500ms / 10ms)
    // Allow generous tolerance due to scheduling overhead
    assert!(
        bms_count >= 15,
        "BMS should have many ticks, got {bms_count}"
    );
    assert!(
        motor_count >= 15,
        "Motor should have many ticks, got {motor_count}"
    );
    // Thermal should have ~5 ticks (500ms / 100ms)
    assert!(
        thermal_count >= 3,
        "Thermal should have some ticks, got {thermal_count}"
    );
    // Thermal should tick much less than BMS/Motor
    assert!(
        thermal_count < bms_count,
        "Thermal ({thermal_count}) should tick less than BMS ({bms_count})"
    );
}
