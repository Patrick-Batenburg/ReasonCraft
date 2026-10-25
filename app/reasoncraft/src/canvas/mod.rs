//! The map canvas: camera, text measurement, painting, hit testing and
//! editing interactions (selection, inline text, keyboard, "+" menu, drag,
//! drops from the building panel, comments, link glyphs, box images).

mod boxes;
pub mod camera;
pub mod comments;
pub mod connect;
pub mod drag;
pub mod editor;
pub mod hit;
pub mod images;
pub mod input;
pub mod links;
pub mod measure;
pub mod paint;
pub mod plus_menu;
pub mod rating;

use eframe::egui::{self, Key, PointerButton, Rect, Response, Role, Sense, WidgetInfo};
use reasoncraft_core::{Item, LayoutParams, layout};

pub use camera::Camera;
use hit::{Hit, hit_test};
use measure::EguiMeasure;

use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::i18n::t;

impl From<Hit> for Item {
    fn from(hit: Hit) -> Self {
        match hit {
            Hit::Box(b) => Item::Box(b),
            Hit::CardFrame(c) | Hit::CardBottomEdge(c) => Item::Card(c),
        }
    }
}

/// Lay out, navigate and paint the active document in the space `ui` has
/// left. A click selects what is under the pointer (or nothing); a drag
/// moves it; the selected box gets a "+" menu and the edited box a text field.
pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let rect = ui.max_rect();
    let response = ui.interact(
        rect,
        ui.scope_id().with("map_canvas"),
        Sense::click_and_drag(),
    );
    let label = t("canvas.label");
    response.widget_info(|| WidgetInfo::labeled(Role::Canvas, true, &label));

    let ctx = ui.ctx().clone();
    let tokens = app.tokens;
    let ReasonCraftApp {
        tabs,
        active,
        measure_cache,
        pending,
        settings,
        images,
        ..
    } = app;
    images.evict_unused(tabs);
    let tab = &mut tabs[*active];
    if images.load(&ctx, &tab.doc, &tab.assets) {
        ctx.request_repaint();
    }
    // Recomputed every frame: about 0.3 ms for 500 boxes, and never stale.
    let layout = layout(
        &tab.doc,
        &EguiMeasure::new(&ctx, measure_cache),
        &LayoutParams::default(),
    );

    tab.viewport = Some(rect);
    if tab.fit_pending {
        tab.camera.fit(paint::content_bounds(&layout), rect);
        tab.fit_pending = false;
    }
    if std::mem::take(&mut tab.reveal_pending)
        && let Some(target) = tab.selection.and_then(|item| match item {
            Item::Box(b) => layout.boxes.get(&b),
            Item::Card(c) => layout.cards.get(&c),
        })
    {
        tab.camera.reveal(*target, rect, camera::REVEAL_MARGIN);
    }
    let panning = navigate(ui, &response, &mut tab.camera, rect);

    if tab.connect_mode.is_none()
        && response.clicked()
        && !panning
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = hit_test(&layout, tab.camera.screen_to_world(pos), tab.camera.zoom);
        pending.push(Action::Select(hit.map(Item::from)));
    }

    let painter = ui.painter_at(rect);
    paint::paint(
        &painter,
        &layout,
        &tab.doc,
        &tab.camera,
        &tokens,
        tab.selection,
        tab.editing,
        paint::PaintOptions {
            show_markers: settings.show_markers,
            eval_shading: settings.eval_shading,
        },
        images,
    );
    comments::badges(ui, tab, &layout, pending);
    links::glyphs(ui, tab, &layout, pending);
    connect::update(ui, &response, &layout, tab, &tokens, pending, panning);
    drag::update(ui, &response, &layout, tab, &tokens, pending, panning);
    drag::panel_drop(ui, &response, &layout, tab, &tokens, pending);
    editor::show(ui, tab, &layout, &tokens, pending);
    if !response.dragged() {
        plus_menu::show(ui, tab, &layout, &tokens, pending);
    }
    comments::show(ui, tab, &layout, &tokens, pending);
}

/// Pan with middle or right drag, or space + left drag; Ctrl+wheel and pinch
/// zoom around the pointer; the wheel scrolls (Shift+wheel sideways, which
/// egui already turns into a horizontal delta). Returns whether space-panning
/// is armed, so a space+click does not select.
fn navigate(ui: &egui::Ui, response: &Response, cam: &mut Camera, rect: Rect) -> bool {
    let (space, scroll, zoom, hover) = ui.input(|i| {
        (
            i.key_down(Key::Space),
            i.smooth_scroll_delta,
            i.zoom_delta(),
            i.pointer.hover_pos(),
        )
    });
    let drag = response.dragged_by(PointerButton::Middle)
        || response.dragged_by(PointerButton::Secondary)
        || (space && response.dragged_by(PointerButton::Primary));
    if drag {
        cam.offset += response.drag_delta();
    }
    if response.contains_pointer() {
        cam.offset += scroll;
        if zoom != 1.0 {
            cam.zoom_at(hover.unwrap_or(rect.center()), zoom);
        }
    }
    space
}
