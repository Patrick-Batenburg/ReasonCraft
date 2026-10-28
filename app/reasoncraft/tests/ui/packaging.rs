//! App identity: the icon assets rendered from the SVG master, no trace of
//! the old cargo-packager setup, a document path handed over by the OS (as
//! a double-click does), and `--version`. The installers' own checks are in
//! `packaging_windows`, `packaging_linux` and `packaging_macos`.

use std::path::{Path, PathBuf};

use reasoncraft::ReasonCraftApp;
use resvg::{tiny_skia, usvg};

/// The PNG sizes `cargo run -p xtask -- icons` writes.
const PNG_SIZES: [u32; 9] = [16, 24, 32, 48, 64, 128, 256, 512, 1024];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn assets() -> PathBuf {
    crate_dir().join("assets")
}

fn decode(path: &Path) -> image::RgbaImage {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    image::load_from_memory(&bytes)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .to_rgba8()
}

/// The SVG master rendered at `size` px, as the generator renders it.
fn render_master(size: u32) -> image::RgbaImage {
    let svg = std::fs::read(assets().join("icon.svg")).expect("the SVG master exists");
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default()).expect("a valid SVG");
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a non-empty size");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    image::load_from_memory(&pixmap.encode_png().expect("encodes"))
        .expect("decodes")
        .to_rgba8()
}

/// The sizes of the images in an `.ico` file (its directory entries; a
/// width byte of 0 means 256).
fn ico_sizes(bytes: &[u8]) -> Vec<u32> {
    assert_eq!(&bytes[..4], &[0, 0, 1, 0], "an icon file header");
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..count)
        .map(|i| {
            let entry = &bytes[6 + 16 * i..6 + 16 * (i + 1)];
            let (w, h) = (entry[0], entry[1]);
            assert_eq!(w, h, "square icons");
            if w == 0 { 256 } else { u32::from(w) }
        })
        .collect()
}

#[test]
fn icon_assets_present_and_sized() {
    for size in PNG_SIZES {
        let png = decode(&assets().join(format!("icons/icon-{size}.png")));
        assert_eq!(png.dimensions(), (size, size), "icon-{size}.png");
    }

    let ico = std::fs::read(assets().join("icon.ico")).expect("icon.ico exists");
    let mut sizes = ico_sizes(&ico);
    sizes.sort_unstable();
    assert_eq!(sizes, [16, 32, 48, 256]);

    let icns = std::fs::read(assets().join("icon.icns")).expect("icon.icns exists");
    assert_eq!(&icns[..4], b"icns");
    let len = u32::from_be_bytes([icns[4], icns[5], icns[6], icns[7]]) as usize;
    assert_eq!(len, icns.len(), "the icns length field covers the file");

    // The window icon is the 256 px render of the master, not a stale file.
    let window_icon = decode(&assets().join("icon.png"));
    let rendered = render_master(256);
    assert_eq!(window_icon.dimensions(), rendered.dimensions());
    let worst = window_icon
        .pixels()
        .zip(rendered.pixels())
        .flat_map(|(a, b)| a.0.iter().zip(b.0).map(|(x, y)| x.abs_diff(y)))
        .max()
        .unwrap_or(0);
    assert!(
        worst <= 2,
        "icon.png differs from the SVG render by {worst}"
    );
}

#[test]
fn no_cargo_packager_left() {
    // The installers come from `packaging/` (WiX, nfpm, appimagetool, the
    // macOS scripts); nothing is left of cargo-packager and its NSIS
    // template.
    let manifest = std::fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    assert!(
        !manifest.contains("metadata.packager"),
        "the app's Cargo.toml still configures cargo-packager"
    );
    assert!(
        !crate_dir().join("packaging").exists(),
        "app/reasoncraft/packaging is gone (see packaging/ at the root)"
    );
}

#[test]
fn cli_path_opens_file() {
    // The association passes the path quoted, as one argument; it may hold
    // spaces and non-ASCII letters.
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Mijn kaarten");
    std::fs::create_dir(&folder).unwrap();
    let path = folder.join("Één kaart.rcraft");
    let doc = reasoncraft_core::Document::new("Robots will take our jobs");
    reasoncraft_core::file::save(&doc, &path).unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    reasoncraft::file_ops::open_paths(&mut app, [path.clone().into_os_string()]);
    assert!(app.modal.is_none());
    let tab = app.tabs.iter().find(|t| t.path.as_deref() == Some(&path));
    let tab = tab.expect("the file opens in a tab");
    assert_eq!(
        tab.doc.boxes[&tab.doc.root].text,
        "Robots will take our jobs"
    );
}

#[test]
fn version_flag_prints_version() {
    use std::io::Read as _;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    // Should `--version` ever open the window, it does so on a throw-away
    // profile and is stopped, never touching the real settings.
    let profile = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_reasoncraft"))
        .arg("--version")
        .env(reasoncraft::profile::PROFILE_DIR_VAR, profile.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the app starts");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(60) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("`reasoncraft --version` did not exit (it opened the window?)");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    assert!(status.success(), "{status:?}");
    assert_eq!(
        stdout,
        format!("ReasonCraft {}\n", env!("CARGO_PKG_VERSION"))
    );
}
