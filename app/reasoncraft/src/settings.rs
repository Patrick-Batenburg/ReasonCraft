//! Personal preferences that survive restarts.

use reasoncraft_core::checks::{CheckKind, CheckSet};
use reasoncraft_core::outline::OutlineOptions;
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
    pub theme: ThemeChoice,
    /// The name put on new comments.
    pub author: String,
    /// Show rating markers on boxes and cards.
    pub show_markers: bool,
    /// Tint boxes and connectors by their rating.
    pub eval_shading: bool,
    /// The tab the Text panel shows.
    pub text_tab: TextTab,
    /// What the outline shows: numbers, linking words, notes.
    pub outline: OutlineOptions,
    /// Indent outline rows by depth.
    pub outline_indent: bool,
    /// Mark outline rows with their role colour.
    pub outline_colour: bool,
    /// Which course-rule checks the Check tab and the status bar report.
    pub checks: CheckSet,
    /// The language new documents start with: `"en"` or `"nl"`.
    pub default_doc_language: String,
    /// Paper, orientation, fit and margins for PDF export and printing.
    pub page_setup: PageSetup,
    /// The export dialog's last choices (scope, theme, PNG and SVG options).
    pub export: ExportOptions,
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

/// The tabs of the Text panel.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextTab {
    #[default]
    Outline,
    Essay,
    Check,
    Snippets,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Dark,
            author: String::new(),
            show_markers: true,
            eval_shading: true,
            text_tab: TextTab::default(),
            outline: OutlineOptions::default(),
            outline_indent: true,
            outline_colour: true,
            checks: CheckSet::default(),
            default_doc_language: "en".to_owned(),
            page_setup: PageSetup::default(),
            export: ExportOptions::default(),
        }
    }
}
