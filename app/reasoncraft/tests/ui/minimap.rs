//! UI tests for the View tab's overview minimap, map numbering and Fit
//! selection.

use crate::common::harness;
use std::time::{Duration, Instant};

use eframe::egui;
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
use reasoncraft::canvas::paint::content_bounds;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft::settings::{MinimapMode, SETTINGS_KEY, Settings};
use reasoncraft_core::fixtures::sample_iot_like;
use reasoncraft_core::{Document, Item, Layout, layout};

use crate::common::H;

fn harness_with(doc: Document, dialogs: FakeDialogs) -> H {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(dialogs));
    app.tabs[0] = DocTab::from_document(doc);
    harness(app)
}

/// The active document's layout, measured and sized like the canvas's.
fn layout_of(h: &H) -> Layout {
    let mut cache = MeasureCache::default();
    let params = h.state().settings.layout_params();
    layout(
        &h.state().tabs[0].doc,
        &EguiMeasure::new(&h.ctx, &mut cache),
        &params,
    )
}

fn minimap_shown(h: &H) -> bool {
    h.query_by_label(&t("minimap.name")).is_some()
}

/// A big map at 100%, so it does not fit the view.
fn zoomed_in_sample(dialogs: FakeDialogs) -> H {
    let mut h = harness_with(sample_iot_like(117), dialogs);
    apply(h.state_mut(), Action::Zoom100);
    h.run();
    h
}

#[test]
fn minimap_auto_hidden_when_map_fits() {
    let mut h = harness_with(Document::new("Claim"), FakeDialogs::default());
    assert_eq!(h.state().settings.minimap, MinimapMode::Auto);
    assert!(!minimap_shown(&h), "a small map fits");
    apply(h.state_mut(), Action::SetMinimap(MinimapMode::On));
    h.run();
    assert!(minimap_shown(&h), "On shows it anyway");
}

#[test]
fn minimap_shows_when_map_larger() {
    let mut h = zoomed_in_sample(FakeDialogs::default());
    assert!(minimap_shown(&h));
    // Fitted, the whole map is in view again.
    apply(h.state_mut(), Action::ZoomFit);
    h.run();
    assert!(!minimap_shown(&h));
    apply(h.state_mut(), Action::Zoom100);
    apply(h.state_mut(), Action::SetMinimap(MinimapMode::Off));
    h.run();
    assert!(!minimap_shown(&h), "Off hides it");
}

#[test]
fn minimap_click_centres_camera() {
    let mut h = zoomed_in_sample(FakeDialogs::default());
    // Look somewhere else first.
    h.state_mut().tabs[0].camera.offset += egui::vec2(700.0, -400.0);
    h.run();
    let centre = h.get_by_label(&t("minimap.name")).rect().center();
    let selection = h.state().tabs[0].selection;
    h.event(egui::Event::PointerMoved(centre));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: centre,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    }
    let tab = &h.state().tabs[0];
    let view = tab.viewport.expect("canvas shown");
    let looking_at = tab.camera.screen_to_world(view.center());
    let target = content_bounds(&layout_of(&h)).center();
    assert!(
        (looking_at - target).length() < 1.0,
        "{looking_at:?} vs {target:?}"
    );
    assert_eq!(tab.selection, selection, "a click on it selects nothing");
}

