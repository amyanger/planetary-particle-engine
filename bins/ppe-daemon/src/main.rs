use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use tracing_subscriber::EnvFilter;

use ppe_can::{well_known, CanFilter, VirtualCanBus};
use ppe_dashboard::{draw_dashboard, DashboardState};
use ppe_diagnostics::{DtcManager, ObdLiveData, ObdResponder};
use ppe_sim::{PhysicsHandles, Scenario, ScenarioKind, VehiclePhysics, VehiclePhysicsConfig};
use ppe_state::{Gear, VehicleEvent, VehicleFsm};
use ppe_subsystems::{
    BatteryManagementSystem, BmsConfig, MotorController, Subsystem, ThermalManagement,
};

#[derive(Parser)]
#[command(
    name = "ppe-daemon",
    about = "Planetary Particle Engine - Vehicle OS Simulator"
)]
struct Cli {
    /// Scenario to run
    #[arg(short, long, default_value = "city-drive")]
    scenario: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "warn")]
    log_level: String,

    /// Simulation tick rate in milliseconds
    #[arg(short, long, default_value_t = 10)]
    tick_rate: u64,

    /// Run headless (no TUI)
    #[arg(long, default_value_t = false)]
    headless: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup tracing
    let filter = EnvFilter::try_new(&cli.log_level).unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let scenario_kind = ScenarioKind::parse(&cli.scenario).unwrap_or_else(|| {
        eprintln!(
            "Unknown scenario '{}'. Available: {:?}",
            cli.scenario,
            ScenarioKind::all()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
        std::process::exit(1);
    });

    let tick_duration = Duration::from_millis(cli.tick_rate);

    // Create CAN bus
    let bus = VirtualCanBus::new(4096);

    // Create subsystems
    let bms_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 128);
    let (mut bms, bms_handles) = BatteryManagementSystem::new(BmsConfig::default(), bms_node);
    bms.init()?;

    let motor_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 128);
    let (mut motor, motor_handles) = MotorController::new(motor_node);
    motor.init()?;

    let thermal_node = bus.connect(CanFilter::Exact(well_known::EMERGENCY_STOP), 128);
    let (mut thermal, thermal_handles) = ThermalManagement::new(thermal_node);
    thermal.init()?;

    // Create OBD responder
    let obd_node = bus.connect(CanFilter::Exact(well_known::OBD_REQUEST), 64);
    let mut obd = ObdResponder::new(obd_node);

    // CAN monitor node
    let can_monitor = bus.connect(CanFilter::AcceptAll, 256);

    // Create physics
    let mut physics = VehiclePhysics::new(VehiclePhysicsConfig::default());
    let physics_handles = PhysicsHandles {
        bms_voltage: bms_handles.pack_voltage,
        bms_current: bms_handles.pack_current,
        bms_temperature: bms_handles.pack_temperature,
        motor_rpm: motor_handles.rpm,
        motor_torque: motor_handles.torque,
        motor_temperature: motor_handles.temperature,
        motor_throttle: motor_handles.throttle,
        coolant_temp: thermal_handles.coolant_temp,
        ambient_temp: thermal_handles.ambient_temp,
    };

    // Create vehicle FSM
    let mut fsm = VehicleFsm::new();
    fsm.on_event(&VehicleEvent::KeyToAccessory);
    fsm.on_event(&VehicleEvent::KeyToStart);
    fsm.on_event(&VehicleEvent::GearShift(Gear::Drive));
    fsm.on_event(&VehicleEvent::ThrottleApplied(0.1));

    // Create scenario
    let mut scenario = Scenario::new(scenario_kind);

    // Create DTC manager
    let mut dtc_manager = DtcManager::new();

    // Dashboard state
    let dash_state = Arc::new(Mutex::new(DashboardState::new()));
    dash_state.lock().unwrap().current_scenario = scenario_kind;

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // Manual throttle/brake override
    let manual_throttle = Arc::new(std::sync::atomic::AtomicU64::new(f64::NAN.to_bits()));
    let manual_throttle_clone = manual_throttle.clone();

    let paused = Arc::new(AtomicBool::new(false));
    let paused_clone = paused.clone();

    let dash_clone = dash_state.clone();

    // Simulation thread
    let sim_handle = std::thread::Builder::new()
        .name("simulation".into())
        .spawn(move || {
            let mut elapsed = Duration::ZERO;

            while running_clone.load(Ordering::Relaxed) {
                if paused_clone.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }

                // Get throttle/brake from scenario or manual override
                let manual = f64::from_bits(manual_throttle_clone.load(Ordering::Relaxed));
                let (throttle, brake) = if manual.is_nan() {
                    scenario.update(elapsed)
                } else {
                    (manual.clamp(0.0, 1.0), 0.0)
                };

                physics.set_throttle(throttle);
                physics.set_brake(brake);

                // Step physics
                physics.step(tick_duration);

                // Write to sensors
                physics.update_sensors(&physics_handles);

                // Tick subsystems
                let _ = bms.tick(tick_duration);
                let _ = motor.tick(tick_duration);
                let _ = thermal.tick(tick_duration);

                // Aggregate DTCs
                let mut all_dtcs = Vec::new();
                all_dtcs.extend(bms.active_dtcs());
                all_dtcs.extend(motor.active_dtcs());
                all_dtcs.extend(thermal.active_dtcs());
                dtc_manager.update(all_dtcs);

                // Update OBD responder
                obd.update_live_data(ObdLiveData {
                    rpm: physics.motor_rpm(),
                    speed_kmh: physics.speed_kmh(),
                    coolant_temp_c: physics.coolant_temp(),
                    soc_pct: physics.soc() * 100.0,
                    battery_voltage: 0.0,
                    battery_current: 0.0,
                });
                obd.process(&dtc_manager);

                // Update dashboard state
                if let Ok(mut ds) = dash_clone.lock() {
                    ds.vehicle_state = fsm.state();
                    ds.gear = fsm.gear();
                    ds.uptime_secs = elapsed.as_secs_f64();

                    ds.bms_state = bms.state();
                    ds.soc_pct = bms.soc().value();
                    ds.pack_voltage = physics.speed_kmh(); // placeholder
                    ds.pack_current = 0.0;
                    ds.pack_temperature = physics.coolant_temp();

                    ds.motor_state = motor.state();
                    ds.motor_rpm = physics.motor_rpm();
                    ds.motor_torque = 0.0;
                    ds.motor_temperature = physics.motor_temp();

                    ds.coolant_temp = physics.coolant_temp();
                    ds.fan_speed_pct = thermal.fan_speed().value();
                    ds.cooling_state = format!("{}", thermal.state());

                    ds.speed_kmh = physics.speed_kmh();
                    ds.throttle_pct = throttle * 100.0;
                    ds.brake_pct = brake * 100.0;
                    ds.power_kw = physics.power_kw();

                    ds.active_dtcs = dtc_manager.active().to_vec();

                    // Drain CAN monitor
                    for frame in can_monitor.drain() {
                        ds.push_can_frame(frame);
                    }
                }

                elapsed += tick_duration;
                std::thread::sleep(tick_duration);
            }
        })?;

    if cli.headless {
        // Headless mode: just wait for Ctrl+C
        let r = running.clone();
        ctrlc_handler(r);
        sim_handle.join().expect("sim thread panicked");
    } else {
        // TUI mode
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        loop {
            if !running.load(Ordering::Relaxed) {
                break;
            }

            // Draw
            {
                let ds = dash_state.lock().unwrap();
                terminal.draw(|f| draw_dashboard(f, &ds))?;
            }

            // Handle input
            if event::poll(Duration::from_millis(33))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            running.store(false, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            let was_paused = paused.load(Ordering::Relaxed);
                            paused.store(!was_paused, Ordering::Relaxed);
                            dash_state.lock().unwrap().paused = !was_paused;
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            let current = f64::from_bits(manual_throttle.load(Ordering::Relaxed));
                            let new_val = if current.is_nan() {
                                0.1
                            } else {
                                (current + 0.1).min(1.0)
                            };
                            manual_throttle.store(new_val.to_bits(), Ordering::Relaxed);
                        }
                        KeyCode::Char('-') => {
                            let current = f64::from_bits(manual_throttle.load(Ordering::Relaxed));
                            let new_val = if current.is_nan() {
                                0.0
                            } else {
                                (current - 0.1).max(0.0)
                            };
                            manual_throttle.store(new_val.to_bits(), Ordering::Relaxed);
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            // Cycle through scenarios
                            let current = dash_state.lock().unwrap().current_scenario;
                            let all = ScenarioKind::all();
                            let idx = all.iter().position(|s| *s == current).unwrap_or(0);
                            let next = all[(idx + 1) % all.len()];
                            dash_state.lock().unwrap().current_scenario = next;
                            // Reset manual throttle to use scenario
                            manual_throttle.store(f64::NAN.to_bits(), Ordering::Relaxed);
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            // Clear DTCs display
                            dash_state.lock().unwrap().active_dtcs.clear();
                        }
                        _ => {}
                    }
                }
            }
        }

        // Cleanup
        running.store(false, Ordering::Relaxed);
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        let _ = sim_handle.join();
    }

    Ok(())
}

fn ctrlc_handler(running: Arc<AtomicBool>) {
    ctrlc_wait(running);
}

fn ctrlc_wait(running: Arc<AtomicBool>) {
    while running.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
    }
}
