//! Snippets: pieces of source text kept beside the map.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::edit::EditError;
use crate::model::{Document, SourceType};

/// The longest snippet text kept, in characters.
pub const MAX_SNIPPET_CHARS: usize = 50_000;

/// Where a snippet came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetSource {
    pub source_type: SourceType,
    pub citation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// A piece of text, optionally tagged with its source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Uuid,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SnippetSource>,
}

/// `text` cut to at most [`MAX_SNIPPET_CHARS`] characters.
fn capped(text: &str) -> String {
    match text.char_indices().nth(MAX_SNIPPET_CHARS) {
        Some((end, _)) => text[..end].to_owned(),
        None => text.to_owned(),
    }
}

impl Document {
    fn finish_snippet_edit(&mut self) {
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
    }

    fn snippet_index(&self, id: Uuid) -> Result<usize, EditError> {
        self.snippets
            .iter()
            .position(|s| s.id == id)
            .ok_or(EditError::NotFound)
    }

    /// Append a snippet; the text is capped at [`MAX_SNIPPET_CHARS`].
    pub fn add_snippet(&mut self, text: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.snippets.push(Snippet {
            id,
            text: capped(text),
            source: None,
        });
        self.finish_snippet_edit();
        id
    }

    /// Replace a snippet's text; the text is capped at [`MAX_SNIPPET_CHARS`].
    pub fn update_snippet_text(&mut self, id: Uuid, text: &str) -> Result<(), EditError> {
        let i = self.snippet_index(id)?;
        self.snippets[i].text = capped(text);
        self.finish_snippet_edit();
        Ok(())
    }

    /// Set or clear a snippet's source. The link is trimmed; empty means none.
    pub fn set_snippet_source(
        &mut self,
        id: Uuid,
        source: Option<SnippetSource>,
    ) -> Result<(), EditError> {
        let i = self.snippet_index(id)?;
        self.snippets[i].source = source.map(|mut s| {
            s.link = s
                .link
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty());
            s
        });
        self.finish_snippet_edit();
        Ok(())
    }

    /// Delete a snippet.
    pub fn remove_snippet(&mut self, id: Uuid) -> Result<(), EditError> {
        let i = self.snippet_index(id)?;
        self.snippets.remove(i);
        self.finish_snippet_edit();
        Ok(())
    }

    /// Move a snippet to `index`, clamped to the end of the list.
    pub fn move_snippet(&mut self, id: Uuid, index: usize) -> Result<(), EditError> {
        let i = self.snippet_index(id)?;
        let s = self.snippets.remove(i);
        let to = index.min(self.snippets.len());
        self.snippets.insert(to, s);
        self.finish_snippet_edit();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(link: Option<&str>) -> SnippetSource {
        SnippetSource {
            source_type: SourceType::Publication,
            citation: "Doe 2020".into(),
            link: link.map(Into::into),
        }
    }

    #[test]
    fn snippet_crud_and_order() {
        let mut d = Document::new("C");
        let a = d.add_snippet("a");
        let b = d.add_snippet("b");
        let c = d.add_snippet("c");
        let order = |d: &Document| {
            d.snippets
                .iter()
                .map(|s| s.text.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(order(&d), ["a", "b", "c"]);

        d.update_snippet_text(b, "B").unwrap();
        d.set_snippet_source(a, Some(src(Some("  https://x.org  "))))
            .unwrap();
        assert_eq!(
            d.snippets[0].source.as_ref().unwrap().link.as_deref(),
            Some("https://x.org")
        );
        d.set_snippet_source(a, Some(src(Some("  ")))).unwrap();
        assert_eq!(d.snippets[0].source.as_ref().unwrap().link, None);
        d.set_snippet_source(a, None).unwrap();
        assert_eq!(d.snippets[0].source, None);

        d.move_snippet(c, 0).unwrap();
        assert_eq!(order(&d), ["c", "a", "B"]);
        d.move_snippet(c, 99).unwrap();
        assert_eq!(order(&d), ["a", "B", "c"]);

        d.remove_snippet(a).unwrap();
        assert_eq!(order(&d), ["B", "c"]);

        let ghost = Uuid::new_v4();
        assert_eq!(d.update_snippet_text(ghost, "x"), Err(EditError::NotFound));
        assert_eq!(d.set_snippet_source(ghost, None), Err(EditError::NotFound));
        assert_eq!(d.remove_snippet(ghost), Err(EditError::NotFound));
        assert_eq!(d.move_snippet(ghost, 0), Err(EditError::NotFound));
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn snippet_text_capped_at_char_boundary() {
        let long: String = "é日a".chars().cycle().take(60_000).collect();
        let mut d = Document::new("C");
        let id = d.add_snippet(&long);
        assert_eq!(d.snippets[0].text.chars().count(), MAX_SNIPPET_CHARS);
        assert!(long.starts_with(&d.snippets[0].text));
        d.update_snippet_text(id, "short").unwrap();
        d.update_snippet_text(id, &long).unwrap();
        assert_eq!(d.snippets[0].text.chars().count(), MAX_SNIPPET_CHARS);
        let exact = "é".repeat(MAX_SNIPPET_CHARS);
        d.update_snippet_text(id, &exact).unwrap();
        assert_eq!(d.snippets[0].text, exact);
    }

    #[test]
    fn step2_document_without_snippets_unchanged_on_roundtrip() {
        let d = Document::new("C");
        let json = serde_json::to_value(&d).unwrap();
        assert!(json.get("snippets").is_none());
        let back: Document = serde_json::from_value(json.clone()).unwrap();
        assert!(back.snippets.is_empty());
        assert_eq!(serde_json::to_value(&back).unwrap(), json);
    }

    #[test]
    fn snippets_are_a_named_field_not_extra() {
        let mut d = Document::new("C");
        let id = d.add_snippet("t");
        d.set_snippet_source(id, Some(src(None))).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back.snippets, d.snippets);
        assert!(!back.extra.contains_key("snippets"));
    }

    #[test]
    fn duplicate_snippet_ids_invalid() {
        let mut d = Document::new("C");
        d.add_snippet("a");
        let dup = d.snippets[0].clone();
        d.snippets.push(dup);
        let err = d.validate().unwrap_err();
        assert!(err.contains("duplicate snippet id"), "{err}");
    }
}
