//! The find bar of a tab: what it looks for, the hit it is on, and the
//! hits found, cached until the document, query or options change.

use reasoncraft_core::{FindHit, FindOptions, find_all};

use super::DocTab;

/// One tab's find and replace state.
#[derive(Clone, Debug, Default)]
pub struct FindState {
    /// The find bar shows above the canvas.
    pub open: bool,
    /// The bar also shows the replacement field and its buttons.
    pub replace: bool,
    pub query: String,
    pub replacement: String,
    pub opts: FindOptions,
    /// The hit the bar is on (its box is selected).
    pub current: Option<FindHit>,
    /// The query field should take keyboard focus on its next frame.
    pub focus: bool,
    /// The hits last found, with the revision, query and options they were
    /// found for (see [`DocTab::find_hits`]).
    cache: Option<(u64, String, FindOptions, Vec<FindHit>)>,
}

impl DocTab {
    /// Every hit of the find bar's query, found again only when the
    /// document, the query or the options changed since the last call.
    pub fn find_hits(&mut self) -> &[FindHit] {
        let f = &mut self.find;
        let fresh = f
            .cache
            .as_ref()
            .is_some_and(|(rev, q, o, _)| *rev == self.revision && *q == f.query && *o == f.opts);
        if !fresh {
            let hits = find_all(&self.doc, &f.query, f.opts);
            f.cache = Some((self.revision, f.query.clone(), f.opts, hits));
        }
        f.cache.as_ref().map_or(&[], |(.., h)| h)
    }

    /// Where the current hit is among [`Self::find_hits`] (`None` when
    /// there is none, or it is gone since).
    pub fn find_position(&mut self) -> Option<usize> {
        let current = self.find.current.clone()?;
        self.find_hits().iter().position(|h| *h == current)
    }
}
