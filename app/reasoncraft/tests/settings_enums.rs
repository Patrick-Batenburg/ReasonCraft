//! Settings saved by a newer build may name enum values this build does not
//! know. Such a field falls back to its default; the rest of the settings
//! still load.

use eframe::Storage as _;
use reasoncraft::export::{ExportScope, ExportTheme, Fit, Orientation, Paper};
use reasoncraft::settings::{MinimapMode, SETTINGS_KEY, Settings, TextTab};
use reasoncraft::theme::ThemeChoice;
use reasoncraft_core::ConnectorStyle;

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

fn from_ron(text: &str) -> Settings {
    let mut storage = MemStorage::default();
    storage.set_string(SETTINGS_KEY, text.to_owned());
    eframe::get_value(&storage, SETTINGS_KEY).expect("settings load")
}

#[test]
fn unknown_enum_values_keep_other_settings() {
    let s = from_ron("(author:\"Ann\",theme:Sepia,connector_style:Zigzag,minimap:Huge)");
    assert_eq!(s.author, "Ann");
    assert_eq!(s.theme, ThemeChoice::default());
    assert_eq!(s.connector_style, ConnectorStyle::default());
    assert_eq!(s.minimap, MinimapMode::default());
}

#[test]
fn unknown_enum_values_keep_other_settings_json() {
    let s: Settings = serde_json::from_str(
        r#"{"author":"Ann","theme":"Sepia","connector_style":"zigzag","minimap":"huge"}"#,
    )
    .unwrap();
    assert_eq!(s.author, "Ann");
    assert_eq!(s.theme, ThemeChoice::default());
    assert_eq!(s.connector_style, ConnectorStyle::default());
    assert_eq!(s.minimap, MinimapMode::default());
}

#[test]
fn ui_language_of_another_build_or_type_keeps_other_settings() {
    // A language this build lacks is kept (and shows English); a value of
    // the wrong type means "follow the OS".
    let s: Settings = serde_json::from_str(r#"{"ui_language":"xx","author":"Ann"}"#).unwrap();
    assert_eq!(s.ui_language.as_deref(), Some("xx"));
    assert_eq!(s.ui_language_choice(), Some("en"));
    let s: Settings =
        serde_json::from_str(r#"{"ui_language":5,"author":"Ann","minimap":"off"}"#).unwrap();
    assert_eq!(s.ui_language, None);
    assert_eq!(s.ui_language_choice(), None);
    assert_eq!(s.author, "Ann");
    assert_eq!(s.minimap, MinimapMode::Off);
    for ron in ["5", "Some(5)", "Some((1,2))"] {
        let s = from_ron(&format!("(ui_language:{ron},author:\"Ann\",minimap:off)"));
        assert_eq!(s.ui_language, None, "{ron}");
        assert_eq!((s.author.as_str(), s.minimap), ("Ann", MinimapMode::Off));
    }
    let s = from_ron("(ui_language:None)");
    assert_eq!(s.ui_language, None);
    let s = from_ron("(ui_language:Some(\"nl\"))");
    assert_eq!(s.ui_language_choice(), Some("nl"));
}

#[test]
fn unknown_values_in_nested_enums_fall_back_and_later_fields_still_load() {
    let ron = "(text_tab:Notes,export:(scope:Everything,theme:Neon,png_scale:3),\
               page_setup:(paper:A3,orientation:Diagonal,fit:Spiral,margin_mm:5.0),\
               spellcheck:false)";
    let s = from_ron(ron);
    assert_eq!(s.text_tab, TextTab::default());
    assert_eq!(s.export.scope, ExportScope::default());
    assert_eq!(s.export.theme, ExportTheme::default());
    assert_eq!(s.export.png_scale, 3);
    assert_eq!(s.page_setup.paper, Paper::default());
    assert_eq!(s.page_setup.orientation, Orientation::default());
    assert_eq!(s.page_setup.fit, Fit::OnePage);
    assert_eq!(s.page_setup.margin_mm, 5.0);
    assert!(!s.spellcheck);
}

#[test]
fn known_values_still_load_and_round_trip() {
    let s = Settings {
        theme: ThemeChoice::Light,
        connector_style: ConnectorStyle::Elbow,
        minimap: MinimapMode::Off,
        text_tab: TextTab::Check,
        page_setup: reasoncraft::export::PageSetup {
            paper: Paper::Letter,
            orientation: Orientation::Landscape,
            fit: Fit::Tile { scale: 2.0 },
            margin_mm: 12.0,
        },
        ..Settings::default()
    };
    let mut storage = MemStorage::default();
    eframe::set_value(&mut storage, SETTINGS_KEY, &s);
    let back: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert_eq!(back, s);
    let json: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(json, s);
}

#[test]
fn recent_files_of_another_shape_keep_other_settings() {
    let s = from_ron("(recent_files:5,author:\"Ann\")");
    assert!(s.recent_files.is_empty());
    assert_eq!(s.author, "Ann");
    let s = from_ron("(recent_files:[\"a.rcraft\"],author:\"Ann\")");
    assert_eq!(s.recent_files, vec![std::path::PathBuf::from("a.rcraft")]);
}
