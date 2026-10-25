//! Text measurement with egui's fonts, shared by layout and painting.
//!
//! The layout asks for the height of box text wrapped at the box's text
//! width (box width minus `2 * BOX_TEXT_PAD`). The painter wraps the same
//! text at the same width with the same font, via [`wrapped_rows`], so the
//! painted rows always fit the measured box.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Galley, text::LayoutJob};
use reasoncraft_core::{BoxKind, Measure};

/// Box text size at 100% zoom.
pub const BOX_FONT_SIZE: f32 = 13.0;
/// Citation text size in source boxes at 100% zoom.
pub const SOURCE_FONT_SIZE: f32 = 11.5;
/// Entries kept before the cache starts over (old texts pile up while typing).
const CACHE_LIMIT: usize = 20_000;

/// The font box text of `kind` is measured and painted with, at 100% zoom.
pub fn font_for(kind: &BoxKind) -> FontId {
    match kind {
        BoxKind::Source { .. } => FontId::proportional(SOURCE_FONT_SIZE),
        _ => FontId::proportional(BOX_FONT_SIZE),
    }
}

/// `text` wrapped at `width` in the font for `kind`, at 100% zoom.
///
/// egui prefers word boundaries and, when a single word is wider than the
/// row, breaks it between characters, so long URLs and compounds wrap
/// instead of overflowing. (`break_anywhere: true` would break every row
/// mid-word.)
pub fn galley(ctx: &egui::Context, text: &str, width: f32, kind: &BoxKind) -> Arc<Galley> {
    let job = LayoutJob::simple(text.to_owned(), font_for(kind), Color32::PLACEHOLDER, width);
    ctx.fonts_mut(|f| f.layout_job(job))
}

/// The rows `text` wraps into at `width` (see [`galley`]), without the
/// spaces where the rows were broken.
pub fn wrapped_rows(ctx: &egui::Context, text: &str, width: f32, kind: &BoxKind) -> Vec<String> {
    galley(ctx, text, width, kind)
        .rows
        .iter()
        .map(|r| r.text().trim_end().to_owned())
        .collect()
}

/// Measured heights by (text, width bits, font size bits). Lives as long as
/// the app; cleared when the fonts or the display scale change.
#[derive(Default)]
pub struct MeasureCache {
    fonts_stamp: Option<u64>,
    heights: HashMap<(String, u32, u32), f32>,
}

impl MeasureCache {
    pub fn len(&self) -> usize {
        self.heights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heights.is_empty()
    }

    /// Forget everything if the fonts changed since the last frame.
    fn sync(&mut self, ctx: &egui::Context) {
        let stamp = fonts_stamp(ctx);
        if self.fonts_stamp != Some(stamp) || self.heights.len() > CACHE_LIMIT {
            self.heights.clear();
            self.fonts_stamp = Some(stamp);
        }
    }
}

/// A cheap fingerprint of the font setup: font names, families and scale.
fn fonts_stamp(ctx: &egui::Context) -> u64 {
    let mut h = DefaultHasher::new();
    ctx.pixels_per_point().to_bits().hash(&mut h);
    ctx.fonts(|f| {
        let defs = f.definitions();
        for (name, data) in &defs.font_data {
            name.hash(&mut h);
            Arc::as_ptr(data).hash(&mut h);
        }
        for (family, names) in &defs.families {
            family.hash(&mut h);
            names.hash(&mut h);
        }
    });
    h.finish()
}

/// [`Measure`] backed by egui's fonts.
pub struct EguiMeasure<'a> {
    pub ctx: &'a egui::Context,
    cache: RefCell<&'a mut MeasureCache>,
}

impl<'a> EguiMeasure<'a> {
    pub fn new(ctx: &'a egui::Context, cache: &'a mut MeasureCache) -> Self {
        cache.sync(ctx);
        Self {
            ctx,
            cache: RefCell::new(cache),
        }
    }
}

impl Measure for EguiMeasure<'_> {
    fn text_height(&self, text: &str, width: f32, kind: &BoxKind) -> f32 {
        let size = font_for(kind).size;
        let key = (text.to_owned(), width.to_bits(), size.to_bits());
        if let Some(&h) = self.cache.borrow().heights.get(&key) {
            return h;
        }
        let h = galley(self.ctx, text, width, kind).size().y;
        self.cache.borrow_mut().heights.insert(key, h);
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use reasoncraft_core::{BoxKind, Measure};

    /// Run `f` inside a frame, where fonts are available.
    fn with_ctx(f: impl Fn(&egui::Context)) {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| f(ui.ctx()));
        out.textures_delta.clear();
    }

    #[test]
    fn egui_measure_breaks_long_words() {
        with_ctx(|ctx| {
            let mut cache = MeasureCache::default();
            let m = EguiMeasure::new(ctx, &mut cache);
            let one_line = m.text_height("word", 156.0, &BoxKind::Premise);
            let long = "x".repeat(80);
            let h = m.text_height(&long, 156.0, &BoxKind::Premise);
            assert!(one_line > 10.0);
            assert!(
                h > one_line * 1.5,
                "80-char word: {h} vs one line {one_line}"
            );
        });
    }

    #[test]
    fn measure_matches_painted_rows_and_caches() {
        with_ctx(|ctx| {
            let mut cache = MeasureCache::default();
            let text = "A study of 2,000 households found fewer break-ins in monitored homes";
            let h = EguiMeasure::new(ctx, &mut cache).text_height(text, 144.0, &BoxKind::Premise);
            assert_eq!(cache.len(), 1);
            let rows = wrapped_rows(ctx, text, 144.0, &BoxKind::Premise);
            assert!(rows.len() >= 2);
            assert_eq!(rows.concat().replace(' ', ""), text.replace(' ', ""));
            let row_h = ctx.fonts_mut(|f| f.row_height(&font_for(&BoxKind::Premise)));
            assert!((h - rows.len() as f32 * row_h).abs() < 0.5);
            // Cached: a second lookup adds nothing.
            EguiMeasure::new(ctx, &mut cache).text_height(text, 144.0, &BoxKind::Premise);
            assert_eq!(cache.len(), 1);
        });
    }
}
