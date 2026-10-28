//! The UI scale setting and Reset to defaults.

use super::*;

/// eframe storage held in memory.
#[derive(Default)]
struct MemStorage(HashMap<String, String>);

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

/// Open the settings page (its entry may be scrolled out of view at a
/// large UI scale).
fn settings_page(h: &mut H) {
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.settings")).scroll_to_me();
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
}

/// Scroll to the settings page's Reset to defaults (at the end of the
/// page, below the window at a large scale) and click it.
fn reset_button(h: &mut H) {
    h.get_by_label(&t("settings.reset")).scroll_to_me();
    h.run();
    h.get_by_label(&t("settings.reset")).click();
    h.run();
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn ui_scale_applies_and_persists() {
    let mut h = harness(ReasonCraftApp::new_for_test());
    assert_eq!(h.state().settings.ui_scale, 1.0);
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    // The settings page's slider, in steps of 10 %.
    settings_page(&mut h);
    h.get_by_role_and_label(accesskit::Role::Slider, &t("settings.ui_scale"))
        .increment_accesskit();
    h.run();
    assert!(close(h.state().settings.ui_scale, 1.1));
    assert!(close(h.ctx.zoom_factor(), 1.1));
    // Out of range or between steps: the nearest allowed scale.
    for (asked, got) in [(5.0, 2.0), (0.1, 0.8), (1.23, 1.2), (f32::NAN, 1.0)] {
        act(&mut h, Action::SetUiScale(asked));
        assert!(close(h.state().settings.ui_scale, got), "{asked}");
    }
    act(&mut h, Action::SetUiScale(1.5));
    assert!(close(h.ctx.zoom_factor(), 1.5));
    // The map's own zoom is separate.
    assert_eq!(h.state().tabs[0].camera.zoom, 1.0);

    let mut storage = MemStorage::default();
    eframe::App::save(h.state_mut(), &mut storage);
    let saved: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert!(close(saved.ui_scale, 1.5));
    let mut app = ReasonCraftApp::new_for_test();
    app.settings = saved;
    let h = harness(app);
    assert!(close(h.ctx.zoom_factor(), 1.5), "applied at start");
    // A scale from a hand-edited file is used within the range.
    let wild: Settings = serde_json::from_str(r#"{"ui_scale":9.0}"#).unwrap();
    assert_eq!(wild.ui_scale_factor(), 2.0);
}

/// Reset to defaults asks first, then restores every setting but the
/// personal dictionary and the recent-files list.
#[test]
fn reset_to_defaults_keeps_recent_and_dictionary() {
    let mut app = ReasonCraftApp::new_for_test();
    let s = &mut app.settings;
    s.theme = ThemeChoice::Light;
    s.author = "Ann".into();
    s.ui_scale = 1.5;
    s.show_numbers = true;
    s.autosave_secs = 0;
    s.spell_words = vec!["Zork".into()];
    s.recent_files = vec!["maps/a.rcraft".into()];
    let mut h = harness(app);
    settings_page(&mut h);
    reset_button(&mut h);
    // Asked first: Cancel keeps everything.
    assert!(matches!(h.state().modal, Some(Modal::ResetSettings)));
    h.get_by_label(&t("settings.reset.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().settings.author, "Ann");
    reset_button(&mut h);
    h.get_by_label(&t("settings.reset.confirm")).click();
    h.run();
    let want = Settings {
        spell_words: vec!["Zork".into()],
        recent_files: vec!["maps/a.rcraft".into()],
        ..Settings::default()
    };
    assert_eq!(h.state().settings, want);
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    assert!(h.ctx.global_style().visuals.dark_mode, "the default theme");
}

/// An export of the map comes out the same at every interface size: its
/// text is measured at the display's own pixels per point (1.5 here), so
/// at 100 % it matches the map on screen, and the UI scale changes nothing.
#[test]
fn export_does_not_depend_on_ui_scale() {
    use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
    use reasoncraft::export::{
        ExportFormat, ExportOptions, ExportRequest, PageSetup, SvgOptions, bundled_font_family,
        scene_to_svg,
    };
    use reasoncraft::file_ops::FakeDialogs;
    use reasoncraft::scene::{SceneOptions, build_scene};
    use reasoncraft::theme::Tokens;

    let dir = tempfile::tempdir().unwrap();
    let (mut d, _) = sample();
    // Text that wraps, in a few sizes: its rows and heights show how it
    // was measured.
    let root = Parent::Box(d.root);
    for words in [7, 13, 29] {
        let text: Vec<String> = (0..words).map(|i| format!("word{i}il")).collect();
        d.add_card(root, CardKind::Reason, &text.join(" ")).unwrap();
    }
    let dialogs = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(dialogs.clone()));
    app.tabs[0] = DocTab::from_document(d);
    let mut h = crate::common::builder()
        .with_pixels_per_point(1.5)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    let export = |h: &mut H, scale: f32, name: &str| -> String {
        act(h, Action::SetUiScale(scale));
        assert!(close(h.ctx.zoom_factor(), scale));
        let file = dir.path().join(name);
        dialogs.set_export(file.clone());
        h.state_mut().push(Action::RunExport(ExportRequest {
            format: ExportFormat::Svg,
            options: ExportOptions {
                svg_outlines: false,
                ..ExportOptions::default()
            },
            page: PageSetup::default(),
        }));
        h.run_steps(2);
        crate::common::step_until(h, |app| !app.export.busy(), "the export");
        std::fs::read_to_string(&file).unwrap()
    };
    let at_100 = export(&mut h, 1.0, "100.svg");
    // The map as the canvas lays it out at 100 % on this display.
    assert!(close(h.ctx.pixels_per_point(), 1.5));
    let doc = &h.state().tabs[0].doc;
    let mut cache = MeasureCache::default();
    let measure = EguiMeasure::new(&h.ctx, &mut cache);
    let params = h.state().settings.layout_params();
    let lay = reasoncraft_core::layout(doc, &measure, &params);
    let tokens = Tokens::light();
    let scene = build_scene(doc, &lay, &SceneOptions::export(tokens, "en"), &measure);
    let opts = SvgOptions {
        text_as_paths: false,
        ..SvgOptions::new(&tokens)
    };
    let assets = &h.state().tabs[0].assets;
    let on_screen = scene_to_svg(&scene, assets, &opts, &bundled_font_family());
    assert!(
        at_100 == on_screen,
        "the export at 100 % differs from the screen"
    );
    let at_150 = export(&mut h, 1.5, "150.svg");
    let at_130 = export(&mut h, 1.3, "130.svg");
    assert!(at_100 == at_150 && at_100 == at_130, "exports differ");
}
