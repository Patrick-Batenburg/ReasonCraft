//! The Text panel on the right: a tab strip (Outline, Essay, Check,
//! Snippets, Help) and the active tab's body.

use eframe::egui::{self, Color32, Role, Stroke, Ui, WidgetInfo};

mod check;
mod drag_chip;
mod essay;
mod essay_rows;
mod essay_slot;
mod help;
mod outline;
mod snippet_card;
mod snippets;
mod virtual_rows;

pub use check::{check_help, check_key, item_text};
pub use drag_chip::{chip_label, excerpt};
pub use snippet_card::capped_notice;

use super::tab_button;
use crate::actions::Action;
use crate::canvas::input::Os;
use crate::doc_tab::DocTab;
use crate::help::HelpState;
use crate::i18n::t;
use crate::settings::{Settings, TextTab};
use crate::spell::SpellFields;
use crate::theme::Tokens;

const STRIP_H: f32 = 28.0;
/// Room around each tab label, at most and at least: the room shrinks so
/// the tabs fit on one line (at the panel's default width in English and
/// Dutch); in a narrower panel they wrap.
const TAB_PAD: f32 = 14.0;
const TAB_PAD_MIN: f32 = 6.0;
/// The gap between two tabs.
const TAB_GAP: f32 = 2.0;

const TABS: [(TextTab, &str); 5] = [
    (TextTab::Outline, "text.tab.outline"),
    (TextTab::Essay, "text.tab.essay"),
    (TextTab::Check, "text.tab.check"),
    (TextTab::Snippets, "text.tab.snippets"),
    (TextTab::Help, "text.tab.help"),
];

/// The panel's state beyond the document: the settings, the platform's
/// keys and the Help tab's page.
pub struct PanelContext<'a> {
    pub settings: &'a Settings,
    pub os: Os,
    pub help: &'a HelpState,
}

/// Draw the panel's contents and queue the user's commands in `out`.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    cx: PanelContext,
    tk: &Tokens,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    let PanelContext { settings, os, help } = cx;
    tab_strip(ui, settings.text_tab, tk, out);
    ui.add_space(8.0);
    match settings.text_tab {
        TextTab::Outline => outline::show(ui, tab, settings, tk, out),
        TextTab::Check => check::show(ui, tab, settings, tk, out),
        TextTab::Snippets => snippets::show(ui, tab, os, tk, spell, out),
        TextTab::Essay => essay::show(ui, tab, settings, tk, spell, out),
        TextTab::Help => help::show(ui, help, os, tk, out),
    }
}

fn tab_strip(ui: &mut Ui, active: TextTab, tk: &Tokens, out: &mut Vec<Action>) {
    let labels = TABS.map(|(_, key)| t(key));
    let pad = tab_pad(ui, &labels);
    let strip = ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(TAB_GAP, 0.0);
        for ((tab, _), label) in TABS.into_iter().zip(labels) {
            let resp = tab_button(ui, tk, &label, tab == active, tk.panel, pad, STRIP_H);
            if tab == TextTab::Help {
                // Named apart from the ribbon's Help tab.
                let name = crate::a11y::in_group(&t("action.panel_right"), &label);
                resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, tab == active, &name));
            }
            if resp.clicked() && tab != active {
                out.push(Action::SetTextTab(tab));
            }
        }
    });
    let bottom = strip.response.rect.bottom();
    let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.5));
    ui.painter().hline(ui.max_rect().x_range(), bottom, line);
}

/// The room around each tab label that fits all of `labels` on one line,
/// within [`TAB_PAD_MIN`]..=[`TAB_PAD`] (as [`tab_button`] measures them).
fn tab_pad(ui: &Ui, labels: &[String]) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let text: f32 = labels
        .iter()
        .map(|l| {
            let galley = ui
                .painter()
                .layout_no_wrap(l.clone(), font.clone(), Color32::PLACEHOLDER);
            galley.size().x
        })
        .sum();
    let n = labels.len() as f32;
    let room = ui.available_width() - text - TAB_GAP * (n - 1.0);
    // A little slack, so rounding never pushes the last tab to a new line.
    ((room - 1.0) / n).clamp(TAB_PAD_MIN, TAB_PAD)
}
