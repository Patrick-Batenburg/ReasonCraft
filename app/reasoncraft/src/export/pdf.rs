//! A map's SVG as a PDF (svg2pdf), laid out on paper by a [`PageSetup`]:
//! scaled onto one page, or tiled over several. Text stays live text in
//! embedded fonts, so it is selectable and needs no installed font.
//! Printing goes through the OS's PDF viewer.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use pdf_writer::{Content, Finish as _, Name, Pdf, Rect, Ref, Str, TextStr};
use resvg::usvg::{self, fontdb};
use serde::{Deserialize, Serialize};

use super::ExportError;
use crate::actions::{LinkOpener, SystemOpener};
use reasoncraft_core::file::create_first_new;

/// PDF points per millimetre.
const PT_PER_MM: f32 = 72.0 / 25.4;
/// Millimetres per world px at tile scale 1.0.
const MM_PER_PX: f32 = 0.25;
/// The most pages a tiled PDF may have.
pub const MAX_PAGES: usize = 1000;
/// The size of the "r/c" page label, in pt.
const LABEL_SIZE: f32 = 7.0;
/// The start and end of temp files written for printing.
const PRINT_PREFIX: &str = "ReasonCraft-print-";
const PRINT_SUFFIX: &str = ".pdf";
/// How old a print file must be before it is cleaned up.
pub const PRINT_FILE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// A paper size.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Paper {
    #[default]
    A4,
    Letter,
}

impl Paper {
    /// Width and height in portrait, in mm.
    fn size_mm(self) -> (f32, f32) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::Letter => (215.9, 279.4),
        }
    }
}

/// Which way up the paper is.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// How the drawing is put on paper.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub enum Fit {
    /// Scaled to fit one page, centred.
    #[default]
    OnePage,
    /// Split over pages at `scale`, where 1.0 is 1 world px = 0.25 mm.
    Tile { scale: f32 },
}

/// Paper, orientation, fit and margin for PDF export and printing.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct PageSetup {
    #[serde(deserialize_with = "crate::settings::deserialize_or_default")]
    pub paper: Paper,
    #[serde(deserialize_with = "crate::settings::deserialize_or_default")]
    pub orientation: Orientation,
    #[serde(deserialize_with = "crate::settings::deserialize_or_default")]
    pub fit: Fit,
    /// Blank space on each side of the page, in mm.
    pub margin_mm: f32,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            paper: Paper::A4,
            orientation: Orientation::Portrait,
            fit: Fit::OnePage,
            margin_mm: 10.0,
        }
    }
}

impl PageSetup {
    /// The page's width and height in pt.
    pub fn page_size_pt(&self) -> (f32, f32) {
        let (w, h) = self.paper.size_mm();
        let (w, h) = match self.orientation {
            Orientation::Portrait => (w, h),
            Orientation::Landscape => (h, w),
        };
        (w * PT_PER_MM, h * PT_PER_MM)
    }
}

/// Where the drawing goes on one page, in pt with y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    /// pt per SVG px.
    pub scale: f32,
    /// The drawing's top-left corner on the page.
    pub left: f32,
    pub top: f32,
    /// Row and column (from 1) of a tile, shown as "r/c".
    pub tile: Option<(usize, usize)>,
}

