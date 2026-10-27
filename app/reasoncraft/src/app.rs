//! The application: open tabs, settings and the per-frame loop.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::actions::{self, Action, FakeOpener, LinkOpener, SystemOpener};
use crate::canvas::images::{ImageCache, ImageJobs};
use crate::canvas::input::Os;
use crate::canvas::measure::MeasureCache;
use crate::doc_tab::DocTab;
use crate::export::{ExportJobs, FakeClipboard, SharedClipboard, SystemClipboard};
use crate::file_ops::{FakeDialogs, FileDialogs, Modal, RfdDialogs};
pub use crate::settings::{SETTINGS_KEY, Settings};
use crate::shell;
use crate::shell::export_dialog::ExportDialog;
use crate::shell::link_modal::LinkEdit;
use crate::spell::SpellService;
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
    /// Reads the OS locale, for a UI language that follows the OS: the
    /// platform's own in the app; none in tests, which then read English.
    pub os_locale: fn() -> Option<String>,
    /// The platform whose keys apply: this one, or another in tests.
    pub os: Os,
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
    /// Exports running in the background, and the last one's notice.
    pub export: ExportJobs,
    /// Inserted images being prepared in the background.
    pub image_jobs: ImageJobs,
    /// Spelling dictionaries (loaded in the background) and the
    /// misspellings found, shared by all tabs.
    pub spell: SpellService,
    /// A short message in the status bar (an inserted image that was
    /// discarded, say). It and the export notice go after
    /// [`shell::statusbar::NOTE_SECS`] or with the next action.
    pub status_note: Option<String>,
    /// The status note and export notice on show, and since when (egui
    /// time), so they can expire.
    pub(crate) notes_shown: Option<(f64, Option<String>, Option<crate::export::Notice>)>,
    /// The recovery folder session autosave writes to; `None` when autosave
    /// is unavailable (and in tests, unless asked for).
    pub recovery: Option<crate::recovery::RecoveryStore>,
    /// When the last autosave ran (egui time), or `None` before the first
    /// tick.
    pub(crate) autosave_clock: Option<f64>,
    /// The recovery prompt, waiting for another message to be closed first.
    pub(crate) recovery_offer: Option<Vec<crate::recovery::RecoveredDoc>>,
    /// The export or page setup dialog, when open.
    pub export_dialog: Option<ExportDialog>,
    /// Where the panes are this frame, and an F6 move asked for.
    pub(crate) panes: shell::panes::Panes,
    /// The context exports measure text in (at the display's pixels per
    /// point, so they match the screen at 100 % but not the UI scale), and
    /// its cache.
    pub(crate) export_measure: (egui::Context, MeasureCache),
    /// The egui context of the running frame.
    pub(crate) egui_ctx: Option<egui::Context>,
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
        os_locale: fn() -> Option<String>,
        dialogs: Box<dyn FileDialogs>,
        link_opener: Box<dyn LinkOpener>,
        clipboard: SharedClipboard,
    ) -> Self {
        let app = Self {
            tabs: vec![DocTab::with_language(&settings.default_doc_language)],
            active: 0,
            tokens: theme::resolve(settings.theme, true),
            spell: SpellService::new(&settings.spell_words),
            settings,
            os_locale,
            os: Os::current(),
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
            export: ExportJobs::new(clipboard, std::env::temp_dir()),
            image_jobs: ImageJobs::default(),
            status_note: None,
            notes_shown: None,
            recovery: None,
            autosave_clock: None,
            recovery_offer: None,
            export_dialog: None,
            panes: shell::panes::Panes::default(),
            export_measure: (egui::Context::default(), MeasureCache::default()),
            egui_ctx: None,
            applied: None,
            window_title: String::new(),
            close_confirmed: false,
            close_now: false,
            #[cfg(debug_assertions)]
            show_source_icons: false,
        };
        app.apply_ui_language();
        app
    }

    /// Show the interface in the language the settings ask for. The OS
    /// locale is read only when they follow the OS.
    pub(crate) fn apply_ui_language(&self) {
        let lang = match &self.settings.ui_language {
            Some(lang) => lang.as_str(),
            None => crate::i18n::os_ui_language((self.os_locale)().as_deref()),
        };
        crate::i18n::set_ui_language(lang);
    }

    /// Build the app, restoring settings from eframe storage, with autosave
    /// to `recovery_dir` (see [`crate::recovery`]). `os_locale` reads the
    /// OS locale (a BCP 47 tag such as `nl-NL`), called only while the UI
    /// language follows the OS.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        recovery_dir: Option<PathBuf>,
        os_locale: fn() -> Option<String>,
    ) -> Self {
        let settings = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, SETTINGS_KEY))
            .unwrap_or_default();
        // Print files from earlier runs; off the UI thread, as the temp
        // folder can be large.
        std::thread::spawn(crate::export::clean_old_print_files);
        let mut app = Self::with_settings(
            settings,
            os_locale,
            Box::new(RfdDialogs),
            Box::new(SystemOpener),
            Arc::new(Mutex::new(SystemClipboard::default())),
        );
        app.start_recovery(recovery_dir);
        app
    }

    /// Start autosaving to `dir` and offer what crashed sessions left
    /// there. Without a usable folder the app runs without autosave and
    /// says so in the status bar.
    fn start_recovery(&mut self, dir: Option<PathBuf>) {
        match dir.map(crate::recovery::RecoveryStore::open) {
            Some(Ok(store)) => {
                self.recovery = Some(store);
                crate::recovery::offer(self);
            }
            _ => self.status_note = Some(crate::i18n::t("recovery.unavailable")),
        }
    }

    /// Build the app with default settings, no storage, no OS locale (so
    /// the interface is English), file dialogs that always cancel, and a
    /// link opener and clipboard that only record.
    pub fn new_for_test() -> Self {
        Self::new_for_test_with(Box::new(FakeDialogs::default()))
    }

    /// Like [`Self::new_for_test`], with the given file dialogs.
    pub fn new_for_test_with(dialogs: Box<dyn FileDialogs>) -> Self {
        Self::with_settings(
            Settings::default(),
            || None,
            dialogs,
            Box::new(FakeOpener::default()),
            Arc::new(Mutex::new(FakeClipboard::default())),
        )
    }

    /// Like [`Self::new_for_test`], with autosave to the recovery folder
    /// `dir` (a temp folder: never the user's).
    pub fn new_for_test_with_recovery(dir: PathBuf) -> Self {
        let mut app = Self::new_for_test();
        app.start_recovery(Some(dir));
        app
    }

    /// True once the user has answered every unsaved-changes question of a
    /// window close; the window then closes.
    pub fn close_confirmed(&self) -> bool {
        self.close_confirmed
    }

    /// Let the window close (at the end of this frame). This is a normal
    /// exit: the recovery session ends with it.
    pub(crate) fn confirm_close(&mut self) {
        crate::recovery::shutdown(self);
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
        self.egui_ctx = Some(ctx.clone());
        actions::poll_exports(self);
        actions::poll_images(self);
        crate::spell::poll(self);
        shell::statusbar::expire_notes(&ctx, self);
        self.sync_theme(&ctx);
        self.sync_ui_scale(&ctx);
        self.sync_window_title(&ctx);
        // The OS (or the title bar on macOS) asks to close, or Cmd+Q on
        // macOS: ask about unsaved changes first.
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && !self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending.push(Action::CloseWindow);
        } else if self.quit_key(&ctx) {
            self.pending.push(Action::CloseWindow);
        }
        // Shortcuts are taken out first, so no widget sees (or moves focus
        // with) them, but run after the widgets' own actions (see `handle_keys`).
        // A modal takes all keys (Esc closes it). In the backstage only a
        // focused field (the document title or author) passes on the file
        // keys; F6 / Shift+F6 always move between the panes.
        let modal =
            self.modal.is_some() || self.link_edit.is_some() || self.export_dialog.is_some();
        let keys = if modal {
            Vec::new()
        } else if self.ribbon_tab == RibbonTab::File {
            if ctx.text_edit_focused() {
                crate::canvas::input::file_keys(&ctx, self)
            } else {
                shell::panes::pane_keys(&ctx)
            }
        } else {
            crate::canvas::input::handle_keys(&ctx, self)
        };

        self.panes.start_frame();
        shell::titlebar::show(ui, self);
        shell::tabs::show(ui, self);
        shell::ribbon::show(ui, self);
        shell::statusbar::show(ui, self);
        shell::show_side_panels(ui, self);
        shell::show_central(ui, self);
        self.panes.finish_frame(&ctx);
        shell::link_modal::show(&ctx, self);
        shell::export_dialog::show(&ctx, self);
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
        let now = ctx.input(|i| i.time);
        crate::recovery::tick(self, now);
        if let Some(secs) = crate::recovery::due_in(self, now) {
            ctx.request_repaint_after(Duration::from_secs_f64(secs));
        }
        if self.export.busy() || self.image_jobs.busy() || self.spell.busy() {
            // Look for the result again soon, without spinning.
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        if std::mem::take(&mut self.close_now) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Takes Cmd+Q out of the input on macOS: quitting closes the window
    /// (the app menu, which would quit at once, is off; see `main.rs`).
    fn quit_key(&self, ctx: &egui::Context) -> bool {
        let quit = |i: &mut egui::InputState| i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::Q);
        self.os == Os::Mac && ctx.input_mut(quit)
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

    /// The whole interface at the size the settings ask for (egui's zoom
    /// factor). The map's zoom works in points, so it comes on top.
    fn sync_ui_scale(&self, ctx: &egui::Context) {
        let scale = self.settings.ui_scale_factor();
        if ctx.zoom_factor() != scale {
            ctx.set_zoom_factor(scale);
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
