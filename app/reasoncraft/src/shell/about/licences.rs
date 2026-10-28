//! The About page's Licences section: the full licence texts of what
//! ReasonCraft bundles, embedded in the binary. Each opens on its own and
//! scrolls in a box of its own, so the page stays short.

use std::sync::LazyLock;

use eframe::egui::{self, CollapsingHeader, Label, RichText, ScrollArea};

use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

/// The licences of the bundled fonts, dictionaries, spell checker and the
/// installer parts (the WiX UI and custom action in the MSI, the AppImage
/// runtime): the i18n key of each header, and the text.
pub const BUNDLED: [(&str, &str); 8] = [
    (
        "about.licences.ubuntu",
        include_str!("../../../assets/licences/ubuntu-font-licence.txt"),
    ),
    (
        "about.licences.hack",
        include_str!("../../../assets/licences/hack.txt"),
    ),
    (
        "about.licences.emoji",
        include_str!("../../../assets/licences/emoji-icon-font.txt"),
    ),
    (
        "about.licences.dict_en",
        include_str!("../../../../../crates/reasoncraft_spell/dict/en/LICENSE"),
    ),
    (
        "about.licences.dict_nl",
        include_str!("../../../../../crates/reasoncraft_spell/dict/nl/LICENSE"),
    ),
    (
        "about.licences.spellbook",
        include_str!("../../../assets/licences/spellbook.txt"),
    ),
    (
        "about.licences.wix",
        include_str!("../../../assets/licences/wix.txt"),
    ),
    (
        "about.licences.appimage",
        include_str!("../../../assets/licences/appimage-runtime.txt"),
    ),
];

/// The licence texts of the Rust crates the app is built from, written by
/// `cargo run -p xtask -- notices`.
pub const CRATES: &str = include_str!("../../../assets/licences/crates.txt");

/// The line that opens each licence text in [`CRATES`].
const RULE: &str = "========================================================================";

/// One licence text of [`CRATES`]: its name, the crates under it, and the
/// whole entry as written (name, crates and text).
pub struct CrateLicence {
    pub name: &'static str,
    pub used_by: &'static str,
    pub entry: &'static str,
}

/// [`CRATES`] split into its licence texts, once.
pub fn crate_licences() -> &'static [CrateLicence] {
    static SPLIT: LazyLock<Vec<CrateLicence>> = LazyLock::new(|| split_crates(CRATES));
    &SPLIT
}

/// `text` split into its licence texts, each opened by a [`RULE`] line.
/// Lines may end in `\n` or `\r\n` (a checkout that converts line endings).
fn split_crates(text: &'static str) -> Vec<CrateLicence> {
    // Where each rule line starts, and where the line after it starts.
    let mut rules = Vec::new();
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == RULE {
            rules.push((at, at + line.len()));
        }
        at += line.len();
    }
    let ends = rules
        .iter()
        .skip(1)
        .map(|&(rule, _)| rule)
        .chain([text.len()]);
    rules
        .iter()
        .zip(ends)
        .map(|(&(_, start), end)| {
            let entry = &text[start..end];
            let mut lines = entry.lines();
            let name = lines.next().unwrap_or_default();
            let used_by = lines.next().unwrap_or_default();
            let used_by = used_by.strip_prefix("Used by: ").unwrap_or(used_by);
            CrateLicence {
                name,
                used_by,
                entry,
            }
        })
        .collect()
}

/// The section: a heading, a hint, and one closed header per licence.
pub(super) fn show(ui: &mut egui::Ui, tk: &Tokens) {
    let heading = RichText::new(t("about.licences"))
        .strong()
        .size(15.5)
        .color(tk.box_text);
    ui.add(Label::new(heading).selectable(false));
    let hint = RichText::new(t("about.licences.hint")).color(tk.text_muted);
    ui.add(Label::new(hint).wrap());
    for (key, text) in BUNDLED {
        licence(ui, tk, &t(key), (key, 0), text);
    }
    let crates = crate_licences();
    let mut args = FluentArgs::new();
    args.set("count", crates.len());
    let header = RichText::new(t_args("about.licences.crates", &args)).color(tk.box_text);
    CollapsingHeader::new(header)
        .id_salt("about.licences.crates")
        .show(ui, |ui| {
            for (i, c) in crates.iter().enumerate() {
                licence(ui, tk, &crate_header(c), ("crate", i), c.entry);
            }
        });
}

