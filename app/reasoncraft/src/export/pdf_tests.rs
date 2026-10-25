use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use reasoncraft_core::Assets;

use super::pdf::placements;
use super::tests::{preview_doc, scene_of};
use super::*;
use crate::actions::FakeOpener;
use crate::theme::Tokens;

/// A plain SVG of `w` x `h` px.
fn rect_svg(w: u32, h: u32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         viewBox=\"0 0 {w} {h}\"><rect width=\"{w}\" height=\"{h}\" fill=\"#c04040\"/></svg>"
    )
}

fn tile(scale: f32, orientation: Orientation) -> PageSetup {
    PageSetup {
        orientation,
        fit: Fit::Tile { scale },
        ..PageSetup::default()
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The number of `/Type /Page` objects (not `/Type /Pages`).
fn page_count(pdf: &[u8]) -> usize {
    let key = b"/Type /Page";
    pdf.windows(key.len() + 1)
        .filter(|w| &w[..key.len()] == key && w[key.len()] != b's')
        .count()
}

/// Every stream's data, inflated when it is flate-compressed.
fn streams(pdf: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(p) = find(&pdf[at..], b">>\nstream\n") {
        let dict_end = at + p;
        let dict_start = pdf[..dict_end]
            .windows(2)
            .rposition(|w| w == b"<<")
            .unwrap();
        // The stream dictionary may hold nested dictionaries; take the
        // text from the object header on.
        let obj = pdf[..dict_end]
            .windows(3)
            .rposition(|w| w == b"obj")
            .unwrap_or(dict_start);
        let dict = String::from_utf8_lossy(&pdf[obj..dict_end]).into_owned();
        let len: usize = dict
            .split("/Length ")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse().ok())
            .expect("stream has a direct /Length");
        let start = dict_end + b">>\nstream\n".len();
        let data = &pdf[start..start + len];
        out.push(if dict.contains("/FlateDecode") {
            miniz_oxide::inflate::decompress_to_vec_zlib(data).expect("valid flate stream")
        } else {
            data.to_vec()
        });
        at = start + len;
    }
    out
}

/// The ToUnicode mappings (`beginbfchar`) of every font: CID → text.
fn to_unicode(streams: &[Vec<u8>]) -> HashMap<u16, String> {
    let hex = |s: &str| -> Vec<u16> {
        let s = s.trim_matches(|c| c == '<' || c == '>');
        (0..s.len() / 4)
            .map(|i| u16::from_str_radix(&s[4 * i..4 * i + 4], 16).unwrap())
            .collect()
    };
    let mut map = HashMap::new();
    for s in streams {
        let text = String::from_utf8_lossy(s);
        for block in text.split("beginbfchar").skip(1) {
            let block = &block[..block.find("endbfchar").unwrap()];
            for line in block.lines().filter(|l| !l.trim().is_empty()) {
                let mut parts = line.split_whitespace();
                let cid = hex(parts.next().unwrap())[0];
                let units = hex(parts.next().unwrap());
                map.insert(cid, String::from_utf16(&units).unwrap());
            }
        }
    }
    map
}

/// The operands of every `Tj` in a content stream, as bytes.
fn shown_strings(content: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut last = Vec::new();
    let mut i = 0;
    while i < content.len() {
        match content[i] {
            b'(' => {
                last.clear();
                i += 1;
                while content[i] != b')' {
                    if content[i] == b'\\' {
                        i += 1;
                        if content[i].is_ascii_digit() {
                            let end = (i..i + 3)
                                .take_while(|&j| content[j].is_ascii_digit())
                                .last()
                                .unwrap();
                            let oct = std::str::from_utf8(&content[i..=end]).unwrap();
                            last.push(u8::from_str_radix(oct, 8).unwrap());
                            i = end;
                        } else {
                            last.push(match content[i] {
                                b'n' => b'\n',
                                b'r' => b'\r',
                                b't' => b'\t',
                                b'b' => 8,
                                b'f' => 12,
                                c => c,
                            });
                        }
                    } else {
                        last.push(content[i]);
                    }
                    i += 1;
                }
            }
            b'<' if content.get(i + 1) != Some(&b'<') => {
                let end = i + content[i..].iter().position(|&c| c == b'>').unwrap();
                let hex = std::str::from_utf8(&content[i + 1..end]).unwrap();
                last = (0..hex.len() / 2)
                    .map(|k| u8::from_str_radix(&hex[2 * k..2 * k + 2], 16).unwrap())
                    .collect();
                i = end;
            }
            b'T' if content.get(i + 1) == Some(&b'j') => out.push(last.clone()),
            _ => {}
        }
        i += 1;
    }
    out
}

/// The text shown with two-byte (Identity-H) fonts, decoded through the
/// ToUnicode maps, in drawing order.
fn extract_text(pdf: &[u8]) -> String {
    let streams = streams(pdf);
    let map = to_unicode(&streams);
    let mut text = String::new();
    // Content streams are plain ASCII; font programs are not.
    for s in streams.iter().filter(|s| s.is_ascii()) {
        for shown in shown_strings(s).iter().filter(|b| b.len() % 2 == 0) {
            for pair in shown.chunks(2) {
                let cid = u16::from_be_bytes([pair[0], pair[1]]);
                if let Some(t) = map.get(&cid) {
                    text.push_str(t);
                }
            }
        }
    }
    text
}

/// The preview map as live-text SVG, and the rows its boxes wrap into.
fn preview_svg() -> (String, Vec<Vec<String>>) {
    let mut assets = Assets::default();
    let doc = preview_doc(&mut assets);
    let (scene, rows) = scene_of(&doc);
    let opts = SvgOptions {
        text_as_paths: false,
        ..SvgOptions::new(&Tokens::light())
    };
    let svg = scene_to_svg(&scene, &assets, &opts, &bundled_font_family());
    (svg, rows)
}

#[test]
fn pdf_one_page_has_one_page() {
    let (svg, _) = preview_svg();
    let pdf = svg_to_pdf(&svg, &PageSetup::default(), &bundled_fontdb()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(page_count(&pdf), 1);
    assert!(
        find(&pdf, b"/MediaBox [0 0 595.27").is_some(),
        "A4 portrait"
    );
    assert!(find(&pdf, b"Helvetica").is_none(), "no page labels");
    // A huge drawing still fits on one page.
    let pdf = svg_to_pdf(
        &rect_svg(20_000, 9_000),
        &PageSetup::default(),
        &bundled_fontdb(),
    )
    .unwrap();
    assert_eq!(page_count(&pdf), 1);
}

#[test]
fn one_page_is_scaled_to_fit_and_centred() {
    let setup = PageSetup {
        orientation: Orientation::Landscape,
        ..PageSetup::default()
    };
    let (pw, ph) = setup.page_size_pt();
    let m = 10.0 * 72.0 / 25.4;
    // Wider than the page area: the width decides the scale.
    let (pages, _) = placements(2000.0, 500.0, &setup).unwrap();
    let p = pages[0];
    assert!((2000.0 * p.scale - (pw - 2.0 * m)).abs() < 0.01);
    assert!((p.left - m).abs() < 0.01);
    let mid_y = p.top - 500.0 * p.scale / 2.0;
    assert!((mid_y - ph / 2.0).abs() < 0.01, "centred vertically");
    assert_eq!(p.tile, None);
}

#[test]
fn pdf_tile_page_count() {
    let fonts = bundled_fontdb();
    // 1000 x 600 px at 0.25 mm/px = 250 x 150 mm; A4 portrait leaves
    // 190 x 277 mm inside the margins.
    let svg = rect_svg(1000, 600);
    let count = |setup: PageSetup| page_count(&svg_to_pdf(&svg, &setup, &fonts).unwrap());
    assert_eq!(count(tile(1.0, Orientation::Portrait)), 2);
    assert_eq!(count(tile(1.0, Orientation::Landscape)), 1);
    // 500 x 300 mm: 3 columns, 2 rows.
    assert_eq!(count(tile(2.0, Orientation::Portrait)), 6);
    let letter = PageSetup {
        paper: Paper::Letter,
        ..tile(2.0, Orientation::Landscape)
    };
    // Letter landscape: 259.4 x 195.9 mm inside the margins.
    assert_eq!(count(letter), 4);
}

#[test]
fn tiles_are_row_major_with_labels() {
    let setup = tile(2.0, Orientation::Portrait);
    let (pages, [_, _, aw, ah]) = placements(1000.0, 600.0, &setup).unwrap();
    let tiles: Vec<_> = pages.iter().map(|p| p.tile.unwrap()).collect();
    assert_eq!(tiles, [(1, 1), (1, 2), (1, 3), (2, 1), (2, 2), (2, 3)]);
    // Neighbouring tiles continue where the last one stopped.
    assert!((pages[0].left - pages[1].left - aw).abs() < 0.01);
    assert!((pages[3].top - pages[0].top - ah).abs() < 0.01);
    // 1 px = 0.5 mm at scale 2.
    assert!((pages[0].scale - 0.5 * 72.0 / 25.4).abs() < 1e-4);

    let pdf = svg_to_pdf(&rect_svg(1000, 600), &setup, &bundled_fontdb()).unwrap();
    for label in ["1/1", "1/3", "2/3"] {
        let shown = format!("({label}) Tj");
        assert!(find(&pdf, shown.as_bytes()).is_some(), "label {label}");
    }
    assert!(find(&pdf, b"/BaseFont /Helvetica").is_some());
}

#[test]
fn pdf_refuses_bad_setups() {
    let svg = rect_svg(1000, 600);
    let fonts = bundled_fontdb();
    for setup in [
        tile(0.0, Orientation::Portrait),
        tile(f32::NAN, Orientation::Portrait),
        tile(500.0, Orientation::Portrait), // too many pages
        PageSetup {
            margin_mm: 120.0,
            ..PageSetup::default()
        },
    ] {
        let err = svg_to_pdf(&svg, &setup, &fonts).unwrap_err();
        assert_eq!(err.key(), "export-error-pdf", "{setup:?}");
    }
}

#[test]
fn pdf_contains_font_and_text() {
    let (svg, rows) = preview_svg();
    let pdf = svg_to_pdf(&svg, &PageSetup::default(), &bundled_fontdb()).unwrap();
    assert!(find(&pdf, b"/FontFile").is_some(), "fonts are embedded");
    assert!(find(&pdf, b"/ToUnicode").is_some());
    let text = extract_text(&pdf);
    for row in rows.iter().flatten() {
        assert!(text.contains(row.as_str()), "{row:?} not in {text:?}");
    }
    assert!(text.contains("Het centrum"));
    assert!(text.contains("één"), "diacritics survive");
}

#[test]
fn print_writes_temp_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let opener = FakeOpener::default();
    let path = print_via_pdf_with(b"%PDF-1.7 test", dir.path(), &opener).unwrap();
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("ReasonCraft-print-") && name.ends_with(".pdf"));
    assert_eq!(path.parent().unwrap(), dir.path());
    assert_eq!(std::fs::read(&path).unwrap(), b"%PDF-1.7 test");
    assert_eq!(
        *opener.opened.borrow(),
        [path.to_string_lossy().into_owned()]
    );

    *opener.fail.borrow_mut() = Some("no viewer".into());
    let err = print_via_pdf_with(b"%PDF", dir.path(), &opener).unwrap_err();
    assert_eq!(err.key(), "export-error-open");
}

