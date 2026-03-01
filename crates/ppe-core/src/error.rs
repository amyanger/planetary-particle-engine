use thiserror::Error;

#[derive(Debug, Error)]
pub enum PpeError {
    #[error("CAN bus error: {0}")]
    CanBus(String),

    #[error("Sensor error: {0}")]
    Sensor(String),

    #[error("Actuator error: {0}")]
    Actuator(String),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Subsystem fault in {subsystem}: {detail}")]
    SubsystemFault { subsystem: String, detail: String },

    #[error("Watchdog timeout for task: {0}")]
    WatchdogTimeout(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Scheduler error: {0}")]
    Scheduler(String),
}
