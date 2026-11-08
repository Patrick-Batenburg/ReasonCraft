//! The backstage in an 800×500 window: every entry down the left
//! is on screen, above the status bar, and a mouse click on a page's
//! entry opens that page (before, About and the entries above it ran past
//! the window's bottom).

use eframe::egui::{self, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::i18n::t;

use crate::common::H;

fn backstage(size: egui::Vec2) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.ribbon_tab = RibbonTab::File;
    let mut h = Harness::builder()
        .with_size(size)
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

fn click_at(h: &mut H, pos: egui::Pos2) {
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run();
}

const ENTRIES: [&str; 12] = [
    "backstage.back",
    "action.new",
    "action.open",
    "backstage.recent",
    "action.import_argdown",
    "action.save",
    "action.save_as",
    "backstage.export",
    "export.print",
    "backstage.document",
    "backstage.settings",
    "backstage.about",
];

/// The backstage entry `key`: the left-most node with its name (the
/// title bar may repeat a command).
fn entry(h: &H, key: &str) -> egui::Rect {
    h.query_all_by_label(&t(key))
        .map(|n| n.rect())
        .min_by(|a, b| a.min.x.total_cmp(&b.min.x))
        .unwrap_or_else(|| panic!("no entry {key}"))
}

#[test]
fn every_backstage_entry_fits_an_800_by_500_window() {
    let mut h = backstage(egui::vec2(800.0, 500.0));
    // The status bar's zoom slider marks where the backstage ends.
    let status_top = h.get_by_label(&t("status.zoom")).rect().min.y;
    for key in ENTRIES {
        let rect = entry(&h, key);
        assert!(
            rect.min.y >= 0.0 && rect.max.y <= status_top,
            "{key} at {rect:?} is not above the status bar at {status_top}"
        );
    }
    // Each page's entry, clicked, shows the page (its title).
    let about = format!("{} {}", t("app.name"), env!("CARGO_PKG_VERSION"));
    for (key, title) in [
        ("backstage.recent", t("backstage.recent.title")),
        ("backstage.document", t("document.title")),
        ("backstage.settings", t("settings.title")),
        ("backstage.about", about),
    ] {
        // The title may repeat the entry's name ("Document").
        let before = h.query_all_by_label(&title).count();
        let at = entry(&h, key).center();
        click_at(&mut h, at);
        let after = h.query_all_by_label(&title).count();
        assert_eq!(after, before + 1, "{key} shows {title:?}");
    }
}
