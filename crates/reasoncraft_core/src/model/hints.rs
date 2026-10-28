//! Hints the author chose to ignore, kept in the document.

use serde::{Deserialize, Serialize};

use crate::checks::Issue;
use crate::edit::EditError;
use crate::ids::Item;
use crate::model::Document;

/// One ignored check hint: the check, the item and the terms it listed.
/// The kind is kept as its saved name, so a hint of a check that only a
/// newer build knows survives a roundtrip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoredHint {
    pub kind: String,
    pub item: Item,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<String>,
}

impl IgnoredHint {
    /// Whether `issue` is this hint: same check and item, and the same set
    /// of terms in any order.
    pub fn matches(&self, issue: &Issue) -> bool {
        self.kind == issue.kind.key()
            && self.item == issue.item
            && self.terms.len() == issue.terms.len()
            && self.terms.iter().all(|t| issue.terms.contains(t))
    }
}

impl From<&Issue> for IgnoredHint {
    fn from(issue: &Issue) -> Self {
        Self {
            kind: issue.kind.key().to_owned(),
            item: issue.item,
            terms: issue.terms.clone(),
        }
    }
}

impl Document {
    /// Stop reporting `issue`. Refused with [`EditError::NotAllowed`] when it
    /// is already ignored, so it records no history.
    pub fn ignore_hint(&mut self, issue: &Issue) -> Result<(), EditError> {
        if self.ignored_hints.iter().any(|h| h.matches(issue)) {
            return Err(EditError::NotAllowed);
        }
        self.ignored_hints.push(IgnoredHint::from(issue));
        self.finish_edit();
        Ok(())
    }

    /// Report every ignored hint again. Refused with
    /// [`EditError::NotAllowed`] when none is ignored.
    pub fn restore_hints(&mut self) -> Result<(), EditError> {
        if self.ignored_hints.is_empty() {
            return Err(EditError::NotAllowed);
        }
        self.ignored_hints.clear();
        self.finish_edit();
        Ok(())
    }
}
