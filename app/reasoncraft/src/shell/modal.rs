//! The window-wide modal: a file error, or "save changes before closing?".

use eframe::egui;

use crate::actions::{Action, UnsavedChoice};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops::Modal;
use crate::i18n::{FluentArgs, t, t_args};

/// Draw the open modal, if any, and queue the user's answer.
pub fn show(ctx: &egui::Context, app: &mut ReasonCraftApp) {
    let Some(modal) = app.modal.clone() else {
        return;
    };
    let tk = app.tokens;
    let frame = egui::Frame::popup(&ctx.global_style())
        .fill(tk.panel)
        .inner_margin(egui::Margin::same(20));
    let response = egui::Modal::new(egui::Id::unique("file_modal"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            let (heading, body) = match &modal {
                Modal::Error { title, message } => (title.clone(), message.clone()),
                Modal::Unsaved { tab, .. } => {
                    let mut args = FluentArgs::new();
                    let name = app.tabs.get(*tab).map(DocTab::name).unwrap_or_default();
                    args.set("name", name);
                    (t_args("unsaved.title", &args), t("unsaved.message"))
                }
            };
            ui.label(
                egui::RichText::new(heading)
                    .strong()
                    .size(16.0)
                    .color(tk.box_text),
            );
            ui.label(egui::RichText::new(body).color(tk.box_text));
            let buttons: &[(&str, Action)] = match modal {
                Modal::Error { .. } => &[("modal.ok", Action::DismissModal)],
                // Laid out right to left: Cancel, Don't save, Save.
                Modal::Unsaved { .. } => &[
                    (
                        "unsaved.cancel",
                        Action::ResolveUnsaved(UnsavedChoice::Cancel),
                    ),
                    (
                        "unsaved.discard",
                        Action::ResolveUnsaved(UnsavedChoice::Discard),
                    ),
                    ("unsaved.save", Action::ResolveUnsaved(UnsavedChoice::Save)),
                ],
            };
            let mut answer = None;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (key, action) in buttons {
                    if ui.button(t(key)).clicked() {
                        answer = Some(action.clone());
                    }
                }
            });
            answer
        });
    // Esc or a click outside the modal dismisses / cancels it.
    let dismiss = || match modal {
        Modal::Error { .. } => Action::DismissModal,
        Modal::Unsaved { .. } => Action::ResolveUnsaved(UnsavedChoice::Cancel),
    };
    let close = response.should_close();
    let answer = response.inner.or_else(|| close.then(dismiss));
    if let Some(action) = answer {
        app.push(action);
    }
}
