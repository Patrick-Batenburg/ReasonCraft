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

fn settings_page(h: &mut H) {
    h.get_by_label(&t("ribbon.tab.file")).click();
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
    let mut h = harness_with(ReasonCraftApp::new_for_test());
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
    let h = harness_with(app);
    assert!(close(h.ctx.zoom_factor(), 1.5), "applied at start");
    // A scale from a hand-edited file is used within the range.
    let wild: Settings = serde_json::from_str(r#"{"ui_scale":9.0}"#).unwrap();
    assert_eq!(wild.ui_scale_factor(), 2.0);
}

/// Reset to defaults asks first, then restores every setting but the
/// personal dictionary (and, once there is one, the recent-files list).
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
    let mut h = harness_with(app);
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
        ..Settings::default()
    };
    assert_eq!(h.state().settings, want);
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    assert!(h.ctx.global_style().visuals.dark_mode, "the default theme");
}
