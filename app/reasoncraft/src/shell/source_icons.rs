//! The source-type glyphs as egui widgets: painted in a rect, as a button
//! atom, and (debug builds) a preview window. The shapes themselves are
//! [`crate::scene::source_icons`].

use std::sync::Arc;

use eframe::egui::{
    Atom, AtomKind, AtomPaintArgs, Color32, IntoSizedResult, Painter, Rect, SizedAtomKind, Ui, Vec2,
};
use reasoncraft_core::SourceType;

use crate::canvas::scene_paint::paint_shapes;
use crate::scene::source_icons::source_icon_shapes;

/// Draw the glyph for `t` in `rect` (a square, screen px) in `colour`.
pub fn paint_source_icon(painter: &Painter, rect: Rect, t: &SourceType, colour: Color32) {
    paint_shapes(painter, &source_icon_shapes(rect, t, colour));
}

/// The glyph for `t` as a button atom of `size` px, painted in the
/// button's text colour (so it dims with a disabled entry).
pub fn source_atom(t: &SourceType, size: f32) -> Atom<'static> {
    let t = t.clone();
    AtomKind::closure(move |_ui: &Ui, _args| {
        let size = Vec2::splat(size);
        IntoSizedResult {
            intrinsic_size: size,
            sized: SizedAtomKind::Paint {
                paint: Arc::new(move |ui: &Ui, args: AtomPaintArgs| {
                    let rect = Rect::from_center_size(args.rect.center(), size);
                    paint_source_icon(ui.painter(), rect, &t, args.fallback_text_color);
                }),
                size,
            },
        }
    })
    .into()
}

/// A window with every glyph at 14 px and 32 px, on a box and on the
/// ribbon, for checking them by eye (debug builds only).
#[cfg(debug_assertions)]
pub fn preview(ctx: &eframe::egui::Context, app: &crate::app::ReasonCraftApp) {
    use eframe::egui::{Grid, Sense, Window};
    if !app.show_source_icons {
        return;
    }
    let tk = app.tokens;
    let mut types = SourceType::ALL.to_vec();
    types.push(SourceType::Other("hologram".to_owned()));
    Window::new(crate::i18n::t("action.source_icons"))
        .resizable(false)
        .show(ctx, |ui| {
            Grid::new("source_icons")
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for t in &types {
                        ui.label(crate::i18n::t(&format!("source.{}", t.key())));
                        for (px, fill, colour) in [
                            (14.0, tk.box_fill, tk.text_muted),
                            (14.0, tk.box_fill, tk.box_text),
                            (32.0, tk.box_fill, tk.box_text),
                            (32.0, tk.ribbon, tk.text_muted),
                        ] {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, fill);
                            let icon = Rect::from_center_size(rect.center(), Vec2::splat(px));
                            paint_source_icon(ui.painter(), icon, t, colour);
                        }
                        ui.end_row();
                    }
                });
        });
}
