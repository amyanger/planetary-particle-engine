use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BmsState {
    Standby,
    Precharging,
    Active,
    Charging,
    Balancing,
    Fault,
    SafeState,
}

impl fmt::Display for BmsState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standby => write!(f, "STANDBY"),
            Self::Precharging => write!(f, "PRECHARGING"),
            Self::Active => write!(f, "ACTIVE"),
            Self::Charging => write!(f, "CHARGING"),
            Self::Balancing => write!(f, "BALANCING"),
            Self::Fault => write!(f, "FAULT"),
            Self::SafeState => write!(f, "SAFE_STATE"),
        }
    }
}
