//! UI tests for the app shell, driven through the accessibility tree.

use crate::common::{app_with_doc, build};
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::i18n::t;

fn harness() -> Harness<'static, ReasonCraftApp> {
    build(ReasonCraftApp::new_for_test())
}

#[test]
fn ribbon_add_reason_adds_card() {
    let mut h = harness();
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.run();
    assert_eq!(h.state().tabs[0].doc.cards.len(), 0);
    let label = t("action.add_reason");
    assert_eq!(label, "Reason");
    h.get_by_label(&label).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards.len(), 1);
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn theme_switch_applies_dark_visuals() {
    let mut h = harness();
    h.run();
    // Start from light so the switch is observable.
    let view = t("ribbon.tab.view");
    h.get_by_label(&view).click();
    h.run();
    h.get_by_label(&t("theme.light")).click();
    h.run();
    assert!(!h.ctx.global_style().visuals.dark_mode);
    h.get_by_label(&t("theme.dark")).click();
    h.run();
    assert!(h.ctx.global_style().visuals.dark_mode);
    assert_eq!(
        h.state().settings.theme,
        reasoncraft::theme::ThemeChoice::Dark
    );
}

#[test]
fn new_tab_button_opens_second_document() {
    let mut h = harness();
    h.run();
    assert_eq!(h.state().tabs.len(), 1);
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 2);
    assert_eq!(h.state().active, 1);
}

#[test]
fn missing_i18n_key_returns_key() {
    assert_eq!(t("no.such.key"), "no.such.key");
    assert_eq!(t("untitled"), "Untitled");
    assert_eq!(t("ribbon.tab.insert"), "Insert");
}

fn harness_with(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    build(app_with_doc(doc))
}

#[test]
fn canvas_renders_117_box_fixture_without_panic() {
    let doc = reasoncraft_core::fixtures::sample_iot_like(117);
    assert!(doc.boxes.len() >= 117);
    let mut h = harness_with(doc);
    h.run_steps(3);
    let tab = &h.state().tabs[0];
    let viewport = tab.viewport.expect("canvas reports its viewport");
    assert!(viewport.width() > 400.0 && viewport.height() > 300.0);
    // A new document is fitted on its first frame; this map is too wide for 100%.
    assert!(!tab.fit_pending);
    assert!(tab.camera.zoom < 1.0);
}

/// Press and release the primary button at `pos`.
fn click_at(h: &mut Harness<'static, ReasonCraftApp>, pos: egui::Pos2) {
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
}

#[test]
fn canvas_click_selects_box_and_empty_canvas_clears() {
    use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
    use reasoncraft_core::{Item, LayoutParams, layout};

    let doc = reasoncraft_core::fixtures::sample_iot_like(40);
    let mut h = harness_with(doc);
    h.run();
    let (target, screen, corner) = {
        let tab = &h.state().tabs[0];
        let mut cache = MeasureCache::default();
        let l = layout(
            &tab.doc,
            &EguiMeasure::new(&h.ctx, &mut cache),
            &LayoutParams::default(),
        );
        let (&id, rect) = l.boxes.iter().nth(5).unwrap();
        let screen = tab.camera.world_to_screen(rect.center());
        (id, screen, tab.viewport.unwrap().min + egui::vec2(4.0, 4.0))
    };
    click_at(&mut h, screen);
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(target)));
    click_at(&mut h, corner);
    assert_eq!(h.state().tabs[0].selection, None);
}

#[test]
fn window_title_uses_i18n_separator() {
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("doc", "robots");
    args.set("app", t("app.name"));
    assert_eq!(
        reasoncraft::i18n::t_args("window.title", &args),
        "robots — ReasonCraft"
    );
}

/// eframe storage held in memory.
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
fn settings_from_old_storage_fill_defaults() {
    use eframe::Storage as _;
    use reasoncraft::settings::{SETTINGS_KEY, Settings};
    use reasoncraft::theme::ThemeChoice;
    let s: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(s.theme, ThemeChoice::Light);
    assert_eq!(s.author, "");
    assert!(s.show_markers && s.eval_shading);

    // A check name only a newer build knows (a bare identifier in eframe's
    // RON) is skipped; the rest of the settings still load.
    let mut storage = MemStorage::default();
    storage.set_string(
        SETTINGS_KEY,
        "(author:\"Ann\",checks:(disabled:[key_term,future_kind]))".to_owned(),
    );
    let s: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert_eq!(s.author, "Ann");
    assert_eq!(
        s.checks.disabled,
        vec![reasoncraft_core::checks::CheckKind::KeyTerm]
    );

    // The same through eframe's own storage format, as a step-1 build wrote it.
    let mut storage = MemStorage::default();
    storage.set_string(SETTINGS_KEY, "(theme:Light)".to_owned());
    let s: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert_eq!(
        s,
        Settings {
            theme: ThemeChoice::Light,
            ..Settings::default()
        }
    );
    let d = Settings::default();
    // The first run follows the OS's theme (owner decision D3).
    assert_eq!(d.theme, ThemeChoice::System);
    assert!(d.author.is_empty() && d.show_markers && d.eval_shading);

    // The whole struct round-trips.
    let custom = Settings {
        theme: ThemeChoice::System,
        ui_language: Some("nl".to_owned()),
        author: "Ann".to_owned(),
        show_markers: false,
        eval_shading: true,
        connector_style: reasoncraft_core::ConnectorStyle::Elbow,
        minimap: reasoncraft::settings::MinimapMode::On,
        show_numbers: true,
        text_tab: reasoncraft::settings::TextTab::Check,
        outline: reasoncraft_core::outline::OutlineOptions {
            numbering: false,
            labels: true,
            notes: true,
        },
        outline_indent: false,
        outline_colour: true,
        checks: reasoncraft_core::checks::CheckSet {
            disabled: vec![reasoncraft_core::checks::CheckKind::KeyTerm],
        },
        default_doc_language: Some("nl".to_owned()),
        page_setup: reasoncraft::export::PageSetup {
            paper: reasoncraft::export::Paper::Letter,
            orientation: reasoncraft::export::Orientation::Landscape,
            fit: reasoncraft::export::Fit::Tile { scale: 1.5 },
            margin_mm: 5.0,
        },
        export: reasoncraft::export::ExportOptions {
            scope: reasoncraft::export::ExportScope::Selection,
            theme: reasoncraft::export::ExportTheme::Current,
            png_scale: 3,
            transparent: true,
            svg_outlines: false,
        },
        spellcheck: false,
        spell_words: vec!["Reasoncraftish".to_owned()],
        autosave_secs: 120,
        ui_scale: 1.3,
        recent_files: vec!["maps/one.rcraft".into(), "two.rcraft".into()],
    };
    eframe::set_value(&mut storage, SETTINGS_KEY, &custom);
    assert_eq!(
        eframe::get_value::<Settings>(&storage, SETTINGS_KEY),
        Some(custom)
    );
}

