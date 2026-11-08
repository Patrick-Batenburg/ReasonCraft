//! Saving: the temporary file, failures, concurrent saves, permissions.

use std::io::Write;
use std::path::Path;

use super::*;
use crate::model::Document;

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn save_never_touches_a_users_tmp_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map.rcraft");
    let theirs = dir.path().join("map.rcraft.tmp");
    std::fs::write(&theirs, b"the user's file").unwrap();
    save(&Document::new("original"), &path).unwrap();
    // A directory with a temp-like name does not block a save either.
    std::fs::create_dir(dir.path().join("map.rcraft.0.tmp")).unwrap();
    let changed = Document::new("changed");
    save(&changed, &path).unwrap();
    assert_eq!(open(&path).unwrap(), changed);
    assert_eq!(std::fs::read(&theirs).unwrap(), b"the user's file");
    assert_eq!(
        names(dir.path()),
        ["map.rcraft", "map.rcraft.0.tmp", "map.rcraft.tmp"]
    );
}

#[test]
fn failed_save_keeps_the_file_and_leaves_no_temp() {
    let dir = tempfile::tempdir().unwrap();
    // A directory in the way makes the final rename fail.
    let path = dir.path().join("taken.rcraft");
    std::fs::create_dir(&path).unwrap();
    assert!(save(&Document::new("c"), &path).is_err());
    assert_eq!(names(dir.path()), ["taken.rcraft"]);
    // A write that fails half way leaves the old file whole.
    let path = dir.path().join("map.rcraft");
    let original = Document::new("original");
    save(&original, &path).unwrap();
    let failed: Result<(), std::io::Error> = atomic_write(&path, false, |f| {
        f.write_all(b"half")?;
        Err(std::io::Error::other("disk full"))
    });
    assert!(failed.is_err());
    assert_eq!(open(&path).unwrap(), original);
    assert_eq!(names(dir.path()), ["map.rcraft", "taken.rcraft"]);
}

#[test]
fn concurrent_saves_never_mix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map.rcraft");
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let path = path.clone();
            std::thread::spawn(move || save(&Document::new(&format!("doc {i}")), &path).is_ok())
        })
        .collect();
    let ok: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    // Windows may refuse a rename over a file another save is replacing;
    // what matters is that the file is whole and no temp is left.
    assert!(ok.iter().any(|ok| *ok));
    let d = open(&path).unwrap();
    assert!(d.boxes[&d.root].text.starts_with("doc "));
    assert_eq!(names(dir.path()), ["map.rcraft"]);
}

#[cfg(unix)]
#[test]
fn save_keeps_the_files_permissions_and_private_saves_are_private() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let path = dir.path().join("map.rcraft");
    save(&Document::new("c"), &path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    save(&Document::new("d"), &path).unwrap();
    assert_eq!(mode(&path), 0o600);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    save(&Document::new("e"), &path).unwrap();
    assert_eq!(mode(&path), 0o640);
    let private = dir.path().join("copy.rcraft");
    save_package_private(&crate::Package::new(Document::new("p")), &private).unwrap();
    assert_eq!(mode(&private), 0o600);
}

/// Opening keeps at most `MAX_DOCUMENT_CHARS` of text, while editing has no
/// such limit: a map past it is not saved, so no file this app writes is
/// cut when it is opened again (ruling RR-1). The case from the review:
/// 161 snippets of 50,000 characters, written by editing.
#[test]
fn a_map_too_long_to_open_whole_is_not_saved() {
    use crate::caps::MAX_DOCUMENT_CHARS;
    use crate::essay::EssaySlot;
    let mut d = Document::new("Claim");
    let big = "word ".repeat(10_000);
    for _ in 0..161 {
        d.add_snippet(&big);
    }
    d.set_essay_text(EssaySlot::Intro, "My introduction, written by hand.")
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map.rcraft");
    match save(&d, &path) {
        Err(FileError::TooMuchText { chars, max }) => {
            assert_eq!((chars, max), (d.total_text_chars(), MAX_DOCUMENT_CHARS));
            assert!(chars > max);
        }
        other => panic!("saved: {other:?}"),
    }
    assert!(names(dir.path()).is_empty(), "nothing written");

    // At the limit exactly, the file is saved and opens whole.
    d.snippets.truncate(159);
    let intro =
        MAX_DOCUMENT_CHARS - (d.total_text_chars() - "My introduction, written by hand.".len());
    d.set_essay_text(EssaySlot::Intro, &"i".repeat(intro))
        .unwrap();
    assert_eq!(d.total_text_chars(), MAX_DOCUMENT_CHARS);
    save(&d, &path).unwrap();
    let back = open_package(&path).unwrap();
    assert!(!back.cut_on_open);
    assert_eq!(back.doc, d);
}

/// A save to a symbolic link updates the file it points to and keeps the
/// link. Windows needs a privilege to make links: skipped there
/// without it.
#[test]
fn saving_through_a_link_keeps_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.rcraft");
    save(&Document::new("old"), &real).unwrap();
    let link = dir.path().join("link.rcraft");
    #[cfg(unix)]
    std::os::unix::fs::symlink("real.rcraft", &link).unwrap();
    #[cfg(windows)]
    if let Err(e) = std::os::windows::fs::symlink_file("real.rcraft", &link) {
        eprintln!("skipped: no symbolic link ({e})");
        return;
    }
    let new = Document::new("new");
    save(&new, &link).unwrap();
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(open(&real).unwrap(), new);
    assert_eq!(names(dir.path()), ["link.rcraft", "real.rcraft"]);
}

/// A read-only file is not replaced, on any OS (on Unix a rename
/// would replace it, and keep it read-only); the save fails as "permission
/// denied" and leaves the file as it was.
#[test]
fn a_read_only_file_is_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ro.rcraft");
    save(&Document::new("Kept"), &path).unwrap();
    let set_read_only = |on: bool| {
        let mut perm = std::fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(on);
        std::fs::set_permissions(&path, perm).unwrap();
    };
    set_read_only(true);
    let result = save(&Document::new("Replaced"), &path);
    let kept = open(&path).map(|d| d.boxes[&d.root].text.clone());
    set_read_only(false);
    match result {
        Err(FileError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
        other => panic!("{other:?}"),
    }
    assert_eq!(kept.unwrap(), "Kept");
    let left = std::fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(left, 1, "no temporary file left behind");
}
