//! The window-wide modal: a file error, "save changes before closing?",
//! "replace the existing file?" (for a save or an export) or "restore the
//! documents a crash left behind?".

use eframe::egui::{self, Role, WidgetInfo};

use crate::actions::{Action, UnsavedChoice};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops::Modal;
use crate::i18n::{FluentArgs, t, t_args};
use crate::recovery::{RecoveredDoc, RecoveryChoice};

/// A modal button: visible text key, accessible name key, action.
type Button = (&'static str, &'static str, Action);

/// Draw the open modal, if any, and queue the user's answer.
pub fn show(ctx: &egui::Context, app: &mut ReasonCraftApp) {
    let Some(modal) = app.modal.clone() else {
        return;
    };
    let tk = app.tokens;
    let (heading, body) = texts(app, &modal);
    let (buttons, dismiss) = buttons(&modal);
    let frame = egui::Frame::popup(&ctx.global_style())
        .fill(tk.panel)
        .inner_margin(egui::Margin::same(20));
    let response = egui::Modal::new(egui::Id::unique("file_modal"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(
                egui::RichText::new(heading)
                    .strong()
                    .size(16.0)
                    .color(tk.box_text),
            );
            ui.label(egui::RichText::new(body).color(tk.box_text));
            // Save (or the close) goes ahead by itself once the work is done.
            let waiting = match modal {
                Modal::Unsaved { waiting: true, .. } => Some("unsaved.waiting_images"),
                Modal::ExportsRunning => Some("closing.exports.waiting"),
                _ => None,
            };
            if let Some(key) = waiting {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(14.0).color(tk.accent));
                    ui.label(egui::RichText::new(t(key)).color(tk.box_text));
                });
            }
            let mut answer = None;
            // Laid out right to left, so the first button sits rightmost.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (text, name, action) in buttons {
                    let resp = ui.button(t(text));
                    // A name of its own, so it differs from the title bar's Save.
                    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, t(name)));
                    if resp.clicked() {
                        answer = Some(action);
                    }
                }
            });
            answer
        });
    // Esc or a click outside the modal dismisses / cancels it.
    let close = response.should_close();
    let answer = response.inner.or_else(|| close.then_some(dismiss));
    if let Some(action) = answer {
        app.push(action);
    }
}

/// The heading and body text of `modal`.
fn texts(app: &ReasonCraftApp, modal: &Modal) -> (String, String) {
    match modal {
        Modal::Error { title, message } => (title.clone(), message.clone()),
        Modal::Unsaved { tab, .. } => {
            let mut args = FluentArgs::new();
            let name = app.tabs.get(*tab).map(DocTab::name).unwrap_or_default();
            args.set("name", name);
            (t_args("unsaved.title", &args), t("unsaved.message"))
        }
        Modal::ExportsRunning => (t("closing.exports.title"), t("closing.exports.message")),
        Modal::ConfirmReplace { path, .. } | Modal::ConfirmExportReplace { path, .. } => {
            let mut args = FluentArgs::new();
            let file = path.file_name().unwrap_or(path.as_os_str());
            args.set("file", file.to_string_lossy().into_owned());
            let message = match modal {
                Modal::ConfirmReplace {
                    original_changed: true,
                    ..
                } => "recovery.original_changed",
                _ => "replace.message",
            };
            (t_args("replace.title", &args), t(message))
        }
        Modal::Recover(docs) => (t("recovery.title"), recover_body(docs)),
    }
}

/// The recovery prompt's text: one line per document with its name, file
/// and when it was autosaved.
fn recover_body(docs: &[RecoveredDoc]) -> String {
    let mut body = t("recovery.message");
    for doc in docs {
        let mut args = FluentArgs::new();
        args.set("title", doc.meta.title.clone());
        let file = doc
            .meta
            .original
            .as_ref()
            .map_or_else(|| t("recovery.never_saved"), |p| p.display().to_string());
        args.set("file", file);
        let time = doc.meta.saved_at.with_timezone(&chrono::Local);
        args.set("time", time.format("%Y-%m-%d %H:%M").to_string());
        body.push_str(
            "
• ",
        );
        body.push_str(&t_args("recovery.entry", &args));
    }
    body
}

/// The buttons of `modal`, and the action for Esc or a click outside it.
fn buttons(modal: &Modal) -> (Vec<Button>, Action) {
    let unsaved = Action::ResolveUnsaved;
    match modal {
        Modal::Error { .. } => (
            vec![("modal.ok", "modal.ok", Action::DismissModal)],
            Action::DismissModal,
        ),
        Modal::Unsaved { waiting, .. } => {
            let mut buttons = vec![
                (
                    "unsaved.cancel",
                    "unsaved.cancel",
                    unsaved(UnsavedChoice::Cancel),
                ),
                (
                    "unsaved.discard",
                    "unsaved.discard",
                    unsaved(UnsavedChoice::Discard),
                ),
            ];
            // While Save waits for images, it is already chosen.
            if !waiting {
                buttons.push((
                    "unsaved.save",
                    "unsaved.save_changes",
                    unsaved(UnsavedChoice::Save),
                ));
            }
            (buttons, unsaved(UnsavedChoice::Cancel))
        }
        Modal::ConfirmReplace { .. } => (
            vec![
                (
                    "replace.cancel",
                    "replace.cancel",
                    Action::ResolveReplace(false),
                ),
                (
                    "replace.confirm",
                    "replace.confirm",
                    Action::ResolveReplace(true),
                ),
            ],
            Action::ResolveReplace(false),
        ),
        Modal::ExportsRunning => (
            vec![
                (
                    "unsaved.cancel",
                    "unsaved.cancel",
                    Action::ResolveClosingExports(false),
                ),
                (
                    "closing.exports.quit",
                    "closing.exports.quit",
                    Action::ResolveClosingExports(true),
                ),
            ],
            Action::ResolveClosingExports(false),
        ),
        Modal::Recover(_) => {
            let answer = Action::ResolveRecovery;
            (
                vec![
                    (
                        "recovery.later",
                        "recovery.later",
                        answer(RecoveryChoice::Later),
                    ),
                    (
                        "recovery.discard_all",
                        "recovery.discard_all",
                        answer(RecoveryChoice::DiscardAll),
                    ),
                    (
                        "recovery.restore_all",
                        "recovery.restore_all",
                        answer(RecoveryChoice::RestoreAll),
                    ),
                ],
                answer(RecoveryChoice::Later),
            )
        }
        Modal::ConfirmExportReplace { .. } => (
            vec![
                (
                    "replace.cancel",
                    "replace.cancel",
                    Action::ResolveExportReplace(false),
                ),
                (
                    "replace.confirm",
                    "replace.confirm",
                    Action::ResolveExportReplace(true),
                ),
            ],
            Action::ResolveExportReplace(false),
        ),
    }
}
