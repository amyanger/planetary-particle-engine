use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies a component/subsystem in the vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentId {
    Bms,
    Motor,
    Thermal,
    Vehicle,
    Scheduler,
    Diagnostics,
    Physics,
    EnerD,
}

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bms => write!(f, "BMS"),
            Self::Motor => write!(f, "MOTOR"),
            Self::Thermal => write!(f, "THERMAL"),
            Self::Vehicle => write!(f, "VEHICLE"),
            Self::Scheduler => write!(f, "SCHEDULER"),
            Self::Diagnostics => write!(f, "DIAGNOSTICS"),
            Self::Physics => write!(f, "PHYSICS"),
            Self::EnerD => write!(f, "ENER-D"),
        }
    }
}
