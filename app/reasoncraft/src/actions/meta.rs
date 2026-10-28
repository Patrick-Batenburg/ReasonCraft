//! Document metadata: title, author and language, and the check hints the
//! author ignores.

use crate::doc_tab::DocTab;

/// Set the document language; one already set records nothing (a
/// language that is not allowed is refused by the document).
pub(super) fn set_language(tab: &mut DocTab, lang: String) {
    if tab.doc.meta.language != lang {
        tab.edit(|d| d.set_language(&lang));
    }
}

#[cfg(test)]
mod tests;
