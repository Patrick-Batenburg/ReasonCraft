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
