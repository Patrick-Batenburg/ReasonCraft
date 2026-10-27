use std::path::Path;

use super::job::{create_first_new_with, write_atomically};

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

#[test]
fn print_name_retries_on_permission_denied() {
    use std::io::{Error, ErrorKind};
    let dir = tempfile::tempdir().unwrap();
    let candidates = (0..3).map(|n| dir.path().join(format!("print-{n}.pdf")));
    // Windows refuses a name whose file is being deleted: the first name
    // is denied, the next one is free.
    let mut tried = Vec::new();
    let (path, _file) = create_first_new_with(candidates, |p| {
        tried.push(p.to_path_buf());
        match tried.len() {
            1 => Err(Error::from(ErrorKind::PermissionDenied)),
            _ => std::fs::File::create_new(p),
        }
    })
    .unwrap();
    assert_eq!(path, dir.path().join("print-1.pdf"));
    assert_eq!(tried.len(), 2);
    // A folder that refuses every name gives up instead of trying forever.
    let endless = (0u64..).map(|n| dir.path().join(format!("x-{n}")));
    let mut denied = 0;
    let err = create_first_new_with(endless, |_| {
        denied += 1;
        Err(Error::from(ErrorKind::PermissionDenied))
    })
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::PermissionDenied);
    assert_eq!(denied, 16, "sixteen names, then it gives up");
    // Other errors are not retried.
    let mut calls = 0;
    let other = (0..3).map(|n| dir.path().join(format!("y-{n}")));
    let err = create_first_new_with(other, |_| {
        calls += 1;
        Err(Error::other("disk"))
    })
    .unwrap_err();
    assert_eq!((calls, err.kind()), (1, ErrorKind::Other));
}
