//! The map canvas: camera, text measurement, painting, hit testing and
//! editing interactions (selection, inline text, keyboard, "+" menu, drag,
//! drops from the building panel, comments, link glyphs, box images) and
//! the overview minimap.

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
pub mod minimap;
pub mod paint;
pub mod plus_menu;
pub mod rating;

use eframe::egui::{self, Key, PointerButton, Rect, Response, Role, Sense, WidgetInfo};
use reasoncraft_core::{Document, Item, Layout, layout};

pub use camera::Camera;
use hit::{Hit, hit_test};
use measure::EguiMeasure;

use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::i18n::t;
use crate::scene::marks_leads;
use crate::spell::SpellFields;

impl From<Hit> for Item {
    fn from(hit: Hit) -> Self {
        match hit {
            Hit::Box(b) => Item::Box(b),
            Hit::CardFrame(c) | Hit::CardFarEdge(c) => Item::Card(c),
        }
    }
}

/// The widget id of the map: while it (or nothing) has the keyboard focus,
/// the keys work the map.
pub fn canvas_id() -> egui::Id {
    egui::Id::unique("map_canvas")
}

/// Lay out, navigate and paint the active document in the space `ui` has
/// left. A click selects what is under the pointer (or nothing); a drag
/// moves it; the selected box gets a "+" menu and the edited box a text field.
pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let rect = ui.max_rect();
    let response = ui.interact(rect, canvas_id(), Sense::click_and_drag());
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
        spell,
        ..
    } = app;
    images.evict_unused(tabs);
    let tab = &mut tabs[*active];
    if images.load(&ctx, &tab.doc, &tab.assets) {
        ctx.request_repaint();
    }
    // Recomputed every frame: about 0.3 ms for 500 boxes, and never stale.
    let measure = EguiMeasure::new(&ctx, measure_cache);
    let params = settings.layout_params();
    let layout = layout(&tab.doc, &measure, &params);

    if let Some(old) = tab.viewport
        && old != rect
    {
        tab.camera.follow_resize(old, rect);
    }
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
    if std::mem::take(&mut tab.fit_selection_pending)
        && let Some(target) = tab
            .selection
            .and_then(|i| item_bounds(&tab.doc, &layout, i))
    {
        tab.camera.fit_selection(target, rect);
    }
    let panning = navigate(ui, &response, &mut tab.camera, rect);
    drag::autoscroll(ui, &response, &mut tab.camera);

    if tab.connect_mode.is_none()
        && response.clicked()
        && !panning
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = hit_test(&layout, tab.camera.screen_to_world(pos), tab.camera.zoom);
        pending.push(Action::Select(hit.map(Item::from)));
    }

    let leads = marks_leads(&layout, &measure, tab.camera.zoom);
    let find_hits = if tab.find.open {
        tab.find_hits().iter().map(|h| h.node).collect()
    } else {
        Default::default()
    };
    let painter = ui.painter_at(rect);
    paint::paint(
        &painter,
        &layout,
        &tab.doc,
        &tab.camera,
        &tokens,
        tab.selection,
        tab.editing(),
        paint::PaintOptions {
            show_markers: settings.show_markers,
            eval_shading: settings.eval_shading,
            connector_style: settings.connector_style,
            numbers: settings.show_numbers,
        },
        images,
        &measure,
        &leads,
        &find_hits,
    );
    comments::badges(ui, tab, &layout, &leads, pending);
    links::glyphs(ui, tab, &layout, &leads, pending);
    images::preparing(ui, tab, &layout, &tokens);
    connect::update(ui, &response, &layout, tab, &tokens, pending, panning);
    drag::update(ui, &response, &layout, tab, &tokens, pending, panning);
    drag::panel_drop(ui, &response, &layout, tab, &tokens, pending);
    // Under the inline editor and the "+" button, which take its clicks.
    minimap::show(
        ui,
        tab,
        &layout,
        &tokens,
        settings.minimap,
        params.indicators,
    );
    let mut fields = SpellFields::new(spell, settings, &tab.doc.meta.language, &tokens);
    editor::show(ui, tab, &layout, &tokens, &mut fields, pending);
    if !response.dragged() {
        plus_menu::show(ui, tab, &layout, &tokens, pending);
    }
    comments::show(ui, tab, &layout, &tokens, &mut fields, pending);
}

/// The world rect of `item` in `layout`: a box's own rect, or a card's
/// frame with everything under it.
pub fn item_bounds(doc: &Document, layout: &Layout, item: Item) -> Option<Rect> {
    let rect = |i: Item| match i {
        Item::Box(b) => layout.boxes.get(&b).copied(),
        Item::Card(c) => layout.cards.get(&c).copied(),
    };
    let own = rect(item)?;
    Some(match item {
        Item::Box(_) => own,
        Item::Card(_) => doc
            .descendants(item)
            .into_iter()
            .filter_map(rect)
            .fold(own, Rect::union),
    })
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
