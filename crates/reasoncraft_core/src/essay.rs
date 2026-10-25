//! The essay plan: text written for the parts of the map.

use serde::{Deserialize, Serialize};

use crate::ids::Item;

/// One place in the essay that holds text: the introduction, the
/// conclusion, or the paragraph for a box or card of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EssaySlot {
    Intro,
    Conclusion,
    Item(Item),
}
