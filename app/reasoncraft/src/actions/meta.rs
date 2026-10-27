//! Document metadata: title, author and language, and the check hints the
//! author ignores.

use super::Action;
use crate::doc_tab::DocTab;

/// Carry out a metadata action. Typing in the title or author field shows
/// live and becomes one history step when the session ends; a language
/// that is not allowed, or already set, records nothing.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::SetDocMeta { title, author } => tab.edit_meta_live(&title, &author),
        Action::SetDocLanguage(lang) => {
            if tab.doc.meta.language != lang {
                tab.edit(|d| d.set_language(&lang));
            }
        }
        Action::IgnoreHint(issue) => {
            tab.edit(|d| d.ignore_hint(&issue));
        }
        Action::RestoreHints => {
            tab.edit(|d| d.restore_hints());
        }
        _ => unreachable!("not a metadata action"),
    }
}

#[cfg(test)]
mod tests;
