//! The application: open tabs, settings and the per-frame loop.

use crate::actions::{self, Action, FakeOpener, LinkOpener, SystemOpener};
use crate::canvas::images::ImageCache;
use crate::canvas::measure::MeasureCache;
use crate::doc_tab::DocTab;
use crate::file_ops::{FakeDialogs, FileDialogs, Modal, RfdDialogs};
pub use crate::settings::{SETTINGS_KEY, Settings};
use crate::shell;
use crate::shell::link_modal::LinkEdit;
use crate::theme::{self, Tokens};
use eframe::egui;

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
    /// Personal preferences (theme, author, evaluation display).
    pub settings: Settings,
    pub left_panel: bool,
    pub right_panel: bool,
    /// Actions queued by widgets this frame, applied at the end of the frame.
    pub pending: Vec<Action>,
    pub ribbon_tab: RibbonTab,
    /// The tokens for the current theme, resolved at the start of each frame.
    pub tokens: Tokens,
    /// Text heights measured by the canvas, shared by all tabs.
    pub measure_cache: MeasureCache,
    /// Textures of box images, shared by all tabs.
    pub images: ImageCache,
    /// File pickers: the platform's own, or fakes in tests.
    pub dialogs: Box<dyn FileDialogs>,
    /// The error message or unsaved-changes prompt being shown.
    pub modal: Option<Modal>,
    /// The link editor, when open.
    pub link_edit: Option<LinkEdit>,
    /// Opens links in the browser: the platform's handler, or a fake in tests.
    pub link_opener: Box<dyn LinkOpener>,
    applied: Option<Tokens>,
    window_title: String,
    /// Every unsaved-changes question was answered: let the window close.
    close_confirmed: bool,
    /// Send the close command at the end of this frame.
    close_now: bool,
    /// Show the source icon preview window.
    #[cfg(debug_assertions)]
    pub show_source_icons: bool,
}

impl ReasonCraftApp {
    fn with_settings(
        settings: Settings,
        dialogs: Box<dyn FileDialogs>,
        link_opener: Box<dyn LinkOpener>,
    ) -> Self {
        Self {
            tabs: vec![DocTab::with_language(&settings.default_doc_language)],
            active: 0,
            tokens: theme::resolve(settings.theme, true),
            settings,
            left_panel: true,
            right_panel: true,
            pending: Vec::new(),
            ribbon_tab: RibbonTab::default(),
            measure_cache: MeasureCache::default(),
            images: ImageCache::default(),
            dialogs,
            modal: None,
            link_edit: None,
            link_opener,
            applied: None,
            window_title: String::new(),
            close_confirmed: false,
            close_now: false,
            #[cfg(debug_assertions)]
            show_source_icons: false,
        }
    }

    /// Build the app, restoring settings from eframe storage.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, SETTINGS_KEY))
            .unwrap_or_default();
        Self::with_settings(settings, Box::new(RfdDialogs), Box::new(SystemOpener))
    }

    /// Build the app with default settings, no storage, file dialogs that
    /// always cancel and a link opener that only records links.
    pub fn new_for_test() -> Self {
        Self::new_for_test_with(Box::new(FakeDialogs::default()))
    }

    /// Like [`Self::new_for_test`], with the given file dialogs.
    pub fn new_for_test_with(dialogs: Box<dyn FileDialogs>) -> Self {
        Self::with_settings(
            Settings::default(),
            dialogs,
            Box::new(FakeOpener::default()),
        )
    }

    /// True once the user has answered every unsaved-changes question of a
    /// window close; the window then closes.
    pub fn close_confirmed(&self) -> bool {
        self.close_confirmed
    }

    /// Let the window close (at the end of this frame).
    pub(crate) fn confirm_close(&mut self) {
        self.close_confirmed = true;
        self.close_now = true;
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
        // The OS (or the title bar on macOS) asks to close: ask about unsaved
        // changes first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending.push(Action::CloseWindow);
        }
        // Shortcuts are taken out first, so no widget sees (or moves focus
        // with) them, but run after the widgets' own actions (see `handle_keys`).
        // A modal takes all keys (Esc closes it).
        let modal = self.modal.is_some() || self.link_edit.is_some();
        let keys = if self.ribbon_tab == RibbonTab::File || modal {
            Vec::new()
        } else {
            crate::canvas::input::handle_keys(&ctx, self)
        };

        shell::titlebar::show(ui, self);
        shell::tabs::show(ui, self);
        shell::ribbon::show(ui, self);
        shell::statusbar::show(ui, self);
        shell::show_side_panels(ui, self);
        shell::show_central(ui, self);
        shell::link_modal::show(&ctx, self);
        shell::modal::show(&ctx, self);
        #[cfg(debug_assertions)]
        shell::source_icons::preview(&ctx, self);
        shell::resize::show(&ctx);
        self.pending.extend(keys);

        let actions = std::mem::take(&mut self.pending);
        if !actions.is_empty() {
            for action in actions {
                actions::apply(self, action);
            }
            ctx.request_repaint();
        }
        if std::mem::take(&mut self.close_now) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        let os_dark = ctx.system_theme().is_none_or(|t| t == egui::Theme::Dark);
        self.tokens = theme::resolve(self.settings.theme, os_dark);
        if self.applied != Some(self.tokens) {
            // Ctrl+=/-/0 zoom the map, not the whole interface.
            ctx.options_mut(|o| o.zoom_with_keyboard = false);
            theme::apply(ctx, &self.tokens);
            self.applied = Some(self.tokens);
        }
    }

    fn sync_window_title(&mut self, ctx: &egui::Context) {
        let mut args = crate::i18n::FluentArgs::new();
        args.set("doc", self.active_tab().title());
        args.set("app", crate::i18n::t("app.name"));
        let title = crate::i18n::t_args("window.title", &args);
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
        eframe::set_value(storage, SETTINGS_KEY, &self.settings);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.tokens.bg.to_normalized_gamma_f32()
    }
}