#[test]
fn old_print_files_are_cleaned_up() {
    let dir = tempfile::tempdir().unwrap();
    let names = [
        "ReasonCraft-print-1.pdf",
        "ReasonCraft-print-2.pdf",
        "ReasonCraft-print-3.txt",
        "Other-print-4.pdf",
    ];
    for name in names {
        std::fs::write(dir.path().join(name), b"x").unwrap();
    }
    let two_days_ago = SystemTime::now() - Duration::from_secs(2 * 24 * 60 * 60);
    for name in [
        "ReasonCraft-print-1.pdf",
        "ReasonCraft-print-3.txt",
        "Other-print-4.pdf",
    ] {
        let file = std::fs::File::options()
            .write(true)
            .open(dir.path().join(name))
            .unwrap();
        file.set_modified(two_days_ago).unwrap();
    }
    clean_print_files(dir.path(), SystemTime::now(), PRINT_FILE_MAX_AGE);
    let left: Vec<bool> = names.iter().map(|n| dir.path().join(n).exists()).collect();
    // Only the old print PDF goes; the new one and other files stay.
    assert_eq!(left, [false, true, true, true]);
}

#[test]
fn page_setup_defaults_and_old_settings() {
    let setup = PageSetup::default();
    assert_eq!(setup.paper, Paper::A4);
    assert_eq!(setup.orientation, Orientation::Portrait);
    assert_eq!(setup.fit, Fit::OnePage);
    assert_eq!(setup.margin_mm, 10.0);
    // Settings saved before page setup existed still load.
    let old: crate::settings::Settings = serde_json::from_str(r#"{"author":"Ann"}"#).unwrap();
    assert_eq!(old.page_setup, setup);
    let tiled = PageSetup {
        fit: Fit::Tile { scale: 1.5 },
        ..setup
    };
    let json = serde_json::to_string(&tiled).unwrap();
    assert_eq!(serde_json::from_str::<PageSetup>(&json).unwrap(), tiled);
}
