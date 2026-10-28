//! Text derived from the document and cached until it changes: the
//! outline and the check issues.

use std::time::Duration;

use reasoncraft_core::checks::{CheckSet, Issue, run_checks};
use reasoncraft_core::outline::{OutlineLine, OutlineOptions, build_outline};

use super::DocTab;

/// How long typing must pause before the status bar and the Check tab look
/// for issues again (see [`DocTab::settled_issues`]).
pub const ISSUE_SETTLE: Duration = Duration::from_millis(300);

impl DocTab {
    /// The document's outline for `opts`, rebuilt only when the document or
    /// the options changed since the last call.
    pub fn outline(&mut self, opts: OutlineOptions) -> &[OutlineLine] {
        let doc = &self.doc;
        self.outline_cache
            .get((self.revision, opts), || build_outline(doc, &opts))
    }

    /// The document's issues for the checks in `set`, found again only when
    /// the document or the set changed since the last call.
    pub fn issues(&mut self, set: &CheckSet) -> &[Issue] {
        let doc = &self.doc;
        self.issue_cache
            .get((self.revision, set.clone()), || run_checks(doc, set))
    }

    /// Like [`Self::issues`], for what is shown while the author may be
    /// typing: during an edit session the issues last found for `set` are
    /// kept until the document has not changed for [`ISSUE_SETTLE`], so
    /// the checks do not run on every keystroke. `now` is the UI's time in
    /// seconds. Also returns how long until the kept issues are due to be
    /// found again.
    pub fn settled_issues(&mut self, set: &CheckSet, now: f64) -> (&[Issue], Option<Duration>) {
        if self.seen_revision.0 != self.revision {
            self.seen_revision = (self.revision, now);
        }
        let same_set = self.issue_cache.key().is_some_and(|(_, s)| s == set);
        let stale = (self.issue_cache.key()).is_some_and(|(r, _)| *r != self.revision);
        let quiet = Duration::from_secs_f64((now - self.seen_revision.1).max(0.0));
        if self.session().is_some() && same_set && stale && quiet < ISSUE_SETTLE {
            let kept = self.issue_cache.last().map_or(&[][..], Vec::as_slice);
            return (kept, Some(ISSUE_SETTLE - quiet));
        }
        (self.issues(set), None)
    }
}
