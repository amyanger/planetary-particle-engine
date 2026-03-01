use std::time::Duration;

/// A task that runs periodically on the scheduler.
pub struct ScheduledTask {
    pub id: u32,
    pub name: String,
    pub period: Duration,
    pub callback: Box<dyn FnMut(Duration) -> Result<(), ppe_core::PpeError> + Send>,
    pub(crate) next_deadline: tokio::time::Instant,
    pub(crate) deadline_misses: u64,
}

impl ScheduledTask {
    pub fn new(
        id: u32,
        name: impl Into<String>,
        period: Duration,
        callback: impl FnMut(Duration) -> Result<(), ppe_core::PpeError> + Send + 'static,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            period,
            callback: Box::new(callback),
            next_deadline: tokio::time::Instant::now() + period,
            deadline_misses: 0,
        }
    }

    pub fn deadline_misses(&self) -> u64 {
        self.deadline_misses
    }
}
