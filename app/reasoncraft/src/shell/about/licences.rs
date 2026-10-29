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
    ("about.licences.appimage", APPIMAGE_RUNTIME),
];

/// The AppImage type2-runtime's licence, then the full text of each library
/// it links statically, as upstream has it at the version runtime release
/// 20251108 was built with (THIRD-PARTY.md, "Linux AppImage"). The files in
/// `assets/licences/appimage-runtime/` are byte for byte upstream's.
const APPIMAGE_RUNTIME: &str = concat!(
    "AppImage type2-runtime 20251108 (MIT)\n\n",
    include_str!("../../../assets/licences/appimage-runtime/type2-runtime-LICENSE.txt"),
    "\nThe runtime also links mimalloc (MIT), which the notice above leaves out. \
     The full texts follow, at the versions the runtime was built with.\n",
    "\n\n---- musl 1.2.5 (MIT) ----\n\n",
    include_str!("../../../assets/licences/appimage-runtime/musl-COPYRIGHT.txt"),
    "\n\n---- libfuse 3.15.0, patched (LGPL-2.1) ----\n\n",
    "The runtime's source at its tag 20251108 (https://github.com/AppImage/type2-runtime) \
     builds libfuse 3.15.0 with its patch and can relink the runtime with a changed libfuse.\n\n",
    include_str!("../../../assets/licences/appimage-runtime/libfuse-LICENSE.txt"),
    "\n",
    include_str!("../../../assets/licences/appimage-runtime/libfuse-LGPL2.txt"),
    "\n\n---- squashfuse 0.5.2 (BSD-2-Clause) ----\n\n",
    include_str!("../../../assets/licences/appimage-runtime/squashfuse-LICENSE.txt"),
    "\n\n---- zstd 1.5.6 (BSD-3-Clause; zstd is BSD-3-Clause OR GPL-2.0) ----\n\n",
    include_str!("../../../assets/licences/appimage-runtime/zstd-LICENSE.txt"),
    "\n\n---- zlib 1.3.1 (Zlib) ----\n\n",
    include_str!("../../../assets/licences/appimage-runtime/zlib-LICENSE.txt"),
    "\n\n---- mimalloc 2.1.7 (MIT) ----\n\n",
    include_str!("../../../assets/licences/appimage-runtime/mimalloc-LICENSE.txt"),
);

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
        // The runtime's own licence and the full text of each library it links
        // statically (THIRD-PARTY.md, "Linux AppImage").
        let appimage = text("about.licences.appimage");
        for (part, marker) in [
            ("type2-runtime", "Copyright (c) 2004-23 probonopd"),
            ("musl", "Copyright © 2005-2020 Rich Felker, et al."),
            (
                "libfuse",
                "GNU LESSER GENERAL PUBLIC LICENSE\n                       Version 2.1",
            ),
            ("libfuse's scope", "- All files in the lib/ directory."),
            ("squashfuse", "Copyright (c) 2012 Dave Vasilevsky"),
            ("zstd", "For Zstandard software"),
            ("zlib", "(C) 1995-2022 Jean-loup Gailly and Mark Adler"),
            (
                "mimalloc",
                "Copyright (c) 2018-2021 Microsoft Corporation, Daan Leijen",
            ),
        ] {
            assert!(appimage.contains(marker), "the {part} licence text");
        }
    }
}
