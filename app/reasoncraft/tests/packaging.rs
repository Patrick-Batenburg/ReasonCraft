//! App identity and installers: the icon assets rendered from the SVG
//! master, the packager configuration with the `.rcraft` association, and
//! a document path handed over by the OS (as a double-click does).

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
fn packager_config_has_association() {
    let manifest = std::fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    let manifest: toml::Table = manifest.parse().expect("valid TOML");
    let packager = &manifest["package"]["metadata"]["packager"];
    let text = |v: &toml::Value| v.as_str().expect("a string").to_owned();
    let list = |v: &toml::Value| -> Vec<String> {
        v.as_array().expect("a list").iter().map(text).collect()
    };

    assert_eq!(text(&packager["product-name"]), "ReasonCraft");
    assert_eq!(text(&packager["identifier"]), "nl.reasoncraft.app");
    assert_eq!(text(&packager["publisher"]), "ReasonCraft contributors");
    let licence = crate_dir().join(text(&packager["license-file"]));
    assert!(licence.is_file(), "{}", licence.display());

    let formats = list(&packager["formats"]);
    for format in ["nsis", "app", "dmg", "deb", "appimage"] {
        assert!(formats.iter().any(|f| f == format), "format {format}");
    }
    let icons = list(&packager["icons"]);
    for ext in [".png", ".ico", ".icns"] {
        assert!(icons.iter().any(|i| i.ends_with(ext)), "an {ext} icon");
    }

    let nsis = &packager["nsis"];
    assert_eq!(
        text(&nsis["installer-mode"]),
        "currentUser",
        "per user, no admin"
    );
    if let Some(template) = nsis.get("template") {
        assert!(crate_dir().join(text(template)).is_file());
    }

    let associations = packager["file-associations"].as_array().expect("a list");
    assert_eq!(associations.len(), 1);
    let rcraft = &associations[0];
    assert_eq!(list(&rcraft["extensions"]), ["rcraft"]);
    assert_eq!(text(&rcraft["name"]), "ReasonCraft map");
    assert_eq!(text(&rcraft["mime-type"]), "application/x-reasoncraft");
    assert_eq!(text(&rcraft["role"]), "editor", "CFBundleTypeRole Editor");

    // Linux learns the MIME type from the shared-mime-info file.
    let mime = std::fs::read_to_string(crate_dir().join("packaging/linux/reasoncraft-mime.xml"))
        .expect("the MIME description exists");
    assert!(mime.contains(r#"type="application/x-reasoncraft""#));
    assert!(mime.contains(r#"pattern="*.rcraft""#));
    let desktop = std::fs::read_to_string(crate_dir().join("packaging/linux/reasoncraft.desktop"))
        .expect("the desktop entry exists");
    assert!(desktop.contains("MimeType=application/x-reasoncraft;"));
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
