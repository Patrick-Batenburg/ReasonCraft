//! The Help tab: a help topic or the contents, in the UI language, with
//! Back and Contents above it.

use eframe::egui::{Button, Label, RichText, ScrollArea, Ui};

use crate::actions::Action;
use crate::canvas::input::Os;
use crate::help::render::{self, Block, Span, parse};
use crate::help::{HelpPage, HelpState, TOPICS, keyboard, source, title};
use crate::i18n::{t, ui_language};
use crate::theme::Tokens;

/// The topics listed under "Concepts" in the contents; the rest are
/// how-tos.
const CONCEPTS: usize = 10;

pub(super) fn show(ui: &mut Ui, help: &HelpState, os: Os, tk: &Tokens, out: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let back = ui.add_enabled(help.can_go_back(), Button::new(t("help.back")));
        if back.clicked() {
            out.push(Action::HelpBack);
        }
        let on = help.page == HelpPage::Contents;
        if ui.add(Button::selectable(on, t("help.contents"))).clicked() && !on {
            out.push(Action::OpenHelp(HelpPage::Contents));
        }
    });
    ui.add_space(6.0);
    // Each page scrolls on its own, from its top.
    let scroll = ScrollArea::vertical()
        .id_salt(("help", help.page))
        .auto_shrink([false, false]);
    scroll.show(ui, |ui| {
        ui.set_width(ui.available_width());
        let open = match help.page {
            HelpPage::Contents => contents(ui, tk),
            HelpPage::Topic(topic) => {
                let blocks = parse(source(ui_language(), topic));
                let open = render::show(ui, &blocks, tk);
                if topic == "keyboard" {
                    shortcut_table(ui, os, tk);
                }
                open
            }
        };
        if let Some(topic) = open {
            out.push(Action::OpenHelp(HelpPage::Topic(topic)));
        }
    });
}

/// Every topic as a link, concepts first, then how-tos; returns the topic
/// clicked.
fn contents(ui: &mut Ui, tk: &Tokens) -> Option<&'static str> {
    let lang = ui_language();
    let mut open = None;
    ui.spacing_mut().item_spacing.y = 5.0;
    let heading = |ui: &mut Ui, key: &str| {
        let text = RichText::new(t(key)).size(15.5).strong().color(tk.box_text);
        ui.add(Label::new(text).wrap().selectable(false));
    };
    for (key, topics) in [
        ("help.contents.concepts", &TOPICS[..CONCEPTS]),
        ("help.contents.howto", &TOPICS[CONCEPTS..]),
    ] {
        heading(ui, key);
        for &topic in topics {
            if ui.link(title(lang, topic)).clicked() {
                open = Some(topic);
            }
        }
        ui.add_space(10.0);
    }
    open
}

/// The keys of this platform, from the shortcut table: a heading, then
/// one list item per command, its keys in bold (the UI font has the macOS
/// key symbols) and what they do. A list reads well in a narrow panel.
fn shortcut_table(ui: &mut Ui, os: Os, tk: &Tokens) {
    let mut blocks = vec![Block::Heading(
        2,
        vec![Span::Text(t("help.keyboard.table"))],
    )];
    blocks.extend(
        keyboard::rows(os).into_iter().map(|(what, keys)| {
            Block::Item(vec![Span::Bold(keys), Span::Text(format!(": {what}"))])
        }),
    );
    ui.add_space(4.0);
    render::show(ui, &blocks, tk);
}
