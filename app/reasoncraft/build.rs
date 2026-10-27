//! Generates the index of the help pages: every `help/<lang>/<topic>.md`,
//! embedded with `include_str!` (see `src/help/index.rs`).

use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

fn main() {
    let help = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("set by cargo")).join("help");
    println!("cargo:rerun-if-changed={}", help.display());
    let mut files = Vec::new();
    for lang in fs::read_dir(&help).expect("the help folder exists") {
        let lang = lang.expect("a readable help folder").path();
        if !lang.is_dir() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", lang.display());
        let code = name(&lang);
        for page in fs::read_dir(&lang).expect("a readable language folder") {
            let page = page.expect("a readable help page").path();
            if page.extension().is_some_and(|e| e == "md") {
                let stem = page.file_stem().expect("a file name");
                files.push((code.clone(), stem.to_string_lossy().into_owned(), page));
            }
        }
    }
    files.sort();
    let mut out = String::from("&[\n");
    for (lang, topic, path) in files {
        let path = path.to_str().expect("a UTF-8 path");
        writeln!(out, "    ({lang:?}, {topic:?}, include_str!({path:?})),").expect("in memory");
    }
    out.push(']');
    let dest = Path::new(&env::var("OUT_DIR").expect("set by cargo")).join("help_index.rs");
    fs::write(dest, out).expect("OUT_DIR is writable");
}

fn name(path: &Path) -> String {
    path.file_name()
        .expect("a folder name")
        .to_string_lossy()
        .into_owned()
}
