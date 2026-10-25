//! Exports on a worker thread, so a large map never freezes the window.
//!
//! The UI thread builds what needs egui (the [`Scene`], which measures
//! text) and hands it over with copies of everything else (assets, the
//! document); the worker writes the SVG, rasterises, makes the PDF and
//! writes the file. Editing during an export cannot change what is
//! exported. The frame loop polls [`ExportJobs::finished`] and repaints
//! while [`ExportJobs::busy`].

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::outline_md::{outline_markdown, outline_plain_text_lang};
use reasoncraft_core::{Assets, Document};

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
                    let mut sink = sink
                        .lock()
                        .map_err(|_| ExportError::Clipboard("clipboard lock poisoned".into()))?;
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
                    other => return Err(unavailable(other)),
                }
            }
            (Snapshot::Doc { doc, outline }, format) => {
                let lines = build_outline(doc, outline);
                let lang = doc.meta.language.as_str();
                match format {
                    ExportFormat::OutlineMarkdown => outline_markdown(&lines, outline, lang),
                    ExportFormat::OutlineText => outline_plain_text_lang(&lines, outline, lang),
                    other => return Err(unavailable(other)),
                }
                .into_bytes()
            }
        };
        match self.target {
            Target::File(path) => {
                std::fs::write(&path, bytes)?;
                Ok(Outcome::Saved(path))
            }
            Target::Print(dir) => Ok(Outcome::ReadyToPrint(write_print_file(&bytes, &dir)?)),
            Target::Clipboard(_) => Err(ExportError::Clipboard("not an image".into())),
        }
    }
}

fn png_options(options: &ExportOptions) -> PngOptions {
    PngOptions {
        scale: options.scale(),
        transparent: options.transparent,
    }
}

fn unavailable(format: ExportFormat) -> ExportError {
    ExportError::Render(format!("{format:?} cannot be exported from this snapshot"))
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
}

impl ExportJobs {
    pub fn new(clipboard: SharedClipboard, print_dir: PathBuf) -> Self {
        Self {
            running: Vec::new(),
            delay: None,
            clipboard,
            print_dir,
            notice: None,
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
                done.push(Err(ExportError::Render("the export stopped".into())));
                false
            }
        });
        done
    }
}
