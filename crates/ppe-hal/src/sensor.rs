use ppe_core::PpeError;

/// Hardware abstraction for a sensor that reads a value of type T.
pub trait Sensor<T>: Send + Sync {
    /// Read the current sensor value.
    fn read(&self) -> Result<T, PpeError>;

    /// Human-readable sensor name.
    fn name(&self) -> &str;

    /// Whether the sensor is in a healthy state.
    fn is_healthy(&self) -> bool;
}
