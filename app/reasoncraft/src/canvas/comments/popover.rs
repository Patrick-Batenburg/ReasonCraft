//! The comment popover: the list of a box's comments, an input with a
//! verdict picker, and the preset menu.

use std::sync::Arc;

use eframe::egui::{
    self, Area, Atom, AtomKind, AtomPaintArgs, Button, Color32, Frame, Id, IntoSizedResult, Key,
    Margin, Order, Popup, Pos2, Rect, RichText, ScrollArea, Sense, Shape, SizedAtomKind, Stroke,
    TextEdit, Ui, Vec2, vec2,
};
use reasoncraft_core::{Comment, Layout, NodeId, Verdict};

use super::{PRESETS, verdict_colour};
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::shell::icon_button;
use crate::shell::icons::Icon;
use crate::spell::{FieldStyle, SpellFields};
use crate::theme::Tokens;

/// The choices of the popover's verdict picker, with their label keys.
const VERDICTS: [(Option<Verdict>, &str); 4] = [
    (None, "comment.verdict.none"),
    (Some(Verdict::Positive), "comment.verdict.positive"),
    (Some(Verdict::NeedsWork), "comment.verdict.needs_work"),
    (Some(Verdict::Problem), "comment.verdict.problem"),
];

/// Outer width of the popover, in screen px.
pub const POPOVER_WIDTH: f32 = 280.0;
const POPOVER_MARGIN: i8 = 10;
/// Gap between the box and the popover, in screen px.
const POPOVER_GAP: f32 = 14.0;
/// Height of the comment list before it scrolls, in screen px.
const LIST_HEIGHT: f32 = 260.0;

/// The popover's area id.
pub fn popover_id() -> Id {
    Id::unique("comment_popover")
}

/// What is being typed into the popover of one box (widget state).
#[derive(Clone, Default)]
struct Draft {
    text: String,
    verdict: Option<Verdict>,
}

/// The comment popover of `tab.comments_open`, right of its box (left when
/// there is no room). Esc or a primary press outside it closes it (panning
/// with the right or middle button does not).
pub(in crate::canvas) fn show(
    ui: &Ui,
    tab: &DocTab,
    layout: &Layout,
    tk: &Tokens,
    spell: &mut SpellFields,
    pending: &mut Vec<Action>,
) {
    let Some(node) = tab.comments_open else {
        return;
    };
    let (Some(&world), Some(data)) = (layout.boxes.get(&node), tab.doc.boxes.get(&node)) else {
        return;
    };
    let ctx = ui.ctx();
    let id = popover_id();
    // A primary press outside last frame's popover closes it; one in a
    // menu (the presets) does not.
    let outside = ctx.input(|i| {
        let pos = i.pointer.press_origin()?;
        i.pointer.primary_pressed().then_some(pos)
    });
    let shown = egui::AreaState::load(ctx, id).map(|s| s.rect());
    let escape = ctx.input(|i| i.key_pressed(Key::Escape));
    if !Popup::is_any_open(ctx)
        && (escape || outside.zip(shown).is_some_and(|(p, r)| !r.contains(p)))
    {
        pending.push(Action::CloseComments);
        return;
    }

    let view = ui.clip_rect();
    let boxed = tab.camera.world_rect_to_screen(world);
    let mut pos = boxed.right_top() + vec2(POPOVER_GAP, 0.0);
    if pos.x + POPOVER_WIDTH > view.right()
        && boxed.left() - POPOVER_GAP - POPOVER_WIDTH >= view.left()
    {
        pos.x = boxed.left() - POPOVER_GAP - POPOVER_WIDTH;
    }
    let draft_id = tab.widget_id(("comment_draft", node));
    let field_id = tab.widget_id(("comment_field", node));
    let mut draft: Draft = ctx.data(|d| d.get_temp(draft_id)).unwrap_or_default();

    Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(pos)
        .constrain_to(view)
        .show(ctx, |ui| {
            let stroke = Stroke::new(1.0, tk.attachment_line);
            let inner = POPOVER_WIDTH - 2.0 * (f32::from(POPOVER_MARGIN) + stroke.width);
            Frame::new()
                .fill(tk.panel)
                .stroke(stroke)
                .corner_radius(8)
                .inner_margin(Margin::same(POPOVER_MARGIN))
                .shadow(ui.style().visuals.popup_shadow)
                .show(ui, |ui| {
                    ui.set_width(inner);
                    let field = Field {
                        id: field_id,
                        draft: &mut draft,
                        spell,
                    };
                    contents(ui, node, &data.comments, field, tk, pending);
                });
        });
    ctx.data_mut(|d| d.insert_temp(draft_id, draft));
}

/// The popover's comment field: its id, what is typed and the underlines.
struct Field<'a, 'b> {
    id: Id,
    draft: &'a mut Draft,
    spell: &'a mut SpellFields<'b>,
}

