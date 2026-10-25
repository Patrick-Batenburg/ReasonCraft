//! Personal preferences that survive restarts.

use serde::{Deserialize, Serialize};

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
        }
    }
}
