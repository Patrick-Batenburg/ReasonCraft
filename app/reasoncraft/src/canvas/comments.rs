//! Comments on boxes: the preset texts, the speech-bubble badge with the
//! comment count, and the popover that lists, adds and removes comments.

use eframe::egui::{
    self, Align2, Color32, Rect, Role, Sense, Shape, Stroke, Ui, WidgetInfo, pos2, vec2,
};
use reasoncraft_core::{Comment, Layout, Verdict};

use super::links::LINK_GLYPH;
use super::paint::{Pass, screen_font};
use super::rating::{CORNER_INSET, MARKER_MIN_ZOOM};
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t_args};
use crate::theme::Tokens;

mod popover;
pub(super) use popover::show;
pub use popover::{POPOVER_WIDTH, popover_id};

/// The preset comments: i18n key of the text, and the verdict it carries.
pub const PRESETS: [(&str, Verdict); 12] = [
    ("comment.preset.great_work", Verdict::Positive),
    ("comment.preset.good_structure", Verdict::Positive),
    ("comment.preset.strong_evidence", Verdict::Positive),
    ("comment.preset.excellent_evaluation", Verdict::Positive),
    ("comment.preset.getting_there", Verdict::NeedsWork),
    ("comment.preset.refinement_needed", Verdict::NeedsWork),
    ("comment.preset.more_evidence_needed", Verdict::NeedsWork),
    ("comment.preset.too_accepting", Verdict::NeedsWork),
    ("comment.preset.unclear", Verdict::Problem),
    ("comment.preset.rethink_structure", Verdict::Problem),
    ("comment.preset.source_not_appropriate", Verdict::Problem),
    ("comment.preset.inconsistent_evaluation", Verdict::Problem),
];

/// Badge height and its width for one, two or three characters (world px).
const BADGE_H: f32 = 12.0;
const BADGE_W: [f32; 3] = [16.0, 20.0, 25.0];
const BADGE_FONT: f32 = 10.0;

/// The colour of a verdict; no verdict is muted.
pub fn verdict_colour(verdict: Option<Verdict>, tk: &Tokens) -> Color32 {
    match verdict {
        Some(Verdict::Positive) => tk.reason,
        Some(Verdict::NeedsWork) => tk.rebuttal,
        Some(Verdict::Problem) => tk.objection,
        None => tk.text_muted,
    }
}

/// How serious a verdict is, for picking the worst.
fn severity(verdict: Option<Verdict>) -> u8 {
    match verdict {
        None => 0,
        Some(Verdict::Positive) => 1,
        Some(Verdict::NeedsWork) => 2,
        Some(Verdict::Problem) => 3,
    }
}

/// The badge colour: that of the worst verdict among `comments`.
pub fn badge_colour(comments: &[Comment], tk: &Tokens) -> Color32 {
    let worst = comments
        .iter()
        .map(|c| c.verdict)
        .max_by_key(|v| severity(*v))
        .flatten();
    verdict_colour(worst, tk)
}

/// The count shown on a badge ("99+" beyond 99).
fn badge_text(count: usize) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
}

/// The body of the badge of a box at `world` with `count` comments, in
/// world units. It sits inside the box's top-left corner, below the link
/// glyph's place, [`CORNER_INSET`] from the edges and the glyph, and grows
/// to the right: it never reaches into a neighbouring box.
pub fn badge_rect(world: Rect, count: usize) -> Rect {
    let chars = badge_text(count).len().clamp(1, 3);
    let w = BADGE_W[chars - 1];
    let top = world.top() + CORNER_INSET + LINK_GLYPH + CORNER_INSET;
    Rect::from_min_size(pos2(world.left() + CORNER_INSET, top), vec2(w, BADGE_H))
}

/// The badge's accessible name and tooltip, e.g. "3 comments".
pub fn badge_label(count: usize) -> String {
    let mut args = FluentArgs::new();
    args.set("count", count);
    t_args("comment.badge", &args)
}

