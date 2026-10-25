//! Exports of a map: the [`Scene`](crate::scene::Scene) written as SVG,
//! with the fonts the screen uses, so a file looks like the canvas on any
//! machine.

mod svg;

use std::sync::{Arc, LazyLock};

use eframe::egui::FontDefinitions;
use eframe::egui::emath::GuiRounding as _;
use resvg::usvg::fontdb;

pub use svg::{DEFAULT_MARGIN, SvgOptions, scene_to_svg};

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
mod tests;
