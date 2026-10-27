//! Developer tasks for the ReasonCraft workspace. Never shipped.
//!
//! `cargo run -p xtask -- icons` renders `app/reasoncraft/assets/icon.svg`
//! to the PNG sizes, `icon.png` (the window icon), `icon.ico` (Windows)
//! and `icon.icns` (macOS). The generated files are committed.
//!
//! `cargo run -p xtask -- sample-map <path>` saves the bundled reasoning
//! example as a `.rcraft` file, for installer smoke tests.
//!
//! `cargo run -p xtask -- licence` writes the installer's licence page,
//! `app/reasoncraft/packaging/LICENSE.txt`: both licence texts, as
//! ReasonCraft is under either at the user's option.
//!
//! `cargo run -p xtask -- notices` writes the Rust crates' licence texts for
//! the About page (see [`notices`]).

use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use icns::{IconFamily, IconType};
use resvg::{tiny_skia, usvg};

mod notices;

/// The PNG sizes written to `assets/icons/icon-<size>.png`.
const PNG_SIZES: [u32; 9] = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
/// The sizes inside `icon.ico`.
const ICO_SIZES: [u32; 4] = [16, 32, 48, 256];
/// The sizes inside `icon.icns`, with their icon types.
const ICNS_TYPES: [(u32, IconType); 10] = [
    (16, IconType::RGBA32_16x16),
    (32, IconType::RGBA32_16x16_2x),
    (32, IconType::RGBA32_32x32),
    (64, IconType::RGBA32_32x32_2x),
    (128, IconType::RGBA32_128x128),
    (256, IconType::RGBA32_128x128_2x),
    (256, IconType::RGBA32_256x256),
    (512, IconType::RGBA32_256x256_2x),
    (512, IconType::RGBA32_512x512),
    (1024, IconType::RGBA32_512x512_2x),
];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let outcome = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["icons"] => icons(),
        ["sample-map", path] => sample_map(Path::new(path)),
        ["licence"] => licence(),
        ["notices"] => notices::notices(),
        _ => {
            eprintln!("usage: cargo run -p xtask -- icons | sample-map <path> | licence | notices");
            return ExitCode::FAILURE;
        }
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_dir() -> &'static Path {
    let xtask = Path::new(env!("CARGO_MANIFEST_DIR"));
    xtask.parent().expect("xtask sits in the workspace")
}

fn assets_dir() -> PathBuf {
    workspace_dir().join("app/reasoncraft/assets")
}

fn licence() -> Result<()> {
    let root = workspace_dir();
    let mit = fs::read_to_string(root.join("LICENSE-MIT"))?;
    let apache = fs::read_to_string(root.join("LICENSE-APACHE"))?;
    let text = [
        "ReasonCraft
Copyright (c) 2026 Patrick van Batenburg and ReasonCraft contributors",
        "ReasonCraft is licensed under either of the following licences, at your option.",
        "==================== MIT License ====================",
        mit.trim_end(),
        "==================== Apache License, Version 2.0 ====================",
        apache.trim_end(),
    ]
    .join("\n\n")
        + "\n";
    write(
        &root.join("app/reasoncraft/packaging/LICENSE.txt"),
        text.as_bytes(),
    )
}

fn icons() -> Result<()> {
    let assets = assets_dir();
    let svg = fs::read(assets.join("icon.svg"))?;
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default())?;
    let render = |size: u32| render(&tree, size);

    fs::create_dir_all(assets.join("icons"))?;
    for size in PNG_SIZES {
        write(
            &assets.join(format!("icons/icon-{size}.png")),
            &render(size)?.encode_png()?,
        )?;
    }
    write(&assets.join("icon.png"), &render(256)?.encode_png()?)?;

    let mut ico = ico::IconDir::new(ico::ResourceType::Icon);
    for size in ICO_SIZES {
        let rgba = rgba(&render(size)?);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        ico.add_entry(ico::IconDirEntry::encode(&image)?);
    }
    ico.write(BufWriter::new(fs::File::create(assets.join("icon.ico"))?))?;
    println!("wrote {}", assets.join("icon.ico").display());

    let mut icns = IconFamily::new();
    for (size, kind) in ICNS_TYPES {
        let rgba = rgba(&render(size)?);
        let image = icns::Image::from_data(icns::PixelFormat::RGBA, size, size, rgba)?;
        icns.add_icon_with_type(&image, kind)?;
    }
    icns.write(BufWriter::new(fs::File::create(assets.join("icon.icns"))?))?;
    println!("wrote {}", assets.join("icon.icns").display());
    Ok(())
}

fn sample_map(path: &Path) -> Result<()> {
    use reasoncraft_core::examples::{Example, example};
    reasoncraft_core::file::save(&example(Example::Reasoning, "en"), path)?;
    println!("wrote {}", path.display());
    Ok(())
}

/// The SVG rendered to a `size` × `size` pixmap.
fn render(tree: &usvg::Tree, size: u32) -> Result<tiny_skia::Pixmap> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or("empty icon size")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::write(path, bytes)?;
    println!("wrote {}", path.display());
    Ok(())
}

/// The pixmap's pixels as straight (not premultiplied) RGBA bytes.
fn rgba(pixmap: &tiny_skia::Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}
