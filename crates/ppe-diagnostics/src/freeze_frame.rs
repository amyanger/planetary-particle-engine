use ppe_core::Dtc;
use serde::{Deserialize, Serialize};

/// Snapshot of vehicle state when a DTC was set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeFrame {
    pub dtc: Dtc,
    pub timestamp_ms: u64,
    pub speed_kmh: f64,
    pub rpm: f64,
    pub soc_pct: f64,
    pub battery_voltage: f64,
    pub battery_current: f64,
    pub coolant_temp: f64,
    pub motor_temp: f64,
}

impl FreezeFrame {
    pub fn new(dtc: Dtc, timestamp_ms: u64) -> Self {
        Self {
            dtc,
            timestamp_ms,
            speed_kmh: 0.0,
            rpm: 0.0,
            soc_pct: 0.0,
            battery_voltage: 0.0,
            battery_current: 0.0,
            coolant_temp: 0.0,
            motor_temp: 0.0,
        }
    }
}
