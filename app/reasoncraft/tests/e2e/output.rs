//! End to end: write the essay (introduction and two sections),
//! delete a branch with text and undo it, export the map, the outline and
//! the essay in every file format through the app's own commands, check
//! that each file is valid, then save, reopen and compare the essay.

use std::io::{Cursor, Read};
use std::path::Path;

use reasoncraft::actions::{Action, Rate, poll_exports};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft_core::{EssaySlot, Item, NodeId, essay_order};

use super::{card_of, editing, run, typed};

/// Waits (at most 60 s) until no export runs, picking up the results as
/// the frame loop would.
fn wait_for_exports(app: &mut ReasonCraftApp) {
    crate::common::poll_until("the exports", || {
        poll_exports(app);
        !app.export.busy()
    });
}

fn essay(slot: EssaySlot, text: &str) -> Action {
    Action::EditEssayText {
        slot,
        text: text.to_owned(),
    }
}

fn paragraph(b: NodeId, text: &str) -> Action {
    essay(EssaySlot::Item(Item::Box(b)), text)
}

/// Exports the active map as `format` to `dir/out.<ext>` and returns the
/// file's bytes.
pub(super) fn export(
    app: &mut ReasonCraftApp,
    fake: &FakeDialogs,
    dir: &Path,
    format: ExportFormat,
) -> Vec<u8> {
    let name = format!("{format:?}.{}", format.extension());
    let path = dir.join(name);
    fake.set_export(path.clone());
    let request = ExportRequest {
        format,
        options: ExportOptions::default(),
        page: PageSetup::default(),
    };
    run(app, [Action::RunExport(request)]);
    wait_for_exports(app);
    assert!(app.modal.is_none(), "{format:?} failed: {:?}", app.modal);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{format:?}: {e}"))
}

/// PNG width and height from the IHDR chunk, after checking the signature.
fn png_size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    assert_eq!(&png[12..16], b"IHDR");
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

/// The number of `/Type /Page` objects (not `/Type /Pages`).
fn pdf_pages(pdf: &[u8]) -> usize {
    let key = b"/Type /Page";
    pdf.windows(key.len() + 1)
        .filter(|w| &w[..key.len()] == key && w[key.len()] != b's')
        .count()
}

/// Checks the OOXML parts Word needs and returns the text of
/// `word/document.xml`'s paragraphs.
fn docx_text(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    let mut part = |name: &str| {
        let mut s = String::new();
        zip.by_name(name)
            .unwrap_or_else(|_| panic!("{name} missing"))
            .read_to_string(&mut s)
            .unwrap();
        s
    };
    let types = part("[Content_Types].xml");
    roxmltree::Document::parse(&types).expect("well-formed content types");
    let xml = part("word/document.xml");
    let doc = roxmltree::Document::parse(&xml).expect("well-formed document.xml");
    doc.descendants()
        .filter(|n| n.has_tag_name("t"))
        .filter_map(|n| n.text())
        .collect::<Vec<_>>()
        .join(" ")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("UTF-8")
}

