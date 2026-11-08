//! Personal preferences that survive restarts.

use std::path::{Path, PathBuf};

use reasoncraft_core::checks::{CheckKind, CheckSet};
use reasoncraft_core::layout::IndicatorOptions;
use reasoncraft_core::outline::OutlineOptions;
use reasoncraft_core::{ConnectorStyle, LayoutParams};
use serde::{Deserialize, Serialize};

use crate::export::{ExportOptions, PageSetup};
use crate::theme::ThemeChoice;

/// eframe storage key for [`Settings`].
pub const SETTINGS_KEY: &str = "settings";

/// Personal preferences, stored as a whole under [`SETTINGS_KEY`]. Fields
/// missing from older saved settings take their defaults.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    #[serde(deserialize_with = "deserialize_or_default")]
    pub theme: ThemeChoice,
    /// The interface language, one of [`crate::i18n::UI_LANGUAGES`];
    /// `None` follows the OS. Another value shows English.
    #[serde(deserialize_with = "deserialize_text_or_none")]
    pub ui_language: Option<String>,
    /// The name put on new comments.
    pub author: String,
    /// Show rating markers on boxes and cards.
    pub show_markers: bool,
    /// Tint boxes and connectors by their rating.
    pub eval_shading: bool,
    /// How connectors are drawn, on screen and in exports.
    #[serde(deserialize_with = "deserialize_or_default")]
    pub connector_style: ConnectorStyle,
    /// When the overview minimap shows in the canvas corner.
    #[serde(deserialize_with = "deserialize_or_default")]
    pub minimap: MinimapMode,
    /// Show each box's outline number on the map (and in exports).
    pub show_numbers: bool,
    /// The tab the Text panel shows.
    #[serde(deserialize_with = "deserialize_or_default")]
    pub text_tab: TextTab,
    /// What the outline shows: numbers, linking words, notes.
    pub outline: OutlineOptions,
    /// Indent outline rows by depth.
    pub outline_indent: bool,
    /// Mark outline rows with their role colour.
    pub outline_colour: bool,
    /// Which course-rule checks the Check tab and the status bar report.
    pub checks: CheckSet,
    /// The language new documents start with, `"en"` or `"nl"`; `None`
    /// follows the interface language (see [`Settings::new_doc_language`]).
    #[serde(deserialize_with = "deserialize_text_or_none")]
    pub default_doc_language: Option<String>,
    /// Paper, orientation, fit and margins for PDF export and printing.
    pub page_setup: PageSetup,
    /// The export dialog's last choices (scope, theme, PNG and SVG options).
    pub export: ExportOptions,
    /// Underline misspellings in editing fields and allow the spelling pass (F7).
    pub spellcheck: bool,
    /// The personal dictionary: words accepted in every language.
    pub spell_words: Vec<String>,
    /// Seconds between autosaves to the recovery folder; 0 turns autosave
    /// off. Used clamped to [`AUTOSAVE_MIN`]..=[`AUTOSAVE_MAX`].
    pub autosave_secs: u32,
    /// The size of the whole interface, 1.0 for 100 %. Used in steps of
    /// 10 % within [`UI_SCALE_MIN`]..=[`UI_SCALE_MAX`] (see
    /// [`Settings::ui_scale_factor`]). The map's own zoom comes on top.
    pub ui_scale: f32,
    /// The maps last opened or saved, newest first, at most [`MAX_RECENT`].
    #[serde(
        deserialize_with = "deserialize_paths",
        serialize_with = "serialize_paths"
    )]
    pub recent_files: Vec<PathBuf>,
}

/// Read a field, taking its default when the saved value is not one this
/// build knows (an enum variant added or renamed by another version).
/// Works for every format the settings are saved in (RON and JSON): a
/// refused unit variant has been consumed by then, so the rest of the file
/// is read on as usual. An unknown variant that carries data (`Name(..)`)
/// is not skipped and still fails the load; none of the settings enums
/// has such a variant today beyond the known [`Fit::Tile`](crate::export::Fit).
pub fn deserialize_or_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(T::deserialize(d).unwrap_or_default())
}

