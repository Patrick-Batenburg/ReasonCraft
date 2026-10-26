//! UI tests for the document export formats driven from the ribbon:
//! Markdown and Word outlines, essays, and sample files for review by eye.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use eframe::egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, FakeOpener};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, FakeClipboard, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, NodeId, Parent};

type H = Harness<'static, ReasonCraftApp>;

/// What a test steers the app with.
struct Fakes {
    dialogs: FakeDialogs,
    opener: FakeOpener,
    clipboard: FakeClipboard,
}

/// "Claim" with reasons "Alpha" (with "Deep" under it) and "Beta"; returns
/// the harness and Alpha's box.
fn harness(dir: &Path) -> (H, Fakes, NodeId) {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (alpha_card, alpha) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Alpha")
        .unwrap();
    let _ = alpha_card;
    doc.add_card(Parent::Box(alpha), CardKind::Reason, "Deep")
        .unwrap();
    doc.add_card(Parent::Box(root), CardKind::Objection, "Beta")
        .unwrap();
    let fakes = Fakes {
        dialogs: FakeDialogs::default(),
        opener: FakeOpener::default(),
        clipboard: FakeClipboard::default(),
    };
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fakes.dialogs.clone()));
    app.tabs[0] = DocTab::from_document(doc);
    app.link_opener = Box::new(fakes.opener.clone());
    app.export.clipboard = Arc::new(Mutex::new(fakes.clipboard.clone()));
    app.export.print_dir = dir.to_owned();
    app.left_panel = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    (h, fakes, alpha)
}

/// Steps frames until no export is running (at most 60 s).
fn wait(h: &mut H) {
    let start = Instant::now();
    while h.state().export.busy() {
        assert!(start.elapsed() < Duration::from_secs(60), "export hangs");
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run();
}

fn request(format: ExportFormat, options: ExportOptions) -> Action {
    Action::RunExport(ExportRequest {
        format,
        options,
        page: PageSetup::default(),
    })
}

/// Clicks the button labelled with `key`'s text (the Text panel's tabs
/// and the ribbon's tabs share some names) and steps a few frames (the
/// busy spinner keeps repainting, so `run` would not settle).
fn click_button(h: &mut H, key: &str) {
    let label = t(key);
    h.get_by_role_and_label(Role::Button, &label).click();
    h.run_steps(4);
}

fn open_export_tab(h: &mut H) {
    h.get_by_role_and_label(Role::Tab, &t("ribbon.tab.export"))
        .click();
    h.run();
}

#[test]
fn outline_markdown_export_uses_doc_language() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    h.state_mut().tabs[0].doc.meta.language = "nl".into();
    let file = dir.path().join("outline.md");
    fakes.dialogs.set_export(file.clone());
    open_export_tab(&mut h);
    click_button(&mut h, "export.outline");
    h.get_by_label(&t("export.format.outline_md")).click();
    h.run_steps(2);
    wait(&mut h);
    let md = std::fs::read_to_string(&file).unwrap();
    assert!(md.contains("omdat"), "{md}");
    assert!(!md.contains("because"));
    assert!(md.contains("Alpha") && md.contains("Beta"));
}

#[test]
fn word_and_essay_exports_run_from_the_ribbon() {
    use reasoncraft_core::EssaySlot;

    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    h.state_mut().tabs[0]
        .doc
        .set_essay_text(EssaySlot::Intro, "Written introduction")
        .unwrap();
    for (menu, format, name) in [
        (
            "export.outline",
            "export.format.outline_docx",
            "outline.docx",
        ),
        ("export.essay", "export.format.essay_md", "essay.md"),
        ("export.essay", "export.format.essay_docx", "essay.docx"),
    ] {
        let file = dir.path().join(name);
        fakes.dialogs.set_export(file.clone());
        open_export_tab(&mut h);
        click_button(&mut h, menu);
        h.get_by_label(&t(format)).click();
        h.run_steps(2);
        wait(&mut h);
        assert!(h.state().modal.is_none(), "{name}");
        let bytes = std::fs::read(&file).unwrap();
        if name.ends_with(".md") {
            assert!(
                String::from_utf8(bytes)
                    .unwrap()
                    .contains("Written introduction")
            );
        } else {
            assert!(bytes.starts_with(b"PK"), "{name} is a zip");
        }
    }
}