/// The pages for a drawing of `w` x `h` SVG px, and the printable area
/// (x, y, width, height) each page clips to.
pub(super) fn placements(
    w: f32,
    h: f32,
    setup: &PageSetup,
) -> Result<(Vec<Placement>, [f32; 4]), ExportError> {
    let (pw, ph) = setup.page_size_pt();
    let margin = setup.margin_mm * PT_PER_MM;
    let (aw, ah) = (pw - 2.0 * margin, ph - 2.0 * margin);
    let valid = |v: f32| v.is_finite() && v > 0.0;
    if !(valid(w) && valid(h) && valid(aw) && valid(ah) && setup.margin_mm >= 0.0) {
        return Err(ExportError::Pdf("invalid drawing size or margins".into()));
    }
    let area = [margin, margin, aw, ah];
    match setup.fit {
        Fit::OnePage => {
            // Shrink to fit, but never enlarge past the natural size (as a
            // tile at scale 1), so a small map prints at normal text size.
            let scale = (aw / w).min(ah / h).min(MM_PER_PX * PT_PER_MM);
            let page = Placement {
                scale,
                left: margin + (aw - w * scale) / 2.0,
                top: ph - margin - (ah - h * scale) / 2.0,
                tile: None,
            };
            Ok((vec![page], area))
        }
        Fit::Tile { scale } => {
            if !valid(scale) {
                return Err(ExportError::Pdf("invalid tile scale".into()));
            }
            let s = scale * MM_PER_PX * PT_PER_MM;
            // A sliver under 0.01 pt does not get its own page.
            let count = |len: f32, avail: f32| ((len * s - 0.01) / avail).ceil().max(1.0);
            let (cols, rows) = (count(w, aw), count(h, ah));
            if cols * rows > MAX_PAGES as f32 {
                return Err(ExportError::Pdf(format!(
                    "{cols} × {rows} pages is more than {MAX_PAGES}"
                )));
            }
            let (cols, rows) = (cols as usize, rows as usize);
            let pages = (0..rows)
                .flat_map(|r| (0..cols).map(move |c| (r, c)))
                .map(|(r, c)| Placement {
                    scale: s,
                    left: margin - c as f32 * aw,
                    top: ph - margin + r as f32 * ah,
                    tile: Some((r + 1, c + 1)),
                })
                .collect();
            Ok((pages, area))
        }
    }
}

/// `svg` as a PDF laid out by `setup`, with text drawn from `fonts` and
/// embedded as selectable text.
pub fn svg_to_pdf(
    svg: &str,
    setup: &PageSetup,
    fonts: &fontdb::Database,
) -> Result<Vec<u8>, ExportError> {
    let usvg_opts = usvg::Options {
        fontdb: Arc::new(fonts.clone()),
        ..Default::default()
    };
    let tree =
        usvg::Tree::from_str(svg, &usvg_opts).map_err(|e| ExportError::Pdf(e.to_string()))?;
    let (w, h) = (tree.size().width(), tree.size().height());
    let (pages, [ax, ay, aw, ah]) = placements(w, h, setup)?;
    let (pw, ph) = setup.page_size_pt();

    let options = svg2pdf::ConversionOptions {
        embed_text: true,
        ..Default::default()
    };
    let (chunk, svg_ref) =
        svg2pdf::to_chunk(&tree, options).map_err(|e| ExportError::Pdf(e.to_string()))?;

    let mut alloc = Ref::new(1);
    let catalog_id = alloc.bump();
    let tree_id = alloc.bump();
    // The label font, only for tiles with room for labels in the margin.
    let labels = pages[0].tile.is_some() && ay >= LABEL_SIZE;
    let font_id = labels.then(|| alloc.bump());
    let mut map = HashMap::new();
    let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
    let svg_id = map[&svg_ref];
    let ids: Vec<(Ref, Ref)> = pages.iter().map(|_| (alloc.bump(), alloc.bump())).collect();

    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(tree_id);
    pdf.pages(tree_id)
        .kids(ids.iter().map(|(page, _)| *page))
        .count(ids.len() as i32);
    let (svg_name, font_name) = (Name(b"S1"), Name(b"F1"));
    if let Some(font_id) = font_id {
        pdf.type1_font(font_id).base_font(Name(b"Helvetica"));
    }

    for (place, (page_id, content_id)) in pages.iter().zip(&ids) {
        let mut page = pdf.page(*page_id);
        page.media_box(Rect::new(0.0, 0.0, pw, ph));
        page.parent(tree_id);
        page.contents(*content_id);
        let mut resources = page.resources();
        resources.x_objects().pair(svg_name, svg_id);
        if let Some(font_id) = font_id {
            resources.fonts().pair(font_name, font_id);
        }
        resources.finish();
        page.finish();

        let mut content = Content::new();
        content.save_state();
        if place.tile.is_some() {
            content.rect(ax, ay, aw, ah).clip_nonzero().end_path();
        }
        let (dw, dh) = (w * place.scale, h * place.scale);
        content.transform([dw, 0.0, 0.0, dh, place.left, place.top - dh]);
        content.x_object(svg_name);
        content.restore_state();
        if let Some((r, c)) = place.tile.filter(|_| labels) {
            let label = format!("{r}/{c}");
            let x = pw - ax - helvetica_width(&label) * LABEL_SIZE;
            let y = ((ay - LABEL_SIZE) / 2.0).max(2.0);
            content
                .begin_text()
                .set_font(font_name, LABEL_SIZE)
                .next_line(x, y)
                .show(Str(label.as_bytes()))
                .end_text();
        }
        pdf.stream(*content_id, &content.finish());
    }

    pdf.extend(&chunk);
    pdf.document_info(alloc.bump())
        .producer(TextStr("ReasonCraft"));
    Ok(pdf.finish())
}

