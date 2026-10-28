//! The documents crashed sessions left behind, found on start. Their
//! sessions' locks stay held while the recovery prompt is open, so an app
//! started meanwhile does not offer (and restore) the same documents too.

use std::fs::{self, File};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use super::RecoveredDoc;
use super::disk::{entries, remove_if_empty, take_lock};
use super::store::{Op, RecoveryStore};

/// The documents of crashed sessions, oldest first, with those sessions'
/// locks (released by [`RecoveryStore::release`], or when the last copy
/// of this value goes).
#[derive(Clone, Debug, Default)]
pub struct Orphans {
    docs: Vec<RecoveredDoc>,
    locks: Arc<Vec<(PathBuf, File)>>,
}

/// Two offers are the same when they offer the same documents.
impl PartialEq for Orphans {
    fn eq(&self, other: &Self) -> bool {
        self.docs == other.docs
    }
}

impl Deref for Orphans {
    type Target = [RecoveredDoc];
    fn deref(&self) -> &[RecoveredDoc] {
        &self.docs
    }
}

impl<'a> IntoIterator for &'a Orphans {
    type Item = &'a RecoveredDoc;
    type IntoIter = std::slice::Iter<'a, RecoveredDoc>;
    fn into_iter(self) -> Self::IntoIter {
        self.docs.iter()
    }
}

impl RecoveryStore {
    /// The autosaved documents of sessions that ended without a normal exit
    /// (their lock is free), oldest first, keeping those sessions locked.
    /// Empty folders of such sessions are removed.
    pub fn orphaned(&self) -> Orphans {
        let Ok(sessions) = fs::read_dir(&self.dir) else {
            return Orphans::default();
        };
        let mut docs = Vec::new();
        let mut locks = Vec::new();
        for entry in sessions.flatten() {
            let folder = entry.path();
            let session = entry.file_name().to_str().and_then(|n| n.parse().ok());
            let Some(session) = session.filter(|s| *s != self.session) else {
                continue;
            };
            // Held while the folder is read, and while an empty one goes.
            let Some(lock) = take_lock(&folder) else {
                continue;
            };
            let before = docs.len();
            docs.extend(entries(&folder, session));
            if docs.len() == before {
                remove_if_empty(&folder, lock);
            } else {
                locks.push((folder, lock));
            }
        }
        docs.sort_by_key(|d: &RecoveredDoc| d.meta.saved_at);
        Orphans {
            docs,
            locks: Arc::new(locks),
        }
    }

    /// The recovery prompt was answered: once the removals sent before are
    /// done, release the sessions' locks, removing the folders left empty.
    pub fn release(&self, orphans: Orphans) {
        self.send(Op::Release(orphans.locks));
    }
}

/// Release `locks` (once no other copy holds them), removing each folder
/// left empty while its lock is still held.
pub(super) fn release(locks: Arc<Vec<(PathBuf, File)>>) {
    if let Ok(locks) = Arc::try_unwrap(locks) {
        for (folder, lock) in locks {
            remove_if_empty(&folder, lock);
        }
    }
}
