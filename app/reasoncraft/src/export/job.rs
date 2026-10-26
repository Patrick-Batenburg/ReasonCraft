//! Exports on a worker thread, so a large map never freezes the window.
//!
//! The UI thread builds what needs egui (the [`Scene`], which measures
//! text) and hands it over with copies of everything else (assets, the
//! document); the worker writes the SVG, rasterises, makes the PDF and
//! writes the file. Editing during an export cannot change what is
//! exported. Each export is a [`Job`]; the frame loop polls
//! [`ExportJobs::finished`] and repaints while [`ExportJobs::busy`].

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::outline_md::{outline_markdown, outline_plain_text_lang};
use reasoncraft_core::{Assets, Document, to_argdown};
use reasoncraft_core::{essay_markdown, essay_plain_text};

use super::docx::{essay_docx, outline_docx};
use super::{
    ClipboardSink, ExportError, ExportFormat, ExportOptions, PageSetup, PngOptions, SvgOptions,
    bundled_font_family, copy_scene_to_clipboard, scene_to_png, scene_to_svg, shared_fontdb,
    svg_to_pdf, write_print_file,
};
use crate::jobs::{Job, JobState};
use crate::scene::Scene;
use crate::theme::Tokens;

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
    /// The file was written, with something to tell about it: the i18n
    /// key of a status note (e.g. images left out of an Argdown file).
    SavedWithNote(PathBuf, &'static str),
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
        self.run_gated(&WriteGate::default())
    }

    /// Like [`Self::run`], writing the file only while `gate` is open.
    fn run_gated(self, gate: &WriteGate) -> Result<Outcome, ExportError> {
        let fonts = shared_fontdb();
        let note = match (&self.snapshot, self.format) {
            (Snapshot::Doc { doc, .. }, ExportFormat::Argdown)
                if doc.boxes.values().any(|b| b.image.is_some()) =>
            {
                Some("export.argdown_no_images")
            }
            _ => None,
        };
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
                    ExportFormat::EssayText => essay_plain_text(doc, lang).into_bytes(),
                    ExportFormat::EssayDocx => essay_docx(doc, lang)?,
                    ExportFormat::Argdown => to_argdown(doc).into_bytes(),
                    other => return Err(ExportError::Unsupported(other)),
                }
            }
        };
        // Held until the file is complete, so a closed gate never leaves a
        // half-written file behind.
        let _writing = gate.enter()?;
        match self.target {
            Target::File(path) => {
                write_atomically(&path, &bytes)?;
                Ok(match note {
                    Some(key) => Outcome::SavedWithNote(path, key),
                    None => Outcome::Saved(path),
                })
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

/// Makes a new, empty file at the first of `candidates` that does not
/// exist yet. `create_new` means an existing file (or a link) with that
/// name is never opened, let alone overwritten. `private`: on Unix, only
/// the user may read the file (mode 0600).
pub(super) fn create_first_new(
    candidates: impl IntoIterator<Item = PathBuf>,
    private: bool,
) -> std::io::Result<(PathBuf, std::fs::File)> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    for path in candidates {
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("no free file name"))
}

/// Makes a new, empty temporary file next to `path`, named
/// `<name>.<pid>-<counter>.tmp`; a file that already has that name (the
/// user's, or another export's) is never touched.
fn create_temp(path: &Path) -> std::io::Result<(PathBuf, std::fs::File)> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let pid = std::process::id();
    let candidates = std::iter::repeat_with(|| {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{pid}-{n}.tmp"));
        PathBuf::from(name)
    });
    create_first_new(candidates, false)
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

/// Lets exports write their files until it is closed. Closing waits for a
/// write in progress, so afterwards no export is in the middle of writing
/// and none starts: the process can end without leaving a temporary file.
#[derive(Clone, Default)]
pub struct WriteGate(Arc<Mutex<bool>>);

impl WriteGate {
    /// Holds the gate open while the returned guard lives; an error once
    /// it is closed.
    fn enter(&self) -> Result<MutexGuard<'_, bool>, ExportError> {
        // A writer that panicked left nothing half done we could fix.
        let closed = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if *closed {
            return Err(ExportError::Stopped);
        }
        Ok(closed)
    }

    /// Closes the gate, after the write in progress (if any) finished.
    pub fn close(&self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = true;
    }
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
    /// Closed when the window quits while exports run (see [`Self::stop`]).
    gate: WriteGate,
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
            gate: WriteGate::default(),
        }
    }

    /// Whether an export is still running.
    pub fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// How many exports are running.
    pub fn running(&self) -> usize {
        self.running.len()
    }

    /// Starts `work` on a worker thread and clears the last notice.
    pub fn start(&mut self, work: ExportWork) {
        self.notice = None;
        let gate = self.gate.clone();
        self.running
            .push(Job::spawn(self.delay, move || work.run_gated(&gate)));
    }

    /// For quitting while exports run: no export writes a file from now
    /// on. Returns once a write in progress is complete, which is quick:
    /// the export is made in memory before its file is written.
    pub fn stop(&self) {
        self.gate.close();
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
