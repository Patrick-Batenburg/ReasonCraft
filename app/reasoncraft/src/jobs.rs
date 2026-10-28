//! Work on worker threads, so the window keeps drawing while it runs:
//! exports ([`crate::export::ExportJobs`]), inserted images
//! ([`crate::canvas::images::ImageJobs`]) and spelling dictionaries
//! ([`crate::spell::SpellService`]). The frame loop polls each job
//! (it never blocks), keeps repainting while one runs, and the status bar
//! shows one busy chip for the exports and images (a
//! loading dictionary shows in the spelling panel instead).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
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

/// Holds workers back until it is released (tests only): a test keeps a
/// job running while frames go by, then lets it finish, with no timing
/// involved. Clones share the hold.
#[derive(Clone, Debug, Default)]
pub struct Hold(Arc<(Mutex<bool>, Condvar)>);

impl Hold {
    /// The longest a worker waits for [`Self::release`], so a test that
    /// fails before releasing never leaves a thread waiting for ever.
    pub const MAX_WAIT: Duration = Duration::from_secs(120);

    /// Lets every worker waiting on this hold (and every later one) go.
    pub fn release(&self) {
        let (open, cv) = &*self.0;
        *open.lock().unwrap_or_else(PoisonError::into_inner) = true;
        cv.notify_all();
    }

    /// Waits until the hold is released (at most [`Self::MAX_WAIT`]).
    pub fn wait(&self) {
        let (open, cv) = &*self.0;
        let guard = open.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = cv
            .wait_timeout_while(guard, Self::MAX_WAIT, |open| !*open)
            .unwrap_or_else(PoisonError::into_inner);
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
    /// Runs `work` on a new thread, once `hold` is released when one is
    /// given (tests use it to keep a job running while frames go by).
    pub fn spawn(hold: Option<Hold>, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Some(hold) = hold {
                hold.wait();
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
    fn a_held_job_runs_once_released() {
        let hold = Hold::default();
        let job = Job::spawn(Some(hold.clone()), || 7);
        std::thread::sleep(Duration::from_millis(20));
        assert!(matches!(job.poll(), JobState::Running), "held");
        hold.release();
        let start = std::time::Instant::now();
        loop {
            match job.poll() {
                JobState::Done(n) => break assert_eq!(n, 7),
                JobState::Running => {
                    assert!(start.elapsed() < Duration::from_secs(10), "never ran");
                    std::thread::sleep(Duration::from_millis(1));
                }
                JobState::Lost => panic!("lost"),
            }
        }
        // A released hold lets later jobs go at once.
        hold.wait();
    }

    #[test]
    fn job_ids_are_unique() {
        let (a, b) = (JobId::next(), JobId::next());
        assert_ne!(a, b);
    }
}
