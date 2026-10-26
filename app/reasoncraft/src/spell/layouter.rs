//! Red underlines under misspelled words in editing fields: a text-edit
//! layouter that lays the field's text out as egui would, with the
//! misspelled ranges underlined. Only fields use it, so the painted map
//! and exports never show underlines.

use std::ops::Range;
use std::sync::Arc;

use eframe::egui::{
    Align, Color32, FontId, Galley, Stroke, TextBuffer, TextStyle, Ui,
    text::{LayoutJob, TextFormat},
};

use super::SpellService;
use crate::settings::Settings;
use crate::theme::Tokens;

/// Width of the underline under a misspelled word.
pub const UNDERLINE_WIDTH: f32 = 1.5;

/// How a field shows its text: what its `TextEdit` was given (`None`:
/// egui's default for a text field).
#[derive(Clone, Debug)]
pub struct FieldStyle {
    pub font: Option<FontId>,
    pub colour: Option<Color32>,
    pub halign: Align,
}

impl Default for FieldStyle {
    fn default() -> Self {
        Self {
            font: None,
            colour: None,
            halign: Align::LEFT,
        }
    }
}

/// What the editing fields of one document need to underline its
/// misspellings: the dictionaries, the document's language and the
/// underline colour. With spellcheck off nothing is underlined.
pub struct SpellFields<'a> {
    /// `None` when spellcheck is off.
    service: Option<&'a mut SpellService>,
    lang: String,
    underline: Color32,
}

impl<'a> SpellFields<'a> {
    pub fn new(
        service: &'a mut SpellService,
        settings: &Settings,
        lang: &str,
        tk: &Tokens,
    ) -> Self {
        Self {
            service: settings.spellcheck.then_some(service),
            lang: lang.to_owned(),
            underline: tk.spell_error,
        }
    }

    /// The misspelled ranges of `text` (none with spellcheck off or while
    /// the dictionary loads).
    fn misspelled(&mut self, text: &str) -> Vec<Range<usize>> {
        self.service
            .as_mut()
            .and_then(|s| s.misspelled(&self.lang, text))
            .map(|r| r.to_vec())
            .unwrap_or_default()
    }

    /// A layouter for a multiline `TextEdit` shown in `style`.
    pub fn layouter(
        &mut self,
        style: FieldStyle,
    ) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> + '_ {
        move |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let text = text.as_str();
            let bad = self.misspelled(text);
            let font = style
                .font
                .clone()
                .unwrap_or_else(|| TextStyle::Body.resolve(ui.style()));
            let colour = style.colour.unwrap_or_else(|| {
                let v = ui.visuals();
                v.override_text_color
                    .unwrap_or_else(|| v.widgets.inactive.text_color())
            });
            let line_height =
                ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
            let format = TextFormat {
                line_height: Some(line_height),
                ..TextFormat::simple(font, colour)
            };
            let mut job = spell_layouter(text, &bad, format, self.underline);
            job.wrap.max_width = wrap_width;
            job.halign = style.halign;
            ui.fonts_mut(|f| f.layout_job(job))
        }
    }
}

/// `text` as a layout job in `format`, with the `bad` byte ranges
/// underlined in `underline`. Ranges out of order, out of bounds or off a
/// character boundary are skipped. Trailing whitespace is kept, as a
/// field being typed in needs.
pub fn spell_layouter(
    text: &str,
    bad: &[Range<usize>],
    format: TextFormat,
    underline: Color32,
) -> LayoutJob {
    let marked = TextFormat {
        underline: Stroke::new(UNDERLINE_WIDTH, underline),
        ..format.clone()
    };
    let mut job = LayoutJob {
        break_on_newline: true,
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut at = 0;
    for r in bad {
        let valid = at <= r.start
            && r.start < r.end
            && r.end <= text.len()
            && text.is_char_boundary(r.start)
            && text.is_char_boundary(r.end);
        if !valid {
            continue;
        }
        if at < r.start {
            job.append(&text[at..r.start], 0.0, format.clone());
        }
        job.append(&text[r.clone()], 0.0, marked.clone());
        at = r.end;
    }
    if at < text.len() || job.sections.is_empty() {
        job.append(&text[at..], 0.0, format);
    }
    job
}

#[cfg(test)]
mod tests {
    use eframe::egui::{self, TextEdit};

    use super::*;
    use crate::spell::service::tests::wait;

    fn underlined(job: &LayoutJob) -> Vec<&str> {
        job.sections
            .iter()
            .filter(|s| s.format.underline.width > 0.0)
            .map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0])
            .collect()
    }

    #[test]
    fn marks_ranges_and_skips_bad_ones() {
        let format = TextFormat::default();
        let text = "I recieve teh é";
        let job = spell_layouter(text, &[2..9, 10..13], format.clone(), Color32::RED);
        assert_eq!(job.text, text);
        assert_eq!(underlined(&job), ["recieve", "teh"]);
        let s = &job.sections[1];
        assert_eq!(s.format.underline, Stroke::new(1.5, Color32::RED));
        // Overlapping, empty, out of bounds and mid-character ranges.
        let job = spell_layouter(
            text,
            &[2..9, 4..6, 9..9, 15..17, 30..31],
            format,
            Color32::RED,
        );
        assert_eq!(underlined(&job), ["recieve"]);
        assert_eq!(job.text, text);
        assert_eq!(
            spell_layouter("", &[], TextFormat::default(), Color32::RED)
                .sections
                .len(),
            1
        );
    }

    #[test]
    fn underline_cache_hits_on_unchanged_text() {
        let mut service = SpellService::new(&[]);
        wait(&mut service, "en");
        let paragraph = "I recieve the mail. ".repeat(2500);
        assert_eq!(paragraph.chars().count(), 50_000);
        let settings = Settings::default();
        let tk = Tokens::dark();
        let ctx = egui::Context::default();
        let mut first = None;
        for _ in 0..2 {
            let mut text = paragraph.clone();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut spell = SpellFields::new(&mut service, &settings, "en", &tk);
                let mut layouter = spell.layouter(FieldStyle::default());
                ui.add(TextEdit::multiline(&mut text).layouter(&mut layouter));
            });
            out.textures_delta.clear();
            first.get_or_insert(service.checks_run());
        }
        assert_eq!(first, Some(1), "checked on the first frame");
        assert_eq!(service.checks_run(), 1, "the second frame hit the cache");
    }
}