/// The connector style is saved with the settings (eframe's storage
/// format), and settings saved before it existed load as Curved.
#[test]
fn connector_style_persists_in_settings() {
    use reasoncraft::actions::{Action, apply};
    use reasoncraft::settings::{SETTINGS_KEY, Settings};
    use reasoncraft_core::ConnectorStyle;
    let mut storage = MemStorage::default();
    storage_set(&mut storage, "(theme:Light)");
    let old: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert_eq!(old.connector_style, ConnectorStyle::Curved);
    assert!(
        old.spellcheck && old.spell_words.is_empty(),
        "spellcheck on"
    );

    let mut app = reasoncraft::ReasonCraftApp::new_for_test();
    apply(&mut app, Action::SetConnectorStyle(ConnectorStyle::Tapered));
    assert!(
        app.tabs[0].history.undo.is_empty(),
        "a setting, not an edit"
    );
    eframe::set_value(&mut storage, SETTINGS_KEY, &app.settings);
    let back: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert_eq!(back.connector_style, ConnectorStyle::Tapered);
}

fn storage_set(storage: &mut MemStorage, value: &str) {
    use eframe::Storage as _;
    storage.set_string(reasoncraft::settings::SETTINGS_KEY, value.to_owned());
}

#[test]
fn backstage_settings_set_author() {
    let mut h = harness();
    h.run();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    // The lowest text field: the title bar has the command search.
    fn field<'a>(h: &'a Harness<'static, ReasonCraftApp>) -> egui_kittest::Node<'a> {
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
            .expect("author field")
    }
    field(&h).focus();
    h.run();
    field(&h).type_text("Ann ");
    h.run();
    assert_eq!(h.state().settings.author, "Ann");
    // The trailing space stays in the field while typing.
    field(&h).type_text("Lee");
    h.run();
    assert_eq!(h.state().settings.author, "Ann Lee");
    // The theme can be chosen here too.
    let light = reasoncraft::a11y::in_group(&t("settings.theme"), &t("theme.light"));
    h.get_by_label(&light).click();
    h.run();
    assert_eq!(
        h.state().settings.theme,
        reasoncraft::theme::ThemeChoice::Light
    );
}

#[test]
fn backstage_has_export_and_print() {
    use reasoncraft::actions::FakeOpener;
    use reasoncraft::app::RibbonTab;

    let dir = tempfile::tempdir().unwrap();
    let opener = FakeOpener::default();
    let mut h = harness();
    h.state_mut().link_opener = Box::new(opener.clone());
    h.state_mut().export.print_dir = dir.path().to_owned();
    h.run();
    // "Export formats" opens the Export ribbon tab and closes the backstage.
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.export")).click();
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Export);
    assert!(h.query_by_label(&t("backstage.export")).is_none());
    // Print runs Print: a PDF in the print folder, opened in the viewer.
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t(reasoncraft::commands::get("export.print").label))
        .click();
    crate::common::step_until(&mut h, |_| !opener.opened.borrow().is_empty(), "the print");
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
    let printed = std::path::PathBuf::from(&opener.opened.borrow()[0]);
    assert_eq!(printed.parent().unwrap(), dir.path());
}

/// The backstage opens on a page: New while there are no recent maps,
/// Recent once there are; clicking the page's entry again keeps it.
#[test]
fn backstage_opens_on_a_page() {
    let mut h = harness();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    assert!(h.query_by_label(&t("backstage.new.blank")).is_some(), "New");
    h.get_by_label(&t("backstage.back")).click();
    h.run();
    h.state_mut().settings.recent_files = vec!["maps/a.rcraft".into()];
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    let title = t("backstage.recent.title");
    assert!(h.query_by_label(&title).is_some(), "Recent");
    h.get_by_label(&t("backstage.recent")).click();
    h.run();
    assert!(h.query_by_label(&title).is_some(), "still Recent");
}

#[test]
fn backstage_keeps_the_page_picked() {
    let mut h = harness();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    h.get_by_label(&t("backstage.recent")).click();
    h.run();
    assert!(h.query_by_label(&t("backstage.recent.title")).is_some());
    h.get_by_label(&t("backstage.back")).click();
    h.run();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    assert!(h.query_by_label(&t("backstage.recent.title")).is_some());
}
