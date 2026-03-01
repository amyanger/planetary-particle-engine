use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotorState {
    Disabled,
    Initializing,
    Ready,
    Running,
    Regenerating,
    Derating,
    Fault,
}

impl fmt::Display for MotorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "DISABLED"),
            Self::Initializing => write!(f, "INITIALIZING"),
            Self::Ready => write!(f, "READY"),
            Self::Running => write!(f, "RUNNING"),
            Self::Regenerating => write!(f, "REGENERATING"),
            Self::Derating => write!(f, "DERATING"),
            Self::Fault => write!(f, "FAULT"),
        }
    }
}
