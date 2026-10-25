//! Exports on a worker thread, so a large map never freezes the window.
//!
//! The UI thread builds what needs egui (the [`Scene`], which measures
//! text) and hands it over with copies of everything else (assets, the
//! document); the worker writes the SVG, rasterises, makes the PDF and
//! writes the file. Editing during an export cannot change what is
//! exported. The frame loop polls [`ExportJobs::finished`] and repaints
//! while [`ExportJobs::busy`].

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reasoncraft_core::essay_markdown;
use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::outline_md::{outline_markdown, outline_plain_text_lang};
use reasoncraft_core::{Assets, Document};

use super::docx::{essay_docx, outline_docx};
use super::{
    ClipboardSink, ExportError, ExportFormat, ExportOptions, PageSetup, PngOptions, SvgOptions,
    bundled_font_family, copy_scene_to_clipboard, scene_to_png, scene_to_svg, shared_fontdb,
    svg_to_pdf, write_print_file,
};
use crate::scene::Scene;
use crate::theme::Tokens;

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

/// A clipboard the worker can reach.
pub type SharedClipboard = Arc<Mutex<dyn ClipboardSink + Send>>;

/// A snapshot of what an export draws from.
pub enum Snapshot {
    /// A picture of the map: its scene (drawn at zoom 1), the images it
    /// shows and the colours it is drawn in.
    Scene {
        scene: Scene,
        assets: Assets,
        tokens: Tokens,
    },
    /// The document, for text exports, with the outline's switches.
    Doc {
        doc: Box<Document>,
        outline: OutlineOptions,
    },
}

/// Where an export goes.
pub enum Target {
    File(PathBuf),
    Clipboard(SharedClipboard),
    /// A print file in this folder, opened afterwards by the UI thread.
    Print(PathBuf),
}

/// How an export ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The file was written.
    Saved(PathBuf),
    /// The image is on the clipboard.
    Copied,
    /// The print file was written; it still has to be opened.
    ReadyToPrint(PathBuf),
}

/// One export, ready to run on a worker.
pub struct ExportWork {
    pub format: ExportFormat,
    pub options: ExportOptions,
    pub page: PageSetup,
    pub snapshot: Snapshot,
    pub target: Target,
}

impl ExportWork {
    /// Makes the export and writes (or copies) it.
    pub fn run(self) -> Result<Outcome, ExportError> {
        let fonts = shared_fontdb();
        let bytes = match (&self.snapshot, self.format) {
            (
                Snapshot::Scene {
                    scene,
                    assets,
                    tokens,
                },
                format,
            ) => {
                if let Target::Clipboard(sink) = &self.target {
                    let opts = png_options(&self.options);
                    // A copy that panicked earlier leaves nothing half done in
                    // the sink, so a poisoned lock is safe to take over.
                    let mut sink = sink.lock().unwrap_or_else(|e| e.into_inner());
                    copy_scene_to_clipboard(scene, assets, tokens, &opts, &fonts, &mut *sink)?;
                    return Ok(Outcome::Copied);
                }
                let svg = |outlines| {
                    let opts = SvgOptions {
                        text_as_paths: outlines,
                        ..SvgOptions::new(tokens)
                    };
                    scene_to_svg(scene, assets, &opts, &bundled_font_family())
                };
                match format {
                    ExportFormat::Png => {
                        scene_to_png(scene, assets, tokens, &png_options(&self.options), &fonts)?
                    }
                    ExportFormat::Svg => svg(self.options.svg_outlines).into_bytes(),
                    // The PDF embeds the fonts and keeps text selectable.
                    ExportFormat::Pdf => svg_to_pdf(&svg(false), &self.page, &fonts)?,
                    other => return Err(ExportError::Unsupported(other)),
                }
            }
            (Snapshot::Doc { doc, outline }, format) => {
                let lines = build_outline(doc, outline);
                let lang = doc.meta.language.as_str();
                match format {
                    ExportFormat::OutlineMarkdown => {
                        outline_markdown(&lines, outline, lang).into_bytes()
                    }
                    ExportFormat::OutlineText => {
                        outline_plain_text_lang(&lines, outline, lang).into_bytes()
                    }
                    ExportFormat::OutlineDocx => {
                        outline_docx(&lines, outline, lang, &doc.meta.author)?
                    }
                    ExportFormat::EssayMarkdown => essay_markdown(doc, lang).into_bytes(),
                    ExportFormat::EssayDocx => essay_docx(doc, lang)?,
                    other => return Err(ExportError::Unsupported(other)),
                }
            }
        };
        match self.target {
            Target::File(path) => {
                write_atomically(&path, &bytes)?;
                Ok(Outcome::Saved(path))
            }
            Target::Print(dir) => Ok(Outcome::ReadyToPrint(write_print_file(&bytes, &dir)?)),
            Target::Clipboard(_) => Err(ExportError::Unsupported(self.format)),
        }
    }
}

fn png_options(options: &ExportOptions) -> PngOptions {
    PngOptions {
        scale: options.scale(),
        transparent: options.transparent,
    }
}

/// Makes a new, empty temporary file next to `path`, named
/// `<name>.<pid>-<counter>.tmp`. `create_new` means a file that already
/// has that name (the user's, or another export's) is never touched.
fn create_temp(path: &Path) -> std::io::Result<(PathBuf, std::fs::File)> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let pid = std::process::id();
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{pid}-{n}.tmp"));
        let tmp = PathBuf::from(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => return Ok((tmp, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
}

/// Writes `bytes` to `path` through a temporary file next to it, so an
/// existing file is replaced whole or not at all.
pub(super) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    let (tmp, mut file) = create_temp(path)?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| {
            drop(file);
            std::fs::rename(&tmp, path)
        });
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(result?)
}

/// A message about the last export, shown in the status bar.
#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    /// A file was written (with "Open folder").
    Saved(PathBuf),
    /// The image is on the clipboard.
    Copied,
}

/// The exports running in the background, where they may go, and what
/// the status bar says about the last one.
pub struct ExportJobs {
    running: Vec<Job<Result<Outcome, ExportError>>>,
    /// Wait this long before each export starts (tests only).
    pub delay: Option<Duration>,
    /// Where "Copy as image" puts the image: the system clipboard, or a
    /// fake in tests.
    pub clipboard: SharedClipboard,
    /// Where print files are written.
    pub print_dir: PathBuf,
    pub notice: Option<Notice>,
    /// Export errors waiting for the message on screen to close.
    pub waiting_errors: VecDeque<ExportError>,
}

impl ExportJobs {
    pub fn new(clipboard: SharedClipboard, print_dir: PathBuf) -> Self {
        Self {
            running: Vec::new(),
            delay: None,
            clipboard,
            print_dir,
            notice: None,
            waiting_errors: VecDeque::new(),
        }
    }

    /// Whether an export is still running.
    pub fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// Starts `work` on a worker thread and clears the last notice.
    pub fn start(&mut self, work: ExportWork) {
        self.notice = None;
        self.running
            .push(Job::spawn(self.delay, move || work.run()));
    }

    /// The results of the exports that ended since the last call.
    pub fn finished(&mut self) -> Vec<Result<Outcome, ExportError>> {
        let mut done = Vec::new();
        self.running.retain(|job| match job.poll() {
            JobState::Running => true,
            JobState::Done(result) => {
                done.push(result);
                false
            }
            JobState::Lost => {
                done.push(Err(ExportError::Stopped));
                false
            }
        });
        done
    }
}