#[test]
fn e2e_essay_exports_save_reopen() {
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let root = app.active_tab().doc.root;
    let dir = tempfile::tempdir().unwrap();

    // A claim, a strong reason, a weak one, and an objection.
    run(
        &mut app,
        [
            typed("Schools should start later"),
            Action::AddReason,
            typed("Teenagers sleep better"),
        ],
    );
    let strong = editing(&app);
    run(
        &mut app,
        [Action::Select(Some(Item::Box(root))), Action::AddReason],
    );
    run(&mut app, [typed("Fewer accidents on the way")]);
    let weak = editing(&app);
    run(
        &mut app,
        [Action::Select(Some(Item::Box(root))), Action::AddObjection],
    );
    run(&mut app, [typed("Buses cost more")]);
    let objection = editing(&app);
    let (strong_card, weak_card) = (card_of(&app, strong), card_of(&app, weak));
    run(
        &mut app,
        [
            Action::Select(Some(Item::Card(strong_card))),
            Action::Rate(Rate::High),
            Action::Select(Some(Item::Card(weak_card))),
            Action::Rate(Rate::Mid),
        ],
    );
    let order: Vec<Item> = essay_order(&app.active_tab().doc)
        .iter()
        .map(|s| s.item)
        .collect();
    assert_eq!(order.len(), 3);
    assert_eq!(
        order[0],
        Item::Card(card_of(&app, strong)),
        "strongest first"
    );

    // The essay: the introduction and two sections, one step each.
    let steps = app.active_tab().history.undo.len();
    run(
        &mut app,
        [
            essay(EssaySlot::Intro, "Start times"),
            essay(EssaySlot::Intro, "Start times matter."),
            paragraph(strong, "Sleep research is clear."),
            paragraph(weak, "Rested drivers crash less."),
            Action::StopEditing,
        ],
    );
    assert_eq!(app.active_tab().history.undo.len(), steps + 3);

    // Deleting the weak branch keeps its paragraph as an unplaced note;
    // undo puts it back in its slot.
    run(
        &mut app,
        [
            Action::Select(Some(Item::Card(weak_card))),
            Action::DeleteBranch,
        ],
    );
    let doc = &app.active_tab().doc;
    assert!(!doc.boxes.contains_key(&weak));
    assert_eq!(
        doc.essay.unplaced,
        [reasoncraft_core::UnplacedNote::new(
            "Fewer accidents on the way".to_owned(),
            "Rested drivers crash less.".to_owned()
        )]
    );
    run(&mut app, [Action::Undo]);
    let doc = &app.active_tab().doc;
    assert!(doc.essay.unplaced.is_empty());
    assert_eq!(
        doc.essay.text(EssaySlot::Item(Item::Box(weak))),
        Some("Rested drivers crash less.")
    );
    assert!(doc.boxes.contains_key(&objection));

    // Every export format, through the app's commands.
    let png = export(&mut app, &fake, dir.path(), ExportFormat::Png);
    let svg = export(&mut app, &fake, dir.path(), ExportFormat::Svg);
    let pdf = export(&mut app, &fake, dir.path(), ExportFormat::Pdf);
    let outline_md = text(&export(
        &mut app,
        &fake,
        dir.path(),
        ExportFormat::OutlineMarkdown,
    ));
    let outline_txt = text(&export(
        &mut app,
        &fake,
        dir.path(),
        ExportFormat::OutlineText,
    ));
    let outline_docx = export(&mut app, &fake, dir.path(), ExportFormat::OutlineDocx);
    let essay_md = text(&export(
        &mut app,
        &fake,
        dir.path(),
        ExportFormat::EssayMarkdown,
    ));
    let essay_docx = export(&mut app, &fake, dir.path(), ExportFormat::EssayDocx);

    // The SVG parses; the PNG is the same picture at the default 2×.
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())
        .expect("the SVG parses");
    let (w, h) = png_size(&png);
    let size = tree.size();
    assert!(
        (w as f32 - 2.0 * size.width()).abs() <= 2.0,
        "{w} vs {size:?}"
    );
    assert!(
        (h as f32 - 2.0 * size.height()).abs() <= 2.0,
        "{h} vs {size:?}"
    );
    // A small map fits one page.
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(pdf_pages(&pdf), 1);

    for t in [&outline_md, &outline_txt] {
        assert!(t.contains("Schools should start later"), "{t}");
        assert!(t.contains("Buses cost more"), "{t}");
    }
    let outline_word = docx_text(&outline_docx);
    assert!(
        outline_word.contains("Teenagers sleep better"),
        "{outline_word}"
    );
    for t in [essay_md.as_str(), &docx_text(&essay_docx)] {
        for s in [
            "Start times matter.",
            "Sleep research is clear.",
            "Rested drivers crash less.",
        ] {
            assert!(t.contains(s), "{s:?} missing in {t}");
        }
    }
    // Strongest first in the essay too.
    let at = |s| essay_md.find(s).unwrap();
    assert!(at("Sleep research is clear.") < at("Rested drivers crash less."));

    // Save, close, open again: the essay is the same.
    let written = app.active_tab().doc.essay.clone();
    let path = dir.path().join("school.rcraft");
    fake.set_save(path.clone());
    run(&mut app, [Action::Save]);
    assert!(app.modal.is_none(), "save failed: {:?}", app.modal);
    run(&mut app, [Action::CloseTab(0)]);
    assert!(app.modal.is_none(), "a saved tab closes without asking");
    fake.set_open(path);
    run(&mut app, [Action::Open]);
    assert!(app.modal.is_none(), "open failed: {:?}", app.modal);
    assert_eq!(app.active_tab().doc.essay, written);
    assert!(!app.active_tab().is_dirty());
}
