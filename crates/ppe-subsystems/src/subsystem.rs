use ppe_core::Dtc;
use std::fmt;
use std::time::Duration;

/// Health status of a subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemHealth {
    Ok,
    Degraded,
    Fault,
}

impl fmt::Display for SubsystemHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Degraded => write!(f, "DEGRADED"),
            Self::Fault => write!(f, "FAULT"),
        }
    }
}

/// Trait for vehicle subsystems that run on the scheduler.
pub trait Subsystem: Send {
    /// Initialize the subsystem.
    fn init(&mut self) -> Result<(), ppe_core::PpeError>;

    /// Execute one tick of the subsystem logic.
    fn tick(&mut self, dt: Duration) -> Result<(), ppe_core::PpeError>;

    /// Return currently active diagnostic trouble codes.
    fn active_dtcs(&self) -> Vec<Dtc>;

    /// Gracefully shut down the subsystem.
    fn shutdown(&mut self) -> Result<(), ppe_core::PpeError>;

    /// Overall health assessment.
    fn health(&self) -> SubsystemHealth;

    /// Human-readable name.
    fn name(&self) -> &str;
}
