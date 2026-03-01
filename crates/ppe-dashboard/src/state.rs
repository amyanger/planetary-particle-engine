use std::collections::VecDeque;

use ppe_can::CanFrame;
use ppe_core::Dtc;
use ppe_sim::ScenarioKind;
use ppe_state::{BmsState, Gear, MotorState, VehicleState};

/// Shared state for the TUI dashboard, updated by the simulation loop.
pub struct DashboardState {
    // Vehicle
    pub vehicle_state: VehicleState,
    pub gear: Gear,
    pub uptime_secs: f64,
    pub paused: bool,

    // BMS
    pub bms_state: BmsState,
    pub soc_pct: f64,
    pub pack_voltage: f64,
    pub pack_current: f64,
    pub pack_temperature: f64,

    // Motor
    pub motor_state: MotorState,
    pub motor_rpm: f64,
    pub motor_torque: f64,
    pub motor_temperature: f64,

    // Thermal
    pub coolant_temp: f64,
    pub fan_speed_pct: f64,
    pub cooling_state: String,

    // Dynamics
    pub speed_kmh: f64,
    pub throttle_pct: f64,
    pub brake_pct: f64,
    pub power_kw: f64,

    // Diagnostics
    pub active_dtcs: Vec<Dtc>,

    // CAN monitor
    pub can_log: VecDeque<CanFrame>,
    pub can_log_max: usize,

    // Scenario
    pub current_scenario: ScenarioKind,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            vehicle_state: VehicleState::Off,
            gear: Gear::Park,
            uptime_secs: 0.0,
            paused: false,

            bms_state: BmsState::Standby,
            soc_pct: 0.0,
            pack_voltage: 0.0,
            pack_current: 0.0,
            pack_temperature: 0.0,

            motor_state: MotorState::Disabled,
            motor_rpm: 0.0,
            motor_torque: 0.0,
            motor_temperature: 0.0,

            coolant_temp: 0.0,
            fan_speed_pct: 0.0,
            cooling_state: "OFF".into(),

            speed_kmh: 0.0,
            throttle_pct: 0.0,
            brake_pct: 0.0,
            power_kw: 0.0,

            active_dtcs: Vec::new(),

            can_log: VecDeque::new(),
            can_log_max: 100,

            current_scenario: ScenarioKind::Idle,
        }
    }

    pub fn push_can_frame(&mut self, frame: CanFrame) {
        if self.can_log.len() >= self.can_log_max {
            self.can_log.pop_front();
        }
        self.can_log.push_back(frame);
    }
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}
