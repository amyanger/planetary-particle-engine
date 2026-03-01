use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;
use tracing::{debug, warn};

use crate::ScheduledTask;

/// EDF-like scheduler that runs tasks at their specified periods.
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
    running: Arc<AtomicBool>,
    tick_count: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            running: Arc::new(AtomicBool::new(false)),
            tick_count: 0,
        }
    }

    /// Add a task to the scheduler.
    pub fn add_task(&mut self, task: ScheduledTask) {
        self.tasks.push(task);
    }

    /// Get a handle to stop the scheduler.
    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Run the scheduler loop until stopped.
    pub async fn run(&mut self) {
        self.running.store(true, Ordering::SeqCst);

        // Reset all deadlines
        let now = Instant::now();
        for task in &mut self.tasks {
            task.next_deadline = now + task.period;
        }

        while self.running.load(Ordering::SeqCst) {
            // Find the task with the earliest deadline (EDF)
            let now = Instant::now();

            // Find next task to run
            let next_idx = self
                .tasks
                .iter()
                .enumerate()
                .min_by_key(|(_, t)| t.next_deadline)
                .map(|(i, _)| i);

            let Some(idx) = next_idx else {
                tokio::time::sleep(Duration::from_millis(1)).await;
                continue;
            };

            let deadline = self.tasks[idx].next_deadline;
            if deadline > now {
                tokio::time::sleep_until(deadline).await;
            }

            let now = Instant::now();
            let task = &mut self.tasks[idx];

            // Check for deadline miss
            if now > task.next_deadline + task.period {
                task.deadline_misses += 1;
                warn!(
                    task = task.name,
                    misses = task.deadline_misses,
                    "deadline miss"
                );
            }

            let dt = task.period;
            if let Err(e) = (task.callback)(dt) {
                warn!(task = task.name, error = %e, "task error");
            }

            // Schedule next execution
            task.next_deadline = now + task.period;
            self.tick_count += 1;

            debug!(task = task.name, tick = self.tick_count, "task executed");
        }
    }

    /// Stop the scheduler.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