/// Export `h`'s map as SVG to `path`, waiting for the background job.
fn export_svg(h: &mut H, dialogs: &FakeDialogs, path: std::path::PathBuf) -> String {
    dialogs.set_export(path.clone());
    apply(
        h.state_mut(),
        Action::RunExport(ExportRequest {
            format: ExportFormat::Svg,
            options: ExportOptions {
                svg_outlines: false,
                ..ExportOptions::default()
            },
            page: PageSetup::default(),
        }),
    );
    let start = Instant::now();
    while h.state().export.busy() {
        assert!(start.elapsed() < Duration::from_secs(60), "export hangs");
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn minimap_not_in_exports() {
    let dir = tempfile::tempdir().unwrap();
    let dialogs = FakeDialogs::default();
    let mut h = zoomed_in_sample(dialogs.clone());
    apply(h.state_mut(), Action::SetMinimap(MinimapMode::On));
    h.run();
    assert!(minimap_shown(&h));
    let on = export_svg(&mut h, &dialogs, dir.path().join("on.svg"));
    apply(h.state_mut(), Action::SetMinimap(MinimapMode::Off));
    h.run();
    let off = export_svg(&mut h, &dialogs, dir.path().join("off.svg"));
    assert_eq!(on, off, "the minimap changes nothing in an export");
}

/// Numbers reach exports like markers do: the export shows them when
/// they are on, and its boxes are sized like the screen's.
#[test]
fn numbers_reach_exports() {
    let dir = tempfile::tempdir().unwrap();
    let dialogs = FakeDialogs::default();
    let mut h = harness_with(sample_iot_like(12), dialogs.clone());
    let off = export_svg(&mut h, &dialogs, dir.path().join("off.svg"));
    apply(h.state_mut(), Action::ToggleNumbers);
    h.run();
    let on = export_svg(&mut h, &dialogs, dir.path().join("on.svg"));
    let numbers = reasoncraft_core::outline::box_numbers(&h.state().tabs[0].doc, false);
    // Parked items are not exported.
    let main: Vec<_> = numbers
        .iter()
        .filter(|(_, n)| !n.starts_with('P'))
        .collect();
    let row = |n: &str| format!(">{n}</text>");
    for (_, n) in &main {
        assert!(on.contains(&row(n)), "{n} missing from the export");
    }
    let compound: Vec<_> = main.iter().filter(|(_, n)| n.len() > 1).collect();
    assert!(!compound.is_empty());
    assert!(
        compound.iter().all(|(_, n)| !off.contains(&row(n))),
        "numbers off"
    );
    let lay = layout_of(&h);
    let first = *main[0].0;
    let rect = lay.boxes[&first];
    let size = format!("width=\"{}\" height=\"{}\"", rect.width(), rect.height());
    assert!(on.contains(&size), "no {size} in the export");
}

#[test]
fn toggle_numbers_persists() {
    use eframe::Storage as _;
    let mut storage = MemStorage::default();
    storage.set_string(SETTINGS_KEY, "(theme:Light)".to_owned());
    let old: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert!(!old.show_numbers);
    assert_eq!(old.minimap, MinimapMode::Auto);

    let mut app = ReasonCraftApp::new_for_test();
    apply(&mut app, Action::ToggleNumbers);
    apply(&mut app, Action::SetMinimap(MinimapMode::Off));
    assert!(app.settings.show_numbers);
    assert!(app.tabs[0].history.undo.is_empty(), "settings, not edits");
    eframe::set_value(&mut storage, SETTINGS_KEY, &app.settings);
    let back: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert!(back.show_numbers);
    assert_eq!(back.minimap, MinimapMode::Off);
    assert!(back.layout_params().indicators.numbers);
}

/// In-memory eframe storage.
#[derive(Default)]
struct MemStorage(std::collections::HashMap<String, String>);

impl eframe::Storage for MemStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

#[test]
fn fit_selection_shows_the_selected_card() {
    let mut h = zoomed_in_sample(FakeDialogs::default());
    let doc = &h.state().tabs[0].doc;
    let card = *doc.cards.keys().last().unwrap();
    h.state_mut().tabs[0].selection = Some(Item::Card(card));
    // Ctrl+Shift+1.
    h.key_press_modifiers(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::Num1,
    );
    h.run();
    let tab = &h.state().tabs[0];
    let lay = layout_of(&h);
    let target = reasoncraft::canvas::item_bounds(&tab.doc, &lay, Item::Card(card)).unwrap();
    let view = tab.viewport.unwrap();
    let shown = tab.camera.world_rect_to_screen(target);
    assert!(
        view.expand(0.5).contains_rect(shown),
        "{shown:?} in {view:?}"
    );
    assert!((shown.center() - view.center()).length() < 1.0);
}

/// The minimap never covers the inline editor: it hides while a box is
/// being edited and comes back afterwards.
#[test]
fn minimap_hides_while_editing() {
    let mut h = zoomed_in_sample(FakeDialogs::default());
    assert!(minimap_shown(&h));
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0].selection = Some(Item::Box(root));
    apply(h.state_mut(), Action::Edit);
    h.run();
    assert_eq!(h.state().tabs[0].editing(), Some(root));
    assert!(!minimap_shown(&h), "hidden while editing");
    apply(h.state_mut(), Action::StopEditing);
    h.run();
    assert!(minimap_shown(&h));
}
