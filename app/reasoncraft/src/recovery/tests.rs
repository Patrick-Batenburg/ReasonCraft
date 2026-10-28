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

#[test]
fn restore_write_failure_keeps_the_orphan() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    write(&a, "Robots");
    drop(a);
    let mut b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let found = b.orphaned();
    let pkg = || Package::new(Document::new("Robots"));
    b.set_fail_writes(true);
    let key = Uuid::new_v4();
    b.write_then_discard(key, pkg(), meta("Robots"), &found[0]);
    b.wait_idle();
    assert!(found[0].package.is_file(), "the only copy is kept");
    let failures = b.failed_writes();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].tab_key, key);
    b.set_fail_writes(false);
    b.write_then_discard(key, pkg(), meta("Robots"), &found[0]);
    b.wait_idle();
    assert!(b.failed_writes().is_empty());
    assert!(!found[0].package.exists());
    assert!(!found[0].package.parent().unwrap().exists());
}

#[test]
fn a_new_session_leaves_no_staging_folder() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(names, [a.session_dir()]);
    assert!(a.session_dir().join("session.lock").is_file());
    assert!(!a.session_dir().join("session.lock.tmp").exists());
}

#[test]
fn folder_without_lock_file_is_not_offered() {
    let dir = tempfile::tempdir().unwrap();
    let lockless = dir.path().join(Uuid::new_v4().to_string());
    std::fs::create_dir(&lockless).unwrap();
    let key = Uuid::new_v4();
    let pkg = Package::new(Document::new("Robots"));
    reasoncraft_core::file::save_package(&pkg, &lockless.join(format!("{key}.rcraft"))).unwrap();
    let json = serde_json::to_vec(&meta("Robots")).unwrap();
    std::fs::write(lockless.join(format!("{key}.json")), json).unwrap();
    let b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    assert!(b.orphaned().is_empty());
    assert_eq!(entries(&lockless), 2, "left alone");
}

/// The start race: A's folder and lock exist but A has not locked it yet
/// (as when sessions were not made under a temporary name). B takes it for
/// a crashed, empty session and tidies it away; A's write must not bring
/// back a folder without a lock that a third app would offer.
#[test]
fn tidied_session_folder_is_not_made_again_by_a_write() {
    let dir = tempfile::tempdir().unwrap();
    let a_folder = dir.path().join(Uuid::new_v4().to_string());
    std::fs::create_dir(&a_folder).unwrap();
    std::fs::File::create(a_folder.join("session.lock")).unwrap();
    let b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    assert!(b.orphaned().is_empty());
    assert!(!a_folder.exists(), "B tidied the empty folder");
    let pkg = Package::new(Document::new("Robots"));
    let written = super::disk::write_entry(&a_folder, Uuid::new_v4(), &pkg, &meta("Robots"));
    assert!(written.is_err());
    assert!(!a_folder.exists());
    let c = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    assert!(c.orphaned().is_empty());
}

#[test]
fn missing_session_folder_error_is_translated() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = Package::new(Document::new("Robots"));
    let err =
        super::disk::write_entry(dir.path(), Uuid::new_v4(), &pkg, &meta("Robots")).unwrap_err();
    let text = err.to_string();
    assert!(!text.starts_with("recovery."), "{text}");
    assert_eq!(text, crate::i18n::t("recovery.session_gone"));
}

#[test]
fn failed_shutdown_delete_keeps_lock() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    write(&a, "Robots");
    let folder = a.session_dir();
    // A directory cannot be deleted as a file: that delete fails.
    std::fs::create_dir(folder.join("stuck.tmp")).unwrap();
    a.shutdown();
    assert!(folder.join("session.lock").is_file(), "the lock file stays");
    assert_eq!(entries(&folder), 2, "only the lock and the stuck entry");
    // The next start can take the lock and tidy the folder.
    std::fs::remove_dir(folder.join("stuck.tmp")).unwrap();
    let b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    assert!(b.orphaned().is_empty());
    assert!(!folder.exists());
}

#[test]
fn removed_copy_drops_its_kept_discard() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    write(&a, "Robots");
    drop(a);
    let mut b = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let found = b.orphaned();
    let key = Uuid::new_v4();
    let pkg = || Package::new(Document::new("Robots"));
    b.set_fail_writes(true);
    b.write_then_discard(key, pkg(), meta("Robots"), &found[0]);
    b.set_fail_writes(false);
    // The tab's copy goes (say, it was closed): a later write under the
    // same key does not delete the orphan.
    b.remove(key);
    b.write(key, pkg(), meta("Robots"));
    b.wait_idle();
    assert!(found[0].package.is_file());
}

#[test]
fn copies_are_written_through_unique_temp_files() {
    let dir = tempfile::tempdir().unwrap();
    let a = RecoveryStore::open(dir.path().to_path_buf()).unwrap();
    let key = Uuid::new_v4();
    // Files with the old fixed temp names are not ours: never touched.
    let theirs = a.session_dir().join(format!("{key}.json.tmp"));
    std::fs::write(&theirs, b"x").unwrap();
    a.write(key, Package::new(Document::new("R")), meta("R"));
    a.wait_idle();
    assert_eq!(std::fs::read(&theirs).unwrap(), b"x");
    assert!(a.session_dir().join(format!("{key}.json")).is_file());
    assert_eq!(entries(&a.session_dir()), 4, "lock, copy, meta and theirs");
}

#[cfg(unix)]
#[test]
fn recovery_folder_and_copies_are_private() {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    let a = RecoveryStore::open(root.clone()).unwrap();
    let key = write(&a, "Robots");
    assert_eq!(mode(&root), 0o700);
    assert_eq!(mode(&a.session_dir()), 0o700);
    for ext in ["rcraft", "json"] {
        assert_eq!(mode(&a.session_dir().join(format!("{key}.{ext}"))), 0o600);
    }
}