fn contents(
    ui: &mut Ui,
    node: NodeId,
    comments: &[Comment],
    field: Field,
    tk: &Tokens,
    pending: &mut Vec<Action>,
) {
    let Field {
        id: field_id,
        draft,
        spell,
    } = field;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(t("comment.title"))
                .strong()
                .color(tk.box_text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(
                ui,
                Icon::Close,
                &t("comment.close"),
                18.0,
                true,
                tk,
                tk.panel,
            )
            .clicked()
            {
                pending.push(Action::CloseComments);
            }
        });
    });
    if comments.is_empty() {
        ui.label(RichText::new(t("comment.none")).color(tk.text_muted));
    } else {
        ScrollArea::vertical()
            .max_height(LIST_HEIGHT)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for c in comments {
                    comment_row(ui, node, c, tk, pending);
                }
            });
    }
    ui.separator();
    let mut layouter = spell.layouter(FieldStyle::default());
    ui.add(
        TextEdit::multiline(&mut draft.text)
            .id(field_id)
            .layouter(&mut layouter)
            .hint_text(t("comment.hint"))
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (verdict, key) in VERDICTS {
            let dot = dot_atom(verdict_colour(verdict, tk), verdict.is_none());
            let chip = Button::selectable(draft.verdict == verdict, (dot, t(key)));
            if ui.add(chip).clicked() {
                draft.verdict = verdict;
            }
        }
    });
    ui.horizontal(|ui| {
        let ready = !draft.text.trim().is_empty();
        if ui
            .add_enabled(ready, Button::new(t("comment.add")))
            .clicked()
        {
            pending.push(Action::AddComment {
                node,
                text: std::mem::take(&mut draft.text),
                verdict: draft.verdict.take(),
            });
        }
        ui.menu_button((t("comment.presets"), arrow_atom()), |ui| {
            for (key, verdict) in PRESETS {
                let dot = dot_atom(verdict_colour(Some(verdict), tk), false);
                if ui.add(Button::new((dot, t(key)))).clicked() {
                    pending.push(Action::AddComment {
                        node,
                        text: t(key),
                        verdict: Some(verdict),
                    });
                }
            }
        });
    });
}

/// One comment: verdict dot, author and delete button, then the wrapped text.
fn comment_row(ui: &mut Ui, node: NodeId, c: &Comment, tk: &Tokens, pending: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        paint_dot(
            ui,
            rect.center(),
            verdict_colour(c.verdict, tk),
            c.verdict.is_none(),
        );
        let author = match c.author.as_str() {
            "" => t("comment.anonymous"),
            name => name.to_owned(),
        };
        ui.label(RichText::new(author).strong().color(tk.box_text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = t("comment.delete");
            if icon_button(ui, Icon::Close, &label, 16.0, true, tk, tk.panel).clicked() {
                pending.push(Action::RemoveComment {
                    node,
                    comment: c.id,
                });
            }
        });
    });
    ui.add(egui::Label::new(RichText::new(&c.text).color(tk.box_text)).wrap());
    ui.add_space(4.0);
}

/// A verdict dot: filled, or a ring for "no verdict".
fn paint_dot(ui: &Ui, centre: Pos2, colour: Color32, hollow: bool) {
    if hollow {
        ui.painter()
            .circle_stroke(centre, 3.5, Stroke::new(1.25, colour));
    } else {
        ui.painter().circle_filled(centre, 4.0, colour);
    }
}

/// A painted button atom of `size` px.
fn paint_atom(
    size: Vec2,
    paint: impl Fn(&Ui, Rect, &AtomPaintArgs) + Send + Sync + 'static,
) -> Atom<'static> {
    let paint = Arc::new(paint);
    AtomKind::closure(move |_ui: &Ui, _args| {
        let paint = paint.clone();
        IntoSizedResult {
            intrinsic_size: size,
            sized: SizedAtomKind::Paint {
                paint: Arc::new(move |ui: &Ui, args: AtomPaintArgs| {
                    paint(ui, Rect::from_center_size(args.rect.center(), size), &args);
                }),
                size,
            },
        }
    })
    .into()
}

/// A verdict dot as a button atom (the UI font has no "●").
fn dot_atom(colour: Color32, hollow: bool) -> Atom<'static> {
    paint_atom(Vec2::splat(10.0), move |ui, rect, _| {
        paint_dot(ui, rect.center(), colour, hollow);
    })
}

/// A "▾" drawn as a triangle in the button's text colour.
fn arrow_atom() -> Atom<'static> {
    paint_atom(vec2(8.0, 8.0), |ui, rect, args| {
        let c = rect.center();
        let tip = vec![
            c + vec2(-3.5, -1.75),
            c + vec2(3.5, -1.75),
            c + vec2(0.0, 1.75),
        ];
        ui.painter().add(Shape::convex_polygon(
            tip,
            args.fallback_text_color,
            Stroke::NONE,
        ));
    })
}
