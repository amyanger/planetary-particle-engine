use serde::{Deserialize, Serialize};

/// Events that drive vehicle state transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VehicleEvent {
    KeyToAccessory,
    KeyToStart,
    KeyOff,
    ThrottleApplied(f64),
    ThrottleReleased,
    BrakeApplied(f64),
    BrakeReleased,
    GearShift(super::Gear),
    ChargerConnected,
    ChargerDisconnected,
    ChargingComplete,
    FaultDetected(String),
    FaultCleared,
    EmergencyStop,
    EmergencyReset,
    SystemsReady,
}