/// Read an optional text field; a value of another type (also inside
/// RON's `Some(..)`) reads as `None`, and the rest of the file still loads.
/// [`deserialize_or_default`] cannot do this for RON, which stops at the
/// wrong value: here the whole value is read before its type is checked.
pub fn deserialize_text_or_none<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Stored {
        Text(Option<String>),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Stored::deserialize(d)? {
        Stored::Text(text) => text,
        Stored::Other(_) => None,
    })
}

/// Read a list of paths; a value of another shape reads as an empty list,
/// and the rest of the file still loads (see [`deserialize_text_or_none`]).
fn deserialize_paths<'de, D>(d: D) -> Result<Vec<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Stored {
        Paths(Vec<PathBuf>),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Stored::deserialize(d)? {
        Stored::Paths(paths) => paths,
        Stored::Other(_) => Vec::new(),
    })
}

/// Write the paths that are valid UTF-8; the settings format cannot hold
/// others, and one would fail the whole settings write.
fn serialize_paths<S: serde::Serializer>(paths: &[PathBuf], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(paths.iter().filter(|p| p.to_str().is_some()))
}

/// How many recent maps the File page lists.
pub const MAX_RECENT: usize = 10;

/// The default seconds between autosaves.
pub const AUTOSAVE_DEFAULT: u32 = 45;
/// The shortest and longest autosave intervals, in seconds.
pub const AUTOSAVE_MIN: u32 = 30;
pub const AUTOSAVE_MAX: u32 = 600;

/// The smallest and largest interface scales (80 % and 200 %).
pub const UI_SCALE_MIN: f32 = 0.8;
pub const UI_SCALE_MAX: f32 = 2.0;

/// When the overview minimap shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinimapMode {
    /// Only while the map does not fit in the view.
    #[default]
    Auto,
    On,
    Off,
}

impl MinimapMode {
    pub const ALL: [MinimapMode; 3] = [MinimapMode::Auto, MinimapMode::On, MinimapMode::Off];

    /// The key used in command ids and translations.
    pub fn key(self) -> &'static str {
        match self {
            MinimapMode::Auto => "auto",
            MinimapMode::On => "on",
            MinimapMode::Off => "off",
        }
    }
}

/// One of the Outline tab's toolbar switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineToggle {
    Numbering,
    Labels,
    Indent,
    Colour,
    Notes,
}

