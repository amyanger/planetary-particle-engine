use ppe_core::PpeError;

/// Hardware abstraction for an actuator that accepts commands.
pub trait Actuator<Command>: Send + Sync {
    /// Send a command to the actuator.
    fn command(&mut self, cmd: Command) -> Result<(), PpeError>;

    /// Human-readable actuator name.
    fn name(&self) -> &str;

    /// Whether the actuator is in a healthy state.
    fn is_healthy(&self) -> bool;
}
