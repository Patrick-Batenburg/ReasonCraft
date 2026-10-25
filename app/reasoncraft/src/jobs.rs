//! Work on worker threads, so the window keeps drawing while it runs:
//! exports ([`crate::export::ExportJobs`]) and inserted images
//! ([`crate::canvas::images::ImageJobs`]). The frame loop polls each job
//! (it never blocks), keeps repainting while one runs, and the status bar
//! shows one busy chip for all of them.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

/// Names one job, unique for the life of the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JobId(u64);

impl JobId {
    /// A new id, never handed out before.
    pub fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Work running on its own thread, whose result is picked up by polling.
pub struct Job<T> {
    rx: Receiver<T>,
}

/// What a poll of a [`Job`] found.
pub enum JobState<T> {
    Running,
    Done(T),
    /// The worker stopped without a result (it panicked).
    Lost,
}

impl<T: Send + 'static> Job<T> {
    /// Runs `work` on a new thread, after `delay` when one is given (tests
    /// use it to keep a job running while frames go by).
    pub fn spawn(delay: Option<Duration>, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Some(delay) = delay {
                std::thread::sleep(delay);
            }
            // The receiver is gone only when the app closed; nothing to do.
            let _ = tx.send(work());
        });
        Self { rx }
    }

    /// The result, once the work is done. Never blocks.
    pub fn poll(&self) -> JobState<T> {
        match self.rx.try_recv() {
            Ok(out) => JobState::Done(out),
            Err(TryRecvError::Empty) => JobState::Running,
            Err(TryRecvError::Disconnected) => JobState::Lost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_ids_are_unique() {
        let (a, b) = (JobId::next(), JobId::next());
        assert_ne!(a, b);
    }
}
