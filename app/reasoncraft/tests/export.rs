//! UI tests for exports: the Export ribbon tab, the export dialog, export
//! jobs on a worker thread, Copy as image and Print. Files go to temp
//! folders; the clipboard and the PDF viewer are fakes.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use eframe::egui::accesskit::Role;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, FakeOpener};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{
    ExportFormat, ExportOptions, ExportRequest, ExportScope, ExportTheme, FakeClipboard, Notice,
    PageSetup, png_to_rgba,
};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft::theme::Tokens;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent};

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
fn export_png_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    // No extension typed: ".png" is added.
    fakes.dialogs.set_export(dir.path().join("map"));
    open_export_tab(&mut h);
    h.get_by_label(&t("export.png")).click();
    h.run();
    assert!(h.state().export_dialog.is_some(), "the dialog opens first");
    click_button(&mut h, "export.dialog.export");
    wait(&mut h);

    let file = dir.path().join("map.png");
    let (w, _, px) = png_to_rgba(&std::fs::read(&file).unwrap()).unwrap();
    assert!(w > 100);
    // The app shows the dark theme; exports default to the light one.
    let c = Tokens::light().canvas;
    assert_eq!(px[..4], [c.r(), c.g(), c.b(), 255]);
    assert_eq!(h.state().export.notice, Some(Notice::Saved(file)));
    assert!(h.state().export_dialog.is_none());

    h.get_by_label(&t("export.done"));
    h.get_by_label(&t("export.open_folder")).click();
    h.run();
    assert_eq!(
        *fakes.opener.opened.borrow(),
        [dir.path().to_string_lossy().into_owned()]
    );
}

#[test]
fn export_dialog_defaults_to_light_theme() {
    let (mut h, _, _) = harness(tempfile::tempdir().unwrap().path());
    assert_eq!(ExportOptions::default().theme, ExportTheme::Light);
    h.state_mut().push(Action::OpenExport(ExportFormat::Svg));
    h.run();
    let dialog = h.state().export_dialog.clone().unwrap();
    assert_eq!(dialog.options.theme, ExportTheme::Light);
    assert!(dialog.options.svg_outlines, "outlines are on by default");
    let label = t("export.theme.light");
    let light = h.get_by_label(&label);
    let toggled = light.accesskit_node().toggled();
    assert_eq!(toggled, Some(egui::accesskit::Toggled::True));
    // With outlines on, the dialog says emoji are not kept.
    h.get_by_label(&t("export.note.emoji"));
}

#[test]
fn export_runs_off_ui_thread() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    let delay = Duration::from_millis(600);
    h.state_mut().export.delay = Some(delay);
    let file = dir.path().join("slow.png");
    fakes.dialogs.set_export(file.clone());
    h.state_mut()
        .push(request(ExportFormat::Png, ExportOptions::default()));
    let start = Instant::now();
    h.step();
    h.step();
    for _ in 0..5 {
        h.step();
    }
    assert!(
        start.elapsed() < delay,
        "frames keep coming while the export waits"
    );
    assert!(h.state().export.busy());
    assert!(!file.exists());
    h.get_by_label(&t("status.exporting"));
    wait(&mut h);
    assert!(file.exists());
    assert!(h.query_by_label(&t("status.exporting")).is_none());
}

#[test]
fn export_selection_scope_only_subtree() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, alpha) = harness(dir.path());
    h.state_mut().tabs[0].selection = Some(Item::Box(alpha));
    let file = dir.path().join("part.svg");
    fakes.dialogs.set_export(file.clone());
    let options = ExportOptions {
        scope: ExportScope::Selection,
        svg_outlines: false,
        ..ExportOptions::default()
    };
    h.state_mut().push(request(ExportFormat::Svg, options));
    h.run_steps(2);
    wait(&mut h);
    let svg = std::fs::read_to_string(&file).unwrap();
    assert!(svg.contains(">Alpha<") && svg.contains(">Deep<"), "{svg}");
    assert!(!svg.contains("Beta") && !svg.contains("Claim"));
    // The choice is remembered for next time.
    assert_eq!(h.state().settings.export, options);
}

#[test]
fn copy_as_image_puts_image_on_clipboard() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    // The Home tab's button.
    h.get_by_label(&t("action.copy_image")).click();
    h.run_steps(2);
    wait(&mut h);
    let images = fakes.clipboard.images.lock().unwrap();
    assert_eq!(images.len(), 1);
    let (w, hgt, px) = &images[0];
    assert_eq!(px.len() as u32, w * hgt * 4);
    // The export theme (light by default) while the UI is dark, at scale 2.
    assert!(h.state().tokens.dark);
    let c = Tokens::light().canvas;
    assert_eq!(px[..4], [c.r(), c.g(), c.b(), 255]);
    assert!(w % 2 == 0 && hgt % 2 == 0);
    drop(images);
    assert_eq!(h.state().export.notice, Some(Notice::Copied));
    h.get_by_label(&t("export.copied"));

    // With the current theme chosen for exports, the copy is dark.
    h.state_mut().settings.export.theme = ExportTheme::Current;
    h.get_by_label(&t("action.copy_image")).click();
    h.run_steps(2);
    wait(&mut h);
    let images = fakes.clipboard.images.lock().unwrap();
    let c = Tokens::dark().canvas;
    assert_eq!(images[1].2[..4], [c.r(), c.g(), c.b(), 255]);
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
fn print_writes_pdf_and_opens_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    open_export_tab(&mut h);
    h.get_by_label(&t("export.print")).click();
    h.run_steps(2);
    wait(&mut h);
    let opened = fakes.opener.opened.borrow().clone();
    assert_eq!(opened.len(), 1);
    let path = PathBuf::from(&opened[0]);
    assert_eq!(path.parent().unwrap(), dir.path());
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn failed_export_shows_error_and_essay_waits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, _) = harness(dir.path());
    // A folder that does not exist cannot take the file.
    fakes
        .dialogs
        .set_export(dir.path().join("missing").join("map.svg"));
    h.state_mut()
        .push(request(ExportFormat::Svg, ExportOptions::default()));
    h.run_steps(2);
    wait(&mut h);
    assert!(h.state().modal.is_some(), "an error modal");
    h.get_by_label(&t("export.error.title"));
    h.get_by_label(&t("modal.ok")).click();
    h.run();

    open_export_tab(&mut h);
    click_button(&mut h, "export.essay");
    for key in ["export.format.essay_md", "export.format.essay_docx"] {
        assert!(!h.get_by_label(&t(key)).accesskit_node().is_disabled());
    }
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