/// The header of a crate licence: its name and the first few crates.
fn crate_header(c: &CrateLicence) -> String {
    const SHOWN: usize = 3;
    let crates: Vec<&str> = c.used_by.split(", ").collect();
    let mut names = crates[..crates.len().min(SHOWN)].join(", ");
    if crates.len() > SHOWN {
        names += ", …";
    }
    format!("{}: {names}", c.name)
}

/// A closed header that opens on `text`, in a box that scrolls.
fn licence(ui: &mut egui::Ui, tk: &Tokens, header: &str, id: (&str, usize), text: &str) {
    let id = ("about.licence", id);
    CollapsingHeader::new(RichText::new(header).color(tk.box_text))
        .id_salt(id)
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt(id)
                .max_height(320.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let body = RichText::new(text.trim()).size(12.0).color(tk.box_text);
                    ui.add(Label::new(body).wrap());
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_notices_cover_the_dependencies() {
        let crates = crate_licences();
        assert!(crates.len() > 20, "{} licence texts", crates.len());
        let used = |name: &str| {
            crates.iter().any(|c| {
                c.used_by
                    .split(", ")
                    .any(|u| u.split(' ').next() == Some(name))
            })
        };
        for name in ["egui", "eframe", "serde", "wgpu", "spellbook", "resvg"] {
            assert!(used(name), "{name} is listed");
        }
        // Only third-party crates, and each text names its licence.
        assert!(!used("reasoncraft_core") && !used("reasoncraft_spell"));
        for c in crates {
            assert!(!c.name.is_empty() && !c.used_by.is_empty());
            assert!(c.entry.len() > c.name.len() + c.used_by.len() + 50);
        }
        assert!(
            crates
                .iter()
                .any(|c| c.name == "Mozilla Public License 2.0")
        );
    }

    #[test]
    fn crate_notices_split_with_either_line_ending() {
        let lf = format!(
            "Intro\n\n{RULE}\nMIT License\nUsed by: a 1.0, b 2.0\n\nText one\n\n\
             {RULE}\nISC License\nUsed by: c 3.0\n\nText two\n"
        );
        let crlf = lf.replace('\n', "\r\n");
        for text in [lf, crlf] {
            let text: &'static str = Box::leak(text.into_boxed_str());
            let split = split_crates(text);
            assert_eq!(split.len(), 2, "{text:?}");
            let first = (split[0].name, split[0].used_by);
            assert_eq!(first, ("MIT License", "a 1.0, b 2.0"));
            assert_eq!((split[1].name, split[1].used_by), ("ISC License", "c 3.0"));
            assert!(split[0].entry.contains("Text one") && !split[0].entry.contains("ISC"));
            assert!(split[1].entry.trim_end().ends_with("Text two"));
        }
        // The real file with converted line endings gives every entry.
        let converted: &'static str = Box::leak(CRATES.replace('\n', "\r\n").into_boxed_str());
        assert_eq!(split_crates(converted).len(), crate_licences().len());
    }

    #[test]
    fn bundled_licences_are_the_full_texts() {
        let text = |key: &str| BUNDLED.iter().find(|(k, _)| *k == key).unwrap().1;
        assert!(text("about.licences.ubuntu").contains("UBUNTU FONT LICENCE Version 1.0"));
        let hack = text("about.licences.hack");
        assert!(hack.contains("Source Foundry") && hack.contains("BITSTREAM VERA LICENSE"));
        assert!(text("about.licences.dict_en").contains("SCOWL"));
        assert!(text("about.licences.dict_nl").contains("OpenTaal"));
        let spellbook = text("about.licences.spellbook");
        assert!(spellbook.contains("https://github.com/helix-editor/spellbook"));
        assert!(text("about.licences.wix").contains("Microsoft Reciprocal License"));
        assert!(text("about.licences.appimage").contains("libfuse"));
    }
}