impl Settings {
    /// The language choice the settings page and View → Language show as
    /// selected: `None` for System, else the UI language shown (English
    /// for a stored language this build lacks).
    pub fn ui_language_choice(&self) -> Option<&'static str> {
        self.ui_language.as_deref().map(crate::i18n::language_of)
    }

    /// The UI language these settings show: the stored one, or the one
    /// `os_locale` names while they follow the OS (read only then).
    pub fn resolved_ui_language(&self, os_locale: fn() -> Option<String>) -> &'static str {
        match &self.ui_language {
            Some(lang) => crate::i18n::language_of(lang),
            None => crate::i18n::os_ui_language(os_locale().as_deref()),
        }
    }

    /// The language a new blank document starts in: the chosen one, else
    /// the UI language (see [`Self::resolved_ui_language`]).
    pub fn new_doc_language(&self, os_locale: fn() -> Option<String>) -> &str {
        match &self.default_doc_language {
            Some(lang) => lang,
            None => self.resolved_ui_language(os_locale),
        }
    }

    /// `secs` as an autosave setting: 0 (off) or within the allowed range.
    pub fn clamp_autosave(secs: u32) -> u32 {
        if secs == 0 {
            0
        } else {
            secs.clamp(AUTOSAVE_MIN, AUTOSAVE_MAX)
        }
    }

    /// `scale` as an interface scale: rounded to a step of 10 % within
    /// [`UI_SCALE_MIN`]..=[`UI_SCALE_MAX`]; 100 % for a value that is not a
    /// number.
    pub fn clamp_ui_scale(scale: f32) -> f32 {
        if scale.is_finite() {
            ((scale * 10.0).round() / 10.0).clamp(UI_SCALE_MIN, UI_SCALE_MAX)
        } else {
            1.0
        }
    }

    /// The interface scale to use (see [`Self::clamp_ui_scale`]).
    pub fn ui_scale_factor(&self) -> f32 {
        Self::clamp_ui_scale(self.ui_scale)
    }

    /// Every setting back to its default, except the personal dictionary
    /// and the recent files: those are the user's own data, not
    /// preferences.
    pub fn reset_to_defaults(&mut self) {
        *self = Settings {
            spell_words: std::mem::take(&mut self.spell_words),
            recent_files: std::mem::take(&mut self.recent_files),
            ..Settings::default()
        };
    }

    /// Put `path` first in the recent files (once: another spelling of the
    /// same file goes), keeping at most [`MAX_RECENT`]. The path is stored
    /// resolved (see [`recent_form`]); the listed ones are only made
    /// absolute to compare, so no file system walk runs for them. A path
    /// that is not valid UTF-8 is left out: the settings cannot store it.
    pub fn push_recent(&mut self, path: &Path) {
        if path.to_str().is_none() {
            return;
        }
        let path = recent_form(path);
        self.recent_files
            .retain(|p| std::path::absolute(p).as_ref().unwrap_or(p) != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }

    /// Seconds between autosaves, or `None` when autosave is off.
    pub fn autosave_interval(&self) -> Option<u32> {
        (self.autosave_secs != 0).then(|| Self::clamp_autosave(self.autosave_secs))
    }

    /// The layout parameters these settings ask for: boxes get an
    /// indicator row for the marks they show (numbers included, numbered
    /// like the Outline with its current Notes switch).
    pub fn layout_params(&self) -> LayoutParams {
        LayoutParams {
            indicators: IndicatorOptions {
                markers: self.show_markers,
                numbers: self.show_numbers,
                numbers_notes: self.outline.notes,
            },
            ..LayoutParams::default()
        }
    }

    /// Whether the outline switch `toggle` is on.
    pub fn outline_toggle(&self, toggle: OutlineToggle) -> bool {
        match toggle {
            OutlineToggle::Numbering => self.outline.numbering,
            OutlineToggle::Labels => self.outline.labels,
            OutlineToggle::Indent => self.outline_indent,
            OutlineToggle::Colour => self.outline_colour,
            OutlineToggle::Notes => self.outline.notes,
        }
    }

    /// Turn the outline switch `toggle` on or off.
    pub fn set_outline_toggle(&mut self, toggle: OutlineToggle, on: bool) {
        let field = match toggle {
            OutlineToggle::Numbering => &mut self.outline.numbering,
            OutlineToggle::Labels => &mut self.outline.labels,
            OutlineToggle::Indent => &mut self.outline_indent,
            OutlineToggle::Colour => &mut self.outline_colour,
            OutlineToggle::Notes => &mut self.outline.notes,
        };
        *field = on;
    }

    /// Turn the check `kind` on or off (a set, not a flip).
    pub fn set_check_enabled(&mut self, kind: CheckKind, on: bool) {
        let disabled = &mut self.checks.disabled;
        disabled.retain(|k| *k != kind);
        if !on {
            disabled.push(kind);
        }
    }
}

/// How the recent files store `path`: resolved (links and `..` followed)
/// when the file exists, else made absolute. On Windows the resolved form
/// keeps the usual `C:\` spelling, not the `\\?\` one.
fn recent_form(path: &Path) -> PathBuf {
    use std::path::{Component, Prefix};
    let Ok(resolved) = path.canonicalize() else {
        return std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    };
    let mut parts = resolved.components();
    if let Some(Component::Prefix(prefix)) = parts.next()
        && let Prefix::VerbatimDisk(drive) = prefix.kind()
    {
        let rest: PathBuf = parts.filter(|c| *c != Component::RootDir).collect();
        return PathBuf::from(format!("{}:\\", drive as char)).join(rest);
    }
    resolved
}

/// The tabs of the Text panel.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextTab {
    #[default]
    Outline,
    Essay,
    Check,
    Snippets,
    /// The offline help (see [`crate::help`]).
    Help,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            ui_language: None,
            author: String::new(),
            show_markers: true,
            eval_shading: true,
            connector_style: ConnectorStyle::default(),
            minimap: MinimapMode::default(),
            show_numbers: false,
            text_tab: TextTab::default(),
            outline: OutlineOptions::default(),
            outline_indent: true,
            outline_colour: true,
            checks: CheckSet::default(),
            default_doc_language: None,
            page_setup: PageSetup::default(),
            export: ExportOptions::default(),
            spellcheck: true,
            spell_words: Vec::new(),
            autosave_secs: AUTOSAVE_DEFAULT,
            ui_scale: 1.0,
            recent_files: Vec::new(),
        }
    }
}
