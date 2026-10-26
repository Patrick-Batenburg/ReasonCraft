//! Exports of a map: the [`Scene`](crate::scene::Scene) written as SVG,
//! with the fonts the screen uses, so a file looks like the canvas on any
//! machine.

mod docx;
mod docx_props;
mod job;
mod pdf;
mod raster;
mod request;
mod svg;
mod svg_attr;

use std::sync::{Arc, LazyLock};

use eframe::egui::FontDefinitions;
use eframe::egui::emath::GuiRounding as _;
use resvg::usvg::fontdb;

pub use docx::{essay_docx, outline_docx};
pub use job::{ExportJobs, ExportWork, Notice, Outcome, SharedClipboard, Snapshot, Target};
pub use pdf::{
    Fit, MAX_PAGES, Orientation, PRINT_FILE_MAX_AGE, PageSetup, Paper, clean_old_print_files,
    clean_print_files, print_via_pdf, print_via_pdf_with, svg_to_pdf, write_print_file,
};
pub use raster::{
    ClipboardSink, FakeClipboard, MAX_CLIPBOARD_PIXELS, MAX_PIXELS, MAX_SIDE_PX, PngOptions,
    SystemClipboard, copy_image, copy_png_to_clipboard, copy_scene_to_clipboard, png_to_rgba,
    scene_to_png, svg_to_png, svg_to_rgba,
};
pub use request::{ExportFormat, ExportOptions, ExportRequest, ExportScope, ExportTheme};
pub use svg::{DEFAULT_MARGIN, SvgOptions, scene_to_svg, svg_size};

/// Why an export failed. The app layer shows [`key`](Self::key) through
/// `t`; the details are for logs.
#[derive(Debug)]
pub enum ExportError {
    /// The image would pass the pixel caps.
    TooLarge {
        w: u32,
        h: u32,
    },
    /// The image would pass the (lower) clipboard cap.
    ClipboardTooLarge {
        w: u32,
        h: u32,
    },
    Io(std::io::Error),
    Render(String),
    Clipboard(String),
    /// The PDF could not be made.
    Pdf(String),
    /// The PDF viewer could not be opened to print.
    Open(String),
    /// The export's worker stopped without a result.
    Stopped,
    /// This build cannot make this export (yet).
    Unsupported(ExportFormat),
}

impl ExportError {
    /// The message key for this kind of error.
    pub fn key(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "export-error-too_large",
            Self::ClipboardTooLarge { .. } => "export-error-clipboard_too_large",
            Self::Io(_) => "export-error-io",
            Self::Render(_) => "export-error-render",
            Self::Clipboard(_) => "export-error-clipboard",
            Self::Pdf(_) => "export-error-pdf",
            Self::Open(_) => "export-error-open",
            Self::Stopped => "export-error-stopped",
            Self::Unsupported(_) => "export-error-unsupported",
        }
    }

    /// The part of the error to show next to the message (`{ $detail }`).
    pub fn detail(&self) -> String {
        match self {
            Self::TooLarge { w, h } | Self::ClipboardTooLarge { w, h } => format!("{w} × {h}"),
            Self::Io(e) => e.to_string(),
            Self::Render(s) | Self::Clipboard(s) | Self::Pdf(s) | Self::Open(s) => s.clone(),
            Self::Stopped => String::new(),
            Self::Unsupported(format) => format.extension().to_owned(),
        }
    }
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key(), self.detail())
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// The name egui gives its proportional text font.
const TEXT_FONT: &str = "Ubuntu-Light";

/// egui's default fonts (the ones the canvas draws with), loaded once.
static FONTS: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut db = fontdb::Database::new();
    for data in FontDefinitions::default().font_data.values() {
        db.load_font_data((*data.font).as_ref().to_vec());
    }
    Arc::new(db)
});

/// The font family name inside egui's text font, and its vertical metrics
/// per unit of font size.
static TEXT_FACE: LazyLock<TextFace> = LazyLock::new(TextFace::load);

/// A font database holding egui's default fonts and nothing else, so text
/// in exports never depends on the fonts installed.
pub fn bundled_fontdb() -> fontdb::Database {
    (**FONTS).clone()
}

pub(crate) fn shared_fontdb() -> Arc<fontdb::Database> {
    FONTS.clone()
}

/// The `font-family` value that selects the canvas's text font in an SVG
/// (with a generic fallback for viewers without it).
pub fn bundled_font_family() -> String {
    format!("'{}', sans-serif", TEXT_FACE.family)
}

/// The text font's family name and vertical metrics.
struct TextFace {
    family: String,
    /// Ascent, descent (negative) and line gap per unit of font size.
    ascent: f32,
    descent: f32,
    gap: f32,
}

impl TextFace {
    fn load() -> Self {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};

        let defs = FontDefinitions::default();
        let data = &defs.font_data[TEXT_FONT];
        let bytes: &[u8] = (*data.font).as_ref();
        let font = skrifa::FontRef::from_index(bytes, data.index).expect("egui's text font");
        // Read the metrics the way egui does (unscaled, default location).
        let m = font.metrics(Size::unscaled(), LocationRef::default());
        let em = f32::from(m.units_per_em);
        let mut probe = fontdb::Database::new();
        probe.load_font_data(bytes.to_vec());
        let family = probe
            .faces()
            .next()
            .and_then(|f| f.families.first())
            .map(|(name, _)| name.clone())
            .expect("egui's text font has a family name");
        Self {
            family,
            ascent: m.ascent / em,
            descent: m.descent / em,
            gap: m.leading / em,
        }
    }

    /// The unrounded height of one row at `size`, as egui computes it.
    fn font_height(&self, size: f32) -> f32 {
        (self.ascent * size).round_ui() - (self.descent * size).round_ui()
            + (self.gap * size).round_ui()
    }

    /// The height of a laid-out row at `size`: whole pixels at 1 px per
    /// point, as exports are drawn.
    fn row_height(&self, size: f32) -> f32 {
        self.font_height(size).round()
    }

    /// The distance from a row's top to its baseline at `size`, placed as
    /// egui places it: glyphs sit at the bottom of the rounded row, then
    /// snap to a whole pixel.
    fn baseline(&self, size: f32) -> f32 {
        let ascent = (self.ascent * size).round_ui();
        (ascent + self.row_height(size) - self.font_height(size)).round()
    }
}

#[cfg(test)]
mod docx_tests;
#[cfg(test)]
mod job_tests;
#[cfg(test)]
mod output_tests;
#[cfg(test)]
mod pdf_tests;
#[cfg(test)]
mod raster_tests;
#[cfg(test)]
mod tests;
