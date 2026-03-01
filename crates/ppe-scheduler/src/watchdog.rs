use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tracing::error;

/// Software watchdog timer.
/// Must be kicked periodically or it triggers a callback.
pub struct Watchdog {
    name: String,
    timeout: Duration,
    last_kick: Arc<AtomicU64>,
    triggered: bool,
}

impl Watchdog {
    pub fn new(name: impl Into<String>, timeout: Duration) -> Self {
        Self {
            name: name.into(),
            timeout,
            last_kick: Arc::new(AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            )),
            triggered: false,
        }
    }

    /// Get a handle to kick the watchdog from another thread.
    pub fn kick_handle(&self) -> WatchdogKicker {
        WatchdogKicker {
            last_kick: self.last_kick.clone(),
        }
    }

    /// Kick the watchdog (reset timer).
    pub fn kick(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_kick.store(now, Ordering::Relaxed);
    }

    /// Check if the watchdog has timed out.
    pub fn check(&mut self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = self.last_kick.load(Ordering::Relaxed);
        let elapsed = Duration::from_millis(now.saturating_sub(last));

        if elapsed > self.timeout && !self.triggered {
            error!(
                watchdog = self.name,
                elapsed_ms = elapsed.as_millis(),
                "watchdog timeout"
            );
            self.triggered = true;
            return true;
        }
        false
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    pub fn reset(&mut self) {
        self.triggered = false;
        self.kick();
    }
}

/// Handle to kick a watchdog from another context.
#[derive(Clone)]
pub struct WatchdogKicker {
    last_kick: Arc<AtomicU64>,
}

impl WatchdogKicker {
    pub fn kick(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_kick.store(now, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_no_timeout_when_kicked() {
        let mut wd = Watchdog::new("test", Duration::from_millis(100));
        wd.kick();
        std::thread::sleep(Duration::from_millis(50));
        assert!(!wd.check());
    }

    #[test]
    fn watchdog_triggers_on_timeout() {
        let mut wd = Watchdog::new("test", Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(100));
        assert!(wd.check());
        assert!(wd.is_triggered());
    }

    #[test]
    fn watchdog_reset() {
        let mut wd = Watchdog::new("test", Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(100));
        wd.check();
        assert!(wd.is_triggered());
        wd.reset();
        assert!(!wd.is_triggered());
    }
}
