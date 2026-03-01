use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnerDState {
    Dormant,
    SpinUp,
    Sustaining,
    Overdrive,
    Critical,
    Meltdown,
}

impl fmt::Display for EnerDState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dormant => write!(f, "DORMANT"),
            Self::SpinUp => write!(f, "SPIN_UP"),
            Self::Sustaining => write!(f, "SUSTAINING"),
            Self::Overdrive => write!(f, "OVERDRIVE"),
            Self::Critical => write!(f, "CRITICAL"),
            Self::Meltdown => write!(f, "MELTDOWN"),
        }
    }
}
