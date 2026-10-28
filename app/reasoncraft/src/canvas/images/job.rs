//! Inserted images are read, decoded and re-encoded on a worker thread, one
//! at a time (so several large photos never take memory together), and a
//! large photo never freezes the window. The box shows "Preparing
//! image…" meanwhile (see [`super::preparing`]); the result goes into the
//! document through `Action::ImagePrepared`.

use std::path::PathBuf;

use reasoncraft_core::NodeId;

use super::prepare::{FailureKind, ImageFailure, PreparedImage, file_label, prepare_file};
use crate::jobs::{Hold, Job, JobId, JobState, Queue};

/// What preparing an image gave.
pub type ImageResult = Result<PreparedImage, ImageFailure>;

/// A finished image job: for which box, and what it gave.
#[derive(Debug)]
pub struct Prepared {
    pub job: JobId,
    pub node: NodeId,
    pub result: ImageResult,
}

struct Running {
    id: JobId,
    node: NodeId,
    file: String,
    job: Job<ImageResult>,
}

/// The images being prepared, for all tabs. A tab knows which of them are
/// its own (`DocTab::pending_images`).
#[derive(Default)]
pub struct ImageJobs {
    running: Vec<Running>,
    /// Results held back until their tab's edit session ends, so an image
    /// never lands in the middle of someone's typing.
    pub waiting: Vec<Prepared>,
    /// Hold each image back until released before it is read (tests only).
    pub hold: Option<Hold>,
    queue: Queue,
}

impl ImageJobs {
    /// Starts reading and preparing the image at `path` for box `node`.
    pub fn start(&mut self, node: NodeId, path: PathBuf) -> JobId {
        let id = JobId::next();
        let file = file_label(&path);
        let job = (self.queue).push(self.hold.clone(), move || prepare_file(&path));
        self.running.push(Running {
            id,
            node,
            file,
            job,
        });
        id
    }

    /// Whether an image is still being prepared.
    pub fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// How many images are being prepared.
    pub fn running(&self) -> usize {
        self.running.len()
    }

    /// The jobs that ended since the last call. A worker that died gives
    /// [`FailureKind::Stopped`].
    pub fn finished(&mut self) -> Vec<Prepared> {
        let mut done = Vec::new();
        self.running.retain_mut(|r| {
            let result = match r.job.poll() {
                JobState::Running => return true,
                JobState::Done(result) => result,
                JobState::Lost => Err(ImageFailure {
                    file: std::mem::take(&mut r.file),
                    kind: FailureKind::Stopped,
                }),
            };
            done.push(Prepared {
                job: r.id,
                node: r.node,
                result,
            });
            false
        });
        done
    }
}
