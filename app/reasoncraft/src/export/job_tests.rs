use std::path::Path;

use super::job::write_atomically;

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn export_never_clobbers_an_existing_tmp_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map.docx");
    let theirs = dir.path().join("map.docx.tmp");
    std::fs::write(&theirs, b"the user's file").unwrap();
    std::fs::write(&path, b"old").unwrap();
    write_atomically(&path, b"new").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
    assert_eq!(std::fs::read(&theirs).unwrap(), b"the user's file");
    assert_eq!(names(dir.path()), ["map.docx", "map.docx.tmp"]);
}

#[test]
fn concurrent_exports_do_not_share_a_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.docx");
    let handles: Vec<_> = (0..8u8)
        .map(|i| {
            let path = path.clone();
            std::thread::spawn(move || write_atomically(&path, &vec![i; 50_000]).is_ok())
        })
        .collect();
    let ok: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    // Windows may refuse a rename over a file another export is replacing;
    // what matters is that a result is never a mix and no temp is left.
    assert!(ok.iter().any(|ok| *ok));
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 50_000);
    assert!(bytes.iter().all(|b| *b == bytes[0]), "no mixed content");
    assert_eq!(names(dir.path()), ["out.docx"]);
}

#[test]
fn failed_export_removes_its_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    // A directory in the way makes the final rename fail.
    let path = dir.path().join("taken.docx");
    std::fs::create_dir(&path).unwrap();
    assert!(write_atomically(&path, b"x").is_err());
    assert_eq!(names(dir.path()), ["taken.docx"]);
}
