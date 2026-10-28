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
use std::sync::{Arc, Mutex, MutexGuard};

use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::outline_md::{outline_markdown, outline_plain_text_lang};
use reasoncraft_core::{Assets, Document, file, to_argdown};
use reasoncraft_core::{essay_markdown, essay_plain_text};
use resvg::usvg::fontdb;

use super::docx::{essay_docx, outline_docx};
use super::{
    ClipboardSink, ExportError, ExportOptions, ImageFormat, PageSetup, PngOptions, SvgOptions,
    TextFormat, bundled_font_family, copy_scene_to_clipboard, scene_to_png, scene_to_svg,
    shared_fontdb, svg_to_pdf, write_print_file,
};
use crate::i18n::t_lang;
use crate::jobs::{Hold, Job, JobState};
use crate::scene::Scene;
use crate::theme::Tokens;

/// A clipboard the worker can reach.
pub type SharedClipboard = Arc<Mutex<dyn ClipboardSink + Send>>;

/// A picture of the map: its scene (drawn at zoom 1), the images it shows
/// and the colours it is drawn in.
pub struct Picture {
    pub scene: Scene,
    pub assets: Assets,
    pub tokens: Tokens,
}

/// What an export makes, from a snapshot taken on the UI thread, and where
/// it goes. Each kind holds only what it is made from, so no format meets
/// a snapshot it cannot be made from.
pub enum Make {
    /// An image file of the map.
    Image {
        format: ImageFormat,
        picture: Picture,
        path: PathBuf,
    },
    /// A PDF print file in folder `dir`, opened afterwards by the UI thread.
    Print { picture: Picture, dir: PathBuf },
    /// The map as a PNG on the clipboard.
    Copy {
        picture: Picture,
        sink: SharedClipboard,
    },
    /// A text file of the document, with the outline's switches.
    Text {
        format: TextFormat,
        doc: Box<Document>,
        outline: OutlineOptions,
        path: PathBuf,
    },
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
    pub options: ExportOptions,
    pub page: PageSetup,
    pub make: Make,
}

impl ExportWork {
    /// Makes the export and writes (or copies) it.
    pub fn run(self) -> Result<Outcome, ExportError> {
        self.run_gated(&WriteGate::default())
    }

    /// Like [`Self::run`], writing the file only while `gate` is open.
    fn run_gated(self, gate: &WriteGate) -> Result<Outcome, ExportError> {
        let fonts = shared_fontdb();
        let (opts, page) = (&self.options, &self.page);
        let (bytes, print, path, note) = match self.make {
            Make::Copy { picture, sink } => {
                // A copy that panicked earlier leaves nothing half done in
                // the sink, so a poisoned lock is safe to take over.
                let mut sink = sink.lock().unwrap_or_else(|e| e.into_inner());
                let Picture {
                    scene,
                    assets,
                    tokens,
                } = &picture;
                let png = png_options(opts);
                copy_scene_to_clipboard(scene, assets, tokens, &png, &fonts, &mut *sink)?;
                return Ok(Outcome::Copied);
            }
            Make::Image {
                format,
                picture,
                path,
            } => {
                let bytes = image_bytes(format, &picture, opts, page, &fonts)?;
                (bytes, false, path, None)
            }
            Make::Print { picture, dir } => {
                let bytes = image_bytes(ImageFormat::Pdf, &picture, opts, page, &fonts)?;
                (bytes, true, dir, None)
            }
            Make::Text {
                format,
                doc,
                outline,
                path,
            } => {
                let note = (format == TextFormat::Argdown
                    && doc.boxes.values().any(|b| b.image.is_some()))
                .then_some("export.argdown_no_images");
                (text_bytes(format, &doc, &outline)?, false, path, note)
            }
        };
        // Held until the file is complete, so a closed gate never leaves a
        // half-written file behind.
        let _writing = gate.enter()?;
        if print {
            return Ok(Outcome::ReadyToPrint(write_print_file(&bytes, &path)?));
        }
        write_atomically(&path, &bytes)?;
        Ok(match note {
            Some(key) => Outcome::SavedWithNote(path, key),
            None => Outcome::Saved(path),
        })
    }
}

/// `picture` as a PNG, an SVG or a PDF (which embeds the fonts and keeps
/// text selectable).
fn image_bytes(
    format: ImageFormat,
    picture: &Picture,
    options: &ExportOptions,
    page: &PageSetup,
    fonts: &Arc<fontdb::Database>,
) -> Result<Vec<u8>, ExportError> {
    let Picture {
        scene,
        assets,
        tokens,
    } = picture;
    Ok(match format {
        ImageFormat::Png => scene_to_png(scene, assets, tokens, &png_options(options), fonts)?,
        ImageFormat::Svg => {
            let opts = SvgOptions {
                text_as_paths: options.svg_outlines,
                ..SvgOptions::new(tokens)
            };
            scene_to_svg(scene, assets, &opts, &bundled_font_family()).into_bytes()
        }
        ImageFormat::Pdf => {
            let opts = SvgOptions::for_pdf(tokens);
            let svg = scene_to_svg(scene, assets, &opts, &bundled_font_family());
            svg_to_pdf(&svg, page, fonts)?
        }
    })
}

/// `doc` as an outline, an essay or Argdown, in the document's language.
fn text_bytes(
    format: TextFormat,
    doc: &Document,
    outline: &OutlineOptions,
) -> Result<Vec<u8>, ExportError> {
    let lines = build_outline(doc, outline);
    let lang = doc.meta.language.as_str();
    let label = |key: &str| t_lang(lang, key);
    Ok(match format {
        TextFormat::OutlineMarkdown => outline_markdown(&lines, outline, lang, &label).into_bytes(),
        TextFormat::OutlineText => {
            outline_plain_text_lang(&lines, outline, lang, &label).into_bytes()
        }
        TextFormat::OutlineDocx => outline_docx(&lines, outline, lang, &doc.meta.author)?,
        TextFormat::EssayMarkdown => essay_markdown(doc, lang).into_bytes(),
        TextFormat::EssayText => essay_plain_text(doc, lang).into_bytes(),
        TextFormat::EssayDocx => essay_docx(doc, lang)?,
        TextFormat::Argdown => to_argdown(doc).into_bytes(),
    })
}

fn png_options(options: &ExportOptions) -> PngOptions {
    PngOptions {
        scale: options.scale(),
        transparent: options.transparent,
    }
}

/// Writes `bytes` to `path` through a temporary file next to it, so an
/// existing file is replaced whole or not at all.
pub(super) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    Ok(file::atomic_write(path, false, |f| f.write_all(bytes))?)
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
    /// Hold each export back until released before it starts (tests
    /// only).
    pub hold: Option<Hold>,
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
    /// The text on the clipboard Copy as image uses, if any.
    pub fn read_clipboard_text(&self) -> Option<String> {
        // A copy that panicked earlier leaves nothing half done.
        let mut board = self.clipboard.lock().unwrap_or_else(|e| e.into_inner());
        board.text()
    }

    pub fn new(clipboard: SharedClipboard, print_dir: PathBuf) -> Self {
        Self {
            running: Vec::new(),
            hold: None,
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
            .push(Job::spawn(self.hold.clone(), move || work.run_gated(&gate)));
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
