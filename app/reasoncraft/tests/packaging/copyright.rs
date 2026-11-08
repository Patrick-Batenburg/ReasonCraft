//! One copyright line everywhere it is written (ruling S6-F4): the licence,
//! README, the Windows exe's version information and the macOS bundle.
//! The installers' publisher stays "ReasonCraft contributors".

use std::path::PathBuf;

const HOLDER: &str = "2026 Patrick van Batenburg and ReasonCraft contributors";

fn read(path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn one_copyright_line() {
    let line = format!("Copyright (c) {HOLDER}");
    for file in ["../../LICENSE-MIT", "../../README.md"] {
        assert!(read(file).contains(&line), "{file}");
    }
    // The exe's LegalCopyright, the only copyright build.rs writes.
    let build = read("build.rs");
    assert!(build.contains(&format!("\"{line}\"")), "build.rs: {line}");
    assert_eq!(build.matches("Copyright (c)").count(), 1, "build.rs");
    let plist = format!("<string>Copyright © {HOLDER}. MIT OR Apache-2.0.</string>");
    assert!(
        read("../../packaging/macos/Info.plist.in").contains(&plist),
        "Info.plist.in: {plist}"
    );
}
