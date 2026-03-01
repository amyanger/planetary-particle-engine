use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Simulation clock that tracks elapsed time in microseconds.
/// Thread-safe via atomic operations.
#[derive(Debug, Clone)]
pub struct SimClock {
    micros: Arc<AtomicU64>,
}

impl SimClock {
    pub fn new() -> Self {
        Self {
            micros: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Advance the clock by the given duration.
    pub fn advance(&self, dt: Duration) {
        self.micros
            .fetch_add(dt.as_micros() as u64, Ordering::Relaxed);
    }

    /// Current elapsed time as Duration.
    pub fn elapsed(&self) -> Duration {
        Duration::from_micros(self.micros.load(Ordering::Relaxed))
    }

    /// Current elapsed time in seconds (f64).
    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// Reset the clock to zero.
    pub fn reset(&self) {
        self.micros.store(0, Ordering::Relaxed);
    }
}

impl Default for SimClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_advance() {
        let clock = SimClock::new();
        assert_eq!(clock.elapsed(), Duration::ZERO);

        clock.advance(Duration::from_millis(10));
        assert_eq!(clock.elapsed(), Duration::from_millis(10));

        clock.advance(Duration::from_millis(5));
        assert_eq!(clock.elapsed(), Duration::from_millis(15));
    }

    #[test]
    fn clock_reset() {
        let clock = SimClock::new();
        clock.advance(Duration::from_secs(1));
        clock.reset();
        assert_eq!(clock.elapsed(), Duration::ZERO);
    }

    #[test]
    fn clock_clone_shares_state() {
        let clock1 = SimClock::new();
        let clock2 = clock1.clone();
        clock1.advance(Duration::from_millis(100));
        assert_eq!(clock2.elapsed(), Duration::from_millis(100));
    }
}
