//! The edit-link modal: a URL field with Save, Remove link and Cancel.
//! Esc or a click outside cancels.

use eframe::egui::{self, Key, Role, TextEdit, WidgetInfo};
use reasoncraft_core::NodeId;

use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::i18n::t;

/// The link being edited: the box it belongs to and the field's text.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkEdit {
    pub node: NodeId,
    pub text: String,
    /// Shown under the field after a Save with an invalid link.
    pub error: Option<String>,
    /// The field should take keyboard focus on its next frame.
    pub focus: bool,
}

impl LinkEdit {
    pub fn new(node: NodeId, link: Option<&str>) -> Self {
        Self {
            node,
            text: link.unwrap_or_default().to_owned(),
            error: None,
            focus: true,
        }
    }
}

/// Why a link was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkError {
    Invalid,
}

impl LinkError {
    pub fn message(self) -> String {
        match self {
            LinkError::Invalid => t("link.invalid"),
        }
    }
}

/// Check a typed link. `http://`, `https://` and `mailto:` links are kept;
/// a bare domain such as `example.org/x` gets `https://`; empty means no
/// link. Anything else (other schemes, spaces) is refused.
pub fn normalize_link(s: &str) -> Result<Option<String>, LinkError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    if s.chars().any(char::is_whitespace) {
        return Err(LinkError::Invalid);
    }
    let lower = s.to_ascii_lowercase();
    for scheme in ["http://", "https://", "mailto:"] {
        if lower.starts_with(scheme) {
            let rest = &s[scheme.len()..];
            return if rest.is_empty() {
                Err(LinkError::Invalid)
            } else {
                Ok(Some(s.to_owned()))
            };
        }
    }
    let host_port = s.split(['/', '?', '#']).next().unwrap_or_default();
    let (host, port) = host_port.split_once(':').unwrap_or((host_port, ""));
    let host_ok = host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '.');
    let port_ok = !host_port.contains(':') || (!port.is_empty() && port.parse::<u16>().is_ok());
    if host_ok && port_ok {
        Ok(Some(format!("https://{s}")))
    } else {
        Err(LinkError::Invalid)
    }
}

/// Draw the link editor, if open, and queue the user's answer.
pub fn show(ctx: &egui::Context, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let has_link = app.link_edit.as_ref().is_some_and(|e| {
        app.active_tab()
            .doc
            .boxes
            .get(&e.node)
            .is_some_and(|n| n.link.is_some())
    });
    let Some(edit) = app.link_edit.as_mut() else {
        return;
    };
    let frame = egui::Frame::popup(&ctx.global_style())
        .fill(tk.panel)
        .inner_margin(egui::Margin::same(20));
    let response = egui::Modal::new(egui::Id::unique("link_modal"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(420.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            let heading = egui::RichText::new(t("link.title")).strong().size(16.0);
            ui.label(heading.color(tk.box_text));
            let field_id = egui::Id::unique("link_modal_url");
            let field = TextEdit::singleline(&mut edit.text)
                .id(field_id)
                .hint_text(t("link.hint"))
                .desired_width(f32::INFINITY);
            let resp = ui.add(field);
            let label = t("link.url");
            resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &label));
            if std::mem::take(&mut edit.focus) {
                resp.request_focus();
            }
            if resp.changed() {
                edit.error = None;
            }
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if let Some(err) = &edit.error {
                ui.label(egui::RichText::new(err).color(tk.objection));
            }
            let mut answer = None;
            // One row high, so a vanished error message leaves no gap.
            let row = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
            let rtl = egui::Layout::right_to_left(egui::Align::Center);
            ui.allocate_ui_with_layout(row, rtl, |ui| {
                let save = ui.button(t("link.save"));
                // A name of its own, so it differs from the title bar's Save.
                let name = t("link.save_link");
                save.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
                if save.clicked() || enter {
                    match normalize_link(&edit.text) {
                        Ok(link) => answer = Some(Action::SetLink(link)),
                        Err(e) => {
                            edit.error = Some(e.message());
                            // Back to the field, to fix the link.
                            edit.focus = true;
                        }
                    }
                }
                if ui.button(t("link.cancel")).clicked() {
                    answer = Some(Action::DismissModal);
                }
                let remove = ui.add_enabled(has_link, egui::Button::new(t("link.remove")));
                if remove.clicked() {
                    answer = Some(Action::SetLink(None));
                }
            });
            answer
        });
    let close = response.should_close();
    let answer = response
        .inner
        .or_else(|| close.then_some(Action::DismissModal));
    if let Some(action) = answer {
        app.push(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_link_cases() {
        let ok = |s: &str| Ok(Some(s.to_owned()));
        assert_eq!(normalize_link("https://a.b"), ok("https://a.b"));
        assert_eq!(normalize_link("http://a.b/x?y=1"), ok("http://a.b/x?y=1"));
        assert_eq!(normalize_link("  https://a.b  "), ok("https://a.b"));
        assert_eq!(normalize_link("a.b/c"), ok("https://a.b/c"));
        assert_eq!(
            normalize_link("example.org:8080/x"),
            ok("https://example.org:8080/x")
        );
        assert_eq!(normalize_link("mailto:x@y.z"), ok("mailto:x@y.z"));
        assert_eq!(normalize_link(""), Ok(None));
        assert_eq!(normalize_link("   "), Ok(None));
        for bad in [
            "javascript:alert(1)",
            "ftp://x",
            "file:///etc/passwd",
            "https://",
            "mailto:",
            "not a link",
            "localhost",
            "a.b/c d",
            ".b",
        ] {
            assert_eq!(normalize_link(bad), Err(LinkError::Invalid), "{bad}");
        }
        assert_eq!(LinkError::Invalid.message(), t("link.invalid"));
        assert_ne!(t("link.invalid"), "link.invalid");
    }
}
