//! A tab's names: the one it shows, the file name it suggests, and its
//! title with the unsaved and restored marks.

use super::DocTab;
use crate::i18n::t;

/// The longest file name (in characters, without extension) a Save or
/// Export suggests from a document title.
pub const FILE_STEM_CHARS: usize = 100;

impl DocTab {
    /// The file stem; without a file, the document's title (an imported
    /// map, an example), else "Untitled".
    pub fn name(&self) -> String {
        if let Some(stem) = self.path.as_deref().and_then(|p| p.file_stem()) {
            return stem.to_string_lossy().into_owned();
        }
        match self.doc.meta.title.trim() {
            "" => t("untitled"),
            title => title.to_owned(),
        }
    }

    /// [`Self::name`] as the file name a Save or Export suggests: without
    /// the characters file names cannot hold, at most [`FILE_STEM_CHARS`]
    /// long ("Untitled" when nothing is left).
    pub fn file_stem(&self) -> String {
        let name = self.name();
        let kept = name
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
                c if c.is_control() => ' ',
                c => c,
            })
            .collect::<String>();
        let words: Vec<&str> = kept.split_whitespace().collect();
        let stem: String = words.join(" ").chars().take(FILE_STEM_CHARS).collect();
        match stem.trim_end_matches(['.', ' ']) {
            "" => t("untitled"),
            stem => stem.to_owned(),
        }
    }

    /// [`Self::name`], marked when restored after a crash (until saved),
    /// followed by " •" when there are unsaved changes.
    pub fn title(&self) -> String {
        let mut name = self.name();
        if self.recovered_from.is_some() {
            let mut args = crate::i18n::FluentArgs::new();
            args.set("name", name);
            name = crate::i18n::t_args("recovery.tab_title", &args);
        }
        if self.is_dirty() {
            format!("{name} •")
        } else {
            name
        }
    }
}