impl Pass<'_> {
    /// The speech-bubble badge with the comment count of the box at `world`.
    pub(super) fn comment_badge(&self, world: Rect, comments: &[Comment]) {
        let z = self.cam.zoom;
        if comments.is_empty() || z < MARKER_MIN_ZOOM {
            return;
        }
        let body = self.screen(badge_rect(world, comments.len()));
        if !self.visible(body) {
            return;
        }
        let colour = badge_colour(comments, self.t);
        let stroke = Stroke::new((1.2 * z).max(1.0), colour);
        // The tail points down and right, staying inside the box.
        let tail = vec![
            body.right_bottom() - vec2(7.0 * z, 0.5 * z),
            body.right_bottom() - vec2(2.5 * z, 0.5 * z),
            body.right_bottom() + vec2(0.0, 3.5 * z),
        ];
        self.painter
            .add(Shape::convex_polygon(tail, colour, Stroke::NONE));
        self.painter.rect(
            body,
            4.0 * z,
            self.t.box_fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        if let Some(font) = screen_font(BADGE_FONT, z) {
            self.painter.text(
                body.center(),
                Align2::CENTER_CENTER,
                badge_text(comments.len()),
                font,
                colour,
            );
        }
    }
}

/// Make each visible badge a button that opens its box's comments. They
/// sit above the canvas and take drags too, so a press on a badge never
/// selects, drags the box or pans.
pub(super) fn badges(ui: &Ui, tab: &DocTab, layout: &Layout, pending: &mut Vec<Action>) {
    if tab.camera.zoom < MARKER_MIN_ZOOM {
        return;
    }
    let clip = ui.clip_rect();
    for (&id, &world) in &layout.boxes {
        let Some(node) = tab.doc.boxes.get(&id) else {
            continue;
        };
        if node.comments.is_empty() {
            continue;
        }
        let rect = tab
            .camera
            .world_rect_to_screen(badge_rect(world, node.comments.len()))
            .expand(2.0);
        if !clip.intersects(rect) {
            continue;
        }
        let label = badge_label(node.comments.len());
        let response = ui.interact(
            rect,
            ui.scope_id().with(("comment_badge", id)),
            Sense::click_and_drag(),
        );
        response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.on_hover_text(&label).clicked() {
            pending.push(Action::OpenComments(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::t;
    use reasoncraft_core::Uuid;

    fn comment(verdict: Option<Verdict>) -> Comment {
        Comment {
            id: Uuid::new_v4(),
            author: "A".into(),
            text: "T".into(),
            verdict,
            created: String::new(),
        }
    }

    #[test]
    fn badge_colour_follows_worst_verdict() {
        let tk = Tokens::dark();
        let c = |vs: &[Option<Verdict>]| {
            let list: Vec<Comment> = vs.iter().map(|v| comment(*v)).collect();
            badge_colour(&list, &tk)
        };
        use Verdict::*;
        assert_eq!(c(&[]), tk.text_muted);
        assert_eq!(c(&[None, None]), tk.text_muted);
        assert_eq!(c(&[None, Some(Positive)]), tk.reason);
        assert_eq!(c(&[Some(Positive), Some(NeedsWork), None]), tk.rebuttal);
        assert_eq!(
            c(&[Some(Problem), Some(Positive), Some(NeedsWork)]),
            tk.objection
        );
        assert_eq!(c(&[Some(NeedsWork), Some(Problem)]), tk.objection);
    }

    #[test]
    fn badge_sits_in_the_box_below_the_link_glyph() {
        let world = Rect::from_min_size(pos2(100.0, 50.0), vec2(156.0, 40.0));
        let glyph = super::super::links::link_rect(world);
        for count in [1, 12, 100, 5000] {
            let badge = badge_rect(world, count);
            // The tail hangs 3.5 px below the body.
            assert!(
                world.contains_rect(badge.expand2(vec2(0.0, 3.5))),
                "{count}"
            );
            assert_eq!(badge.left(), world.left() + CORNER_INSET);
            assert_eq!(badge.top(), glyph.bottom() + CORNER_INSET);
        }
        assert_eq!(badge_text(7), "7");
        assert_eq!(badge_text(100), "99+");
    }

    #[test]
    fn presets_have_texts_and_keys_match_en_ftl() {
        assert_eq!(
            PRESETS.iter().filter(|p| p.1 == Verdict::Positive).count(),
            4
        );
        assert_eq!(
            PRESETS.iter().filter(|p| p.1 == Verdict::NeedsWork).count(),
            4
        );
        for (key, _) in PRESETS {
            let text = t(key);
            assert_ne!(text, key, "en.ftl lacks `{key}`");
            assert!(text.chars().count() <= 40, "`{key}` is short");
        }
        assert_eq!(badge_label(1), "1 comment");
        assert_eq!(badge_label(3), "3 comments");
    }
}
