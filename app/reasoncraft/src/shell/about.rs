//! The backstage About page: the app's name, version and licence, credits
//! to the dictionary projects, and the third-party notices.

use std::sync::LazyLock;

use eframe::egui::{self, Label, RichText};

use crate::app::ReasonCraftApp;
use crate::help::render::{self, Block, parse};
use crate::i18n::{FluentArgs, t, t_args};

/// `THIRD-PARTY.md`, embedded as it is in the repository.
pub const THIRD_PARTY: &str = include_str!("../../../../THIRD-PARTY.md");

/// [`THIRD_PARTY`] read into blocks, once.
static THIRD_PARTY_BLOCKS: LazyLock<Vec<Block>> = LazyLock::new(|| parse(THIRD_PARTY));

/// The licence ReasonCraft is under.
pub const LICENCE: &str = "MIT OR Apache-2.0";

pub(super) fn page(ui: &mut egui::Ui, app: &ReasonCraftApp) {
    let tk = app.tokens;
    ui.set_max_width(640.0);
    ui.spacing_mut().item_spacing.y = 8.0;
    let name = format!("{} {}", t("app.name"), env!("CARGO_PKG_VERSION"));
    let title = RichText::new(name).strong().size(18.0).color(tk.box_text);
    ui.add(Label::new(title).selectable(false));
    let body = |key: &str, args: Option<&FluentArgs>| {
        let text = args.map_or_else(|| t(key), |a| t_args(key, a));
        RichText::new(text).color(tk.box_text)
    };
    ui.add(Label::new(body("about.tagline", None)).wrap());
    let mut args = FluentArgs::new();
    args.set("licence", LICENCE);
    ui.add(Label::new(body("about.licence", Some(&args))).wrap());
    ui.add(Label::new(body("about.credits", None)).wrap());
    ui.add_space(8.0);
    ui.separator();
    // The notices hold no topic links: nothing in them is clickable.
    render::show(ui, &THIRD_PARTY_BLOCKS, &tk);
}