#[test]
fn essay_text_export_writes_plain_text() {
    use reasoncraft_core::EssaySlot;

    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    h.state_mut().tabs[0]
        .doc
        .set_essay_text(EssaySlot::Intro, "Written introduction")
        .unwrap();
    let file = dir.path().join("essay.txt");
    fakes.dialogs.set_export(file.clone());
    open_export_tab(&mut h);
    click_button(&mut h, "export.essay");
    h.get_by_label(&t("export.format.essay_text")).click();
    h.run_steps(2);
    wait(&mut h);
    assert!(h.state().modal.is_none());
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("Introduction"), "{text}");
    assert!(text.contains("Written introduction"), "{text}");
    assert!(!text.contains('#'), "{text}");
}

/// With `RC_EXPORT_OUT=<dir>` set, writes `task-10-outline.docx` and
/// `task-10-essay.docx` (a Dutch sample with written paragraphs), for
/// looking at by eye.
#[test]
fn export_sample_docx() {
    use reasoncraft_core::{EssaySlot, essay_order, fixtures::sample_iot_like};

    let Some(out) = std::env::var_os("RC_EXPORT_OUT") else {
        return;
    };
    let out = PathBuf::from(out);
    let (mut h, fakes, _) = harness(&out);
    let mut doc = sample_iot_like(30);
    doc.meta.language = "nl".into();
    doc.meta.title = "Slimme sensoren in huis".into();
    doc.meta.author = "Patrick van Batenburg".into();
    doc.set_essay_text(
        EssaySlot::Intro,
        "Slimme sensoren maken het huis veiliger.

Dat licht ik hieronder toe.",
    )
    .unwrap();
    let first = essay_order(&doc).remove(0).elements.remove(0).slot;
    doc.set_essay_text(
        first,
        "Lekkages worden vroeg gemeld, zodat de schade beperkt blijft.",
    )
    .unwrap();
    h.state_mut().tabs[0] = reasoncraft::doc_tab::DocTab::from_document(doc);
    for (format, name) in [
        (ExportFormat::OutlineDocx, "task-10-outline.docx"),
        (ExportFormat::EssayDocx, "task-10-essay.docx"),
    ] {
        fakes.dialogs.set_export(out.join(name));
        h.state_mut()
            .push(request(format, ExportOptions::default()));
        h.run_steps(2);
        wait(&mut h);
        assert!(h.state().modal.is_none(), "{name}");
    }
}

/// With `RC_EXPORT_SAMPLE=<file.rcraft>` and `RC_EXPORT_OUT=<dir>` set,
/// exports the file as `task-7-map.png`, `.svg`, `.pdf` and
/// `task-7-outline.md` through the app, for looking at by eye; the SVG is
/// also rendered to `task-7-svg-render.png`.
#[test]
fn export_sample_files() {
    let (Some(sample), Some(out)) = (
        std::env::var_os("RC_EXPORT_SAMPLE"),
        std::env::var_os("RC_EXPORT_OUT"),
    ) else {
        return;
    };
    let out = PathBuf::from(out);
    let (mut h, fakes, _) = harness(&out);
    reasoncraft::file_ops::open_into_app(h.state_mut(), Path::new(&sample));
    h.run();
    let jobs = [
        (ExportFormat::Png, "task-7-map.png"),
        (ExportFormat::Svg, "task-7-map.svg"),
        (ExportFormat::Pdf, "task-7-map.pdf"),
        (ExportFormat::OutlineMarkdown, "task-7-outline.md"),
    ];
    for (format, name) in jobs {
        fakes.dialogs.set_export(out.join(name));
        h.state_mut()
            .push(request(format, ExportOptions::default()));
        h.run_steps(2);
        wait(&mut h);
        assert!(h.state().modal.is_none(), "{name}");
    }
    let svg = std::fs::read_to_string(out.join("task-7-map.svg")).unwrap();
    let png = reasoncraft::export::svg_to_png(
        &svg,
        &reasoncraft::export::PngOptions::default(),
        &reasoncraft::export::bundled_fontdb(),
    )
    .unwrap();
    std::fs::write(out.join("task-7-svg-render.png"), png).unwrap();
}