/// The width of a page label in Helvetica, per unit of font size.
fn helvetica_width(label: &str) -> f32 {
    // Digits are 556/1000 em, the slash 278/1000.
    label
        .chars()
        .map(|c| if c == '/' { 0.278 } else { 0.556 })
        .sum()
}

/// Writes `pdf` to a new print file in the temp folder and opens it in
/// the OS's PDF viewer, which prints it.
pub fn print_via_pdf(pdf: &[u8]) -> Result<(), ExportError> {
    print_via_pdf_with(pdf, &std::env::temp_dir(), &SystemOpener).map(|_| ())
}

/// [`print_via_pdf`] writing into `dir` and opening with `opener`.
/// Returns the file written.
pub fn print_via_pdf_with(
    pdf: &[u8],
    dir: &Path,
    opener: &dyn LinkOpener,
) -> Result<PathBuf, ExportError> {
    let path = write_print_file(pdf, dir)?;
    opener
        .open(&path.to_string_lossy())
        .map_err(ExportError::Open)?;
    Ok(path)
}

/// Writes `pdf` to a new print file in `dir`, named
/// `ReasonCraft-print-<pid>-<n>.pdf` (so [`clean_print_files`] finds it
/// later) with the first `n` that is free, and returns its path. An
/// existing file is never replaced; on Unix only the user may read it.
pub fn write_print_file(pdf: &[u8], dir: &Path) -> Result<PathBuf, ExportError> {
    let pid = std::process::id();
    let candidates = (0u64..).map(|n| dir.join(format!("{PRINT_PREFIX}{pid}-{n}{PRINT_SUFFIX}")));
    let (path, mut file) = create_first_new(candidates, true)?;
    if let Err(e) = file.write_all(pdf) {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err(e.into());
    }
    Ok(path)
}

/// Deletes print files in the temp folder older than a day. Errors are
/// ignored: a file the viewer still holds is tried again next start.
pub fn clean_old_print_files() {
    clean_print_files(&std::env::temp_dir(), SystemTime::now(), PRINT_FILE_MAX_AGE);
}

/// Deletes the files in `dir` named `ReasonCraft-print-*.pdf` that were
/// last changed more than `max_age` before `now`. Nothing else is touched.
pub fn clean_print_files(dir: &Path, now: SystemTime, max_age: Duration) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let is_print = name.len() > PRINT_PREFIX.len() + PRINT_SUFFIX.len()
            && name.starts_with(PRINT_PREFIX)
            && name.ends_with(PRINT_SUFFIX);
        let Ok(meta) = entry.metadata() else { continue };
        let old = meta
            .modified()
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > max_age);
        if is_print && meta.is_file() && old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
