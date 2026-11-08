//! The app's icons: the assets `cargo xtask icons` renders from the SVG
//! master are there, at their sizes, and not stale.

use std::path::{Path, PathBuf};

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
