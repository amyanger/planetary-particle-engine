use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gear {
    Park,
    Reverse,
    Neutral,
    Drive,
}

impl fmt::Display for Gear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Park => write!(f, "P"),
            Self::Reverse => write!(f, "R"),
            Self::Neutral => write!(f, "N"),
            Self::Drive => write!(f, "D"),
        }
    }
}
