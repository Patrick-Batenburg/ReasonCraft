//! The application: open tabs, settings and the per-frame loop.

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::actions::{self, Action};
use crate::canvas::measure::MeasureCache;
use crate::doc_tab::DocTab;
use crate::shell;
use crate::theme::{self, ThemeChoice, Tokens};

/// eframe storage key for [`Settings`].
pub const SETTINGS_KEY: &str = "settings";

/// Personal preferences that survive restarts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeChoice,
}

/// Which ribbon tab is showing. `File` shows the backstage instead of the map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RibbonTab {
    File,
    #[default]
    Home,
    Insert,
    Evaluate,
    View,
    Export,
    Help,
}

pub struct ReasonCraftApp {
    pub tabs: Vec<DocTab>,
    pub active: usize,
    pub theme: ThemeChoice,
    pub left_panel: bool,
    pub right_panel: bool,
    /// Actions queued by widgets this frame, applied at the end of the frame.
    pub pending: Vec<Action>,
    pub ribbon_tab: RibbonTab,
    /// The tokens for the current theme, resolved at the start of each frame.
    pub tokens: Tokens,
    /// Text heights measured by the canvas, shared by all tabs.
    pub measure_cache: MeasureCache,
    applied: Option<Tokens>,
    window_title: String,
}

impl ReasonCraftApp {
    fn with_settings(settings: Settings) -> Self {
        Self {
            tabs: vec![DocTab::new()],
            active: 0,
            theme: settings.theme,
            left_panel: true,
            right_panel: true,
            pending: Vec::new(),
            ribbon_tab: RibbonTab::default(),
            tokens: theme::resolve(settings.theme, true),
            measure_cache: MeasureCache::default(),
            applied: None,
            window_title: String::new(),
        }
    }

    /// Build the app, restoring settings from eframe storage.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, SETTINGS_KEY))
            .unwrap_or_default();
        Self::with_settings(settings)
    }

    /// Build the app with default settings and no storage.
    pub fn new_for_test() -> Self {
        Self::with_settings(Settings::default())
    }

    pub fn active_tab(&self) -> &DocTab {
        &self.tabs[self.active]
    }

    pub fn push(&mut self, action: Action) {
        self.pending.push(action);
    }

    /// Draw one frame and apply the actions it produced.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.sync_theme(&ctx);
        self.sync_window_title(&ctx);

        shell::titlebar::show(ui, self);
        shell::tabs::show(ui, self);
        shell::ribbon::show(ui, self);
        shell::statusbar::show(ui, self);
        shell::show_side_panels(ui, self);
        shell::show_central(ui, self);

        let actions = std::mem::take(&mut self.pending);
        if !actions.is_empty() {
            for action in actions {
                actions::apply(self, action);
            }
            ctx.request_repaint();
        }
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        let os_dark = ctx.system_theme().is_none_or(|t| t == egui::Theme::Dark);
        self.tokens = theme::resolve(self.theme, os_dark);
        if self.applied != Some(self.tokens) {
            theme::apply(ctx, &self.tokens);
            self.applied = Some(self.tokens);
        }
    }

    fn sync_window_title(&mut self, ctx: &egui::Context) {
        let title = format!(
            "{} - {}",
            self.active_tab().title(),
            crate::i18n::t("app.name")
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

impl eframe::App for ReasonCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SETTINGS_KEY, &Settings { theme: self.theme });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.tokens.bg.to_normalized_gamma_f32()
    }
}
