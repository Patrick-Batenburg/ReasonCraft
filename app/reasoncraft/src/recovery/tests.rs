//! The recovery store on its own: sessions, orphans and tidying up.

use std::path::Path;

use chrono::Utc;
use reasoncraft_core::{Document, Package, Uuid};

use super::{RecoveryMeta, RecoveryStore};

fn meta(title: &str) -> RecoveryMeta {
    RecoveryMeta {
        original: None,
        original_mtime: None,
        title: title.to_owned(),
        saved_at: Utc::now(),
    }
}

fn write(store: &RecoveryStore, title: &str) -> Uuid {
    let key = Uuid::new_v4();
    store.write(key, Package::new(Document::new(title)), meta(title));
    store.wait_idle();
    key
}

fn entries(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |d| d.count())
}

#[test]
fn live_session_is_not_offered() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let key = write(&a, "Robots");
    let b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    assert!(b.orphaned().is_empty(), "A is still running");
    // A crashes: dropped without a shutdown.
    drop(a);
    let c = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let found = c.orphaned();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].tab_key, key);
    assert_eq!(found[0].meta.title, "Robots");
    assert!(found[0].package.is_file());
    let pkg = reasoncraft_core::file::open_package(&found[0].package).unwrap();
    assert_eq!(pkg.doc.boxes[&pkg.doc.root].text, "Robots");
}

#[test]
fn own_session_is_never_offered() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    write(&a, "Robots");
    assert!(a.orphaned().is_empty());
}

#[test]
fn shutdown_removes_own_session() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    write(&a, "Robots");
    a.shutdown();
    assert_eq!(entries(dir.path()), 0);
}

#[test]
fn remove_deletes_both_files() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let key = write(&a, "Robots");
    assert_eq!(entries(&a.session_dir()), 3);
    a.remove(key);
    a.wait_idle();
    assert_eq!(entries(&a.session_dir()), 1, "only the lock remains");
}

#[test]
fn discarding_the_last_entry_removes_the_orphan_folder() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    write(&a, "One");
    write(&a, "Two");
    let orphan_dir = a.session_dir();
    drop(a);
    let b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let found = b.orphaned();
    assert_eq!(found.len(), 2);
    b.discard(&found[0]);
    b.wait_idle();
    assert!(orphan_dir.is_dir(), "one entry left");
    b.discard(&found[1]);
    b.wait_idle();
    assert!(!orphan_dir.exists());
}

#[test]
fn empty_orphan_folders_are_tidied() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let orphan_dir = a.session_dir();
    drop(a);
    let b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    assert!(b.orphaned().is_empty());
    assert!(!orphan_dir.exists());
}

#[test]
fn default_dir_is_a_recovery_folder() {
    // Only computed, never created.
    if let Some(dir) = RecoveryStore::default_dir() {
        assert!(dir.ends_with("recovery"));
    }
}
