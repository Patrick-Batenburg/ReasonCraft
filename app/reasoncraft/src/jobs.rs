//! Work on worker threads, so the window keeps drawing while it runs:
//! exports ([`crate::export::ExportJobs`]), inserted images
//! ([`crate::canvas::images::ImageJobs`]), saves
//! ([`crate::file_ops::SaveJobs`]) and spelling dictionaries
//! ([`crate::spell::SpellService`]). The frame loop polls each job
//! (it never blocks), keeps repainting while one runs, and the status bar
//! shows one busy chip for the exports and images and "Saving…" for saves
//! (a loading dictionary shows in the spelling panel instead). Only a
//! close waits for the saves on the UI thread, at most
//! [`crate::file_ops::SAVE_CLOSE_WAIT`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
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

/// A task for a [`Queue`]'s worker.
type Task = Box<dyn FnOnce() + Send>;

/// One worker thread that runs its jobs one after another, in the order
/// they came, so heavy jobs (decoding large images) never run side by side
/// and add up. The thread starts with the first job, and again if it ever
/// stopped.
#[derive(Default)]
pub struct Queue {
    tx: Option<Sender<Task>>,
}

impl Queue {
    /// Queues `work`, to run once the jobs before it are done and `hold`
    /// (when given) is released.
    pub fn push<T: Send + 'static>(
        &mut self,
        hold: Option<Hold>,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Job<T> {
        let (tx, rx) = mpsc::channel();
        let mut task: Task = Box::new(move || {
            if let Some(hold) = hold {
                hold.wait();
            }
            let _ = tx.send(work());
        });
        loop {
            let worker = self.tx.get_or_insert_with(start_worker);
            match worker.send(task) {
                Ok(()) => return Job { rx },
                // The worker is gone: start another.
                Err(mpsc::SendError(back)) => {
                    task = back;
                    self.tx = None;
                }
            }
        }
    }
}

/// Starts a worker that runs the tasks it is sent until the sender is
/// dropped. A task that panics loses only its own result (its job reads
/// [`JobState::Lost`]); the worker goes on with the next.
fn start_worker() -> Sender<Task> {
    let (tx, rx) = mpsc::channel::<Task>();
    std::thread::spawn(move || {
        for task in rx {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task));
        }
    });
    tx
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

    /// Polls `job` until it ends (at most ten seconds).
    fn outcome<T: Send + 'static>(job: &Job<T>) -> Option<T> {
        let start = std::time::Instant::now();
        loop {
            match job.poll() {
                JobState::Done(v) => return Some(v),
                JobState::Lost => return None,
                JobState::Running => {
                    assert!(start.elapsed() < Duration::from_secs(10), "never ran");
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    #[test]
    fn a_queue_runs_its_jobs_one_at_a_time_in_order() {
        let mut queue = Queue::default();
        let hold = Hold::default();
        let first = queue.push(Some(hold.clone()), || std::thread::current().id());
        let second = queue.push(None, || std::thread::current().id());
        std::thread::sleep(Duration::from_millis(20));
        assert!(matches!(second.poll(), JobState::Running), "waits its turn");
        hold.release();
        let (a, b) = (outcome(&first).unwrap(), outcome(&second).unwrap());
        assert_eq!(a, b, "one worker thread");
        // A job that panics is lost; the next one still runs.
        let lost = queue.push(None, || -> u8 { panic!("job failed") });
        let next = queue.push(None, || 9);
        assert_eq!(outcome(&lost), None);
        assert_eq!(outcome(&next), Some(9));
    }

    #[test]
    fn job_ids_are_unique() {
        let (a, b) = (JobId::next(), JobId::next());
        assert_ne!(a, b);
    }
}
