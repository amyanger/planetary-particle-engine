use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ppe_core::PpeError;
use rand::Rng;

use crate::Sensor;

/// Configuration for sensor noise simulation.
#[derive(Debug, Clone)]
pub struct NoiseModel {
    /// Standard deviation of gaussian noise.
    pub stddev: f64,
    /// Linear drift per read (accumulated).
    pub drift_per_read: f64,
    /// Probability of a random spike [0.0, 1.0].
    pub spike_probability: f64,
    /// Magnitude of spikes when they occur.
    pub spike_magnitude: f64,
}

impl Default for NoiseModel {
    fn default() -> Self {
        Self {
            stddev: 0.0,
            drift_per_read: 0.0,
            spike_probability: 0.0,
            spike_magnitude: 0.0,
        }
    }
}

/// Handle for the physics layer to write true sensor values.
/// Uses atomic f64 (via u64 bit-reinterpretation) for lock-free writes.
#[derive(Debug, Clone)]
pub struct SensorHandle {
    value_bits: Arc<AtomicU64>,
}

impl SensorHandle {
    fn new(initial: f64) -> Self {
        Self {
            value_bits: Arc::new(AtomicU64::new(initial.to_bits())),
        }
    }

    /// Set the true sensor value (called by physics simulation).
    pub fn set(&self, value: f64) {
        self.value_bits.store(value.to_bits(), Ordering::Relaxed);
    }

    fn get(&self) -> f64 {
        f64::from_bits(self.value_bits.load(Ordering::Relaxed))
    }
}

/// A mock sensor that reads from an atomic value and applies optional noise.
pub struct MockSensor {
    name: String,
    handle: SensorHandle,
    noise: NoiseModel,
    healthy: bool,
    drift_accumulator: std::sync::Mutex<f64>,
}

impl MockSensor {
    /// Create a new MockSensor and its corresponding SensorHandle.
    pub fn new(
        name: impl Into<String>,
        initial_value: f64,
        noise: NoiseModel,
    ) -> (Self, SensorHandle) {
        let handle = SensorHandle::new(initial_value);
        let sensor = Self {
            name: name.into(),
            handle: handle.clone(),
            noise,
            healthy: true,
            drift_accumulator: std::sync::Mutex::new(0.0),
        };
        (sensor, handle)
    }

    /// Create a sensor with no noise.
    pub fn new_clean(name: impl Into<String>, initial_value: f64) -> (Self, SensorHandle) {
        Self::new(name, initial_value, NoiseModel::default())
    }

    /// Mark the sensor as unhealthy (simulates hardware failure).
    pub fn set_healthy(&mut self, healthy: bool) {
        self.healthy = healthy;
    }
}

impl Sensor<f64> for MockSensor {
    fn read(&self) -> Result<f64, PpeError> {
        if !self.healthy {
            return Err(PpeError::Sensor(format!("{}: sensor fault", self.name)));
        }

        let mut value = self.handle.get();

        // Apply noise
        if self.noise.stddev > 0.0 {
            let mut rng = rand::thread_rng();
            let noise: f64 = rng.gen::<f64>() * 2.0 - 1.0; // Simple uniform approximation
            value += noise * self.noise.stddev;
        }

        // Apply drift
        if self.noise.drift_per_read != 0.0 {
            let mut drift = self.drift_accumulator.lock().unwrap();
            *drift += self.noise.drift_per_read;
            value += *drift;
        }

        // Apply spike
        if self.noise.spike_probability > 0.0 {
            let mut rng = rand::thread_rng();
            if rng.gen::<f64>() < self.noise.spike_probability {
                let direction: f64 = if rng.gen::<bool>() { 1.0 } else { -1.0 };
                value += direction * self.noise.spike_magnitude;
            }
        }

        Ok(value)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_healthy(&self) -> bool {
        self.healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_sensor_clean_read() {
        let (sensor, handle) = MockSensor::new_clean("test_voltage", 3.7);
        assert_eq!(sensor.read().unwrap(), 3.7);

        handle.set(4.2);
        assert_eq!(sensor.read().unwrap(), 4.2);
    }

    #[test]
    fn mock_sensor_unhealthy() {
        let (mut sensor, _handle) = MockSensor::new_clean("test_voltage", 3.7);
        sensor.set_healthy(false);
        assert!(sensor.read().is_err());
        assert!(!sensor.is_healthy());
    }

    #[test]
    fn sensor_handle_atomic_sharing() {
        let (sensor, handle) = MockSensor::new_clean("shared", 0.0);
        let handle2 = handle.clone();

        handle.set(42.0);
        assert_eq!(sensor.read().unwrap(), 42.0);

        handle2.set(99.0);
        assert_eq!(sensor.read().unwrap(), 99.0);
    }

    #[test]
    fn mock_sensor_with_noise() {
        let noise = NoiseModel {
            stddev: 0.1,
            ..Default::default()
        };
        let (sensor, _handle) = MockSensor::new("noisy", 10.0, noise);

        // Read multiple times, values should vary slightly
        let readings: Vec<f64> = (0..100).map(|_| sensor.read().unwrap()).collect();
        let min = readings.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = readings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            max - min > 0.0,
            "noisy sensor should produce varied readings"
        );
    }
}
