//! UI tests for exports: the Export ribbon tab, the export dialog, export
//! jobs on a worker thread, Copy as image and Print. Files go to temp
//! folders; the clipboard and the PDF viewer are fakes.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;
use egui_kittest::kittest::{NodeT, Queryable};
use reasoncraft::actions::Action;
use reasoncraft::export::{
    ExportFormat, ExportOptions, ExportScope, ExportTheme, Notice, png_to_rgba,
};
use reasoncraft::i18n::t;
use reasoncraft::theme::Tokens;
use reasoncraft_core::Item;

use crate::common::H;

use crate::export_fixture::{click_button, harness, open_export_tab, request, wait};

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
fn copy_as_image_uses_selection_when_scope_selection() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, alpha) = harness(dir.path());
    let copy = |h: &mut H| {
        h.state_mut().push(Action::CopyAsImage);
        h.run_steps(2);
        wait(h);
        let images = fakes.clipboard.images.lock().unwrap();
        let (w, hgt, _) = images.last().unwrap();
        (*w, *hgt)
    };
    let whole = copy(&mut h);
    h.state_mut().settings.export.scope = ExportScope::Selection;
    // Nothing selected: the whole map.
    h.state_mut().tabs[0].selection = None;
    assert_eq!(copy(&mut h), whole);
    // Alpha selected: the same size as a PNG of Alpha's subtree at 2x.
    h.state_mut().tabs[0].selection = Some(Item::Box(alpha));
    let part = copy(&mut h);
    assert!(part.0 < whole.0, "{part:?} is smaller than {whole:?}");
    let file = dir.path().join("part.png");
    fakes.dialogs.set_export(file.clone());
    let options = ExportOptions {
        scope: ExportScope::Selection,
        png_scale: 2,
        ..ExportOptions::default()
    };
    h.state_mut().push(request(ExportFormat::Png, options));
    h.run_steps(2);
    wait(&mut h);
    let (w, hgt, _) = png_to_rgba(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(part, (w, hgt));
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
