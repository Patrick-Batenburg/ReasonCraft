//! The Text panel on the right: a tab strip (Outline, Essay, Check,
//! Snippets) and the active tab's body.

use eframe::egui::{self, Stroke, Ui};

mod check;
mod drag_chip;
mod essay;
mod essay_rows;
mod essay_slot;
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
use crate::i18n::t;
use crate::settings::{Settings, TextTab};
use crate::spell::SpellFields;
use crate::theme::Tokens;

const STRIP_H: f32 = 28.0;
/// Room around each tab label; with the panel's minimum width the four
/// tabs fit on one line (longer labels wrap).
const TAB_PAD: f32 = 14.0;

const TABS: [(TextTab, &str); 4] = [
    (TextTab::Outline, "text.tab.outline"),
    (TextTab::Essay, "text.tab.essay"),
    (TextTab::Check, "text.tab.check"),
    (TextTab::Snippets, "text.tab.snippets"),
];

/// Draw the panel's contents and queue the user's commands in `out`.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    settings: &Settings,
    os: Os,
    tk: &Tokens,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    tab_strip(ui, settings.text_tab, tk, out);
    ui.add_space(8.0);
    match settings.text_tab {
        TextTab::Outline => outline::show(ui, tab, settings, tk, out),
        TextTab::Check => check::show(ui, tab, settings, tk, out),
        TextTab::Snippets => snippets::show(ui, tab, os, tk, spell, out),
        TextTab::Essay => essay::show(ui, tab, settings, tk, spell, out),
    }
}

fn tab_strip(ui: &mut Ui, active: TextTab, tk: &Tokens, out: &mut Vec<Action>) {
    let strip = ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
        for (tab, key) in TABS {
            let resp = tab_button(ui, tk, &t(key), tab == active, tk.panel, TAB_PAD, STRIP_H);
            if resp.clicked() && tab != active {
                out.push(Action::SetTextTab(tab));
            }
        }
    });
    let bottom = strip.response.rect.bottom();
    let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.5));
    ui.painter().hline(ui.max_rect().x_range(), bottom, line);
}
