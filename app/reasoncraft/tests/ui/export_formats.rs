//! UI tests for the document export formats driven from the ribbon:
//! Markdown and Word outlines, essays, and sample files for review by eye.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::Queryable;
use reasoncraft::export::{ExportFormat, ExportOptions};
use reasoncraft::i18n::t;
use reasoncraft::jobs::Hold;

use crate::export_fixture::{click_button, harness, open_export_tab, request, wait};

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
fn export_worker_uses_document_language() {
    use reasoncraft::i18n::t_lang;
    use reasoncraft_core::{CardStrength, ImageRef};

    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, alpha) = harness(dir.path());
    let doc = &mut h.state_mut().tabs[0].doc;
    doc.meta.language = "nl".into();
    let card = doc.cards.values().find(|c| c.premises.contains(&alpha));
    let card = card.unwrap().id;
    doc.set_card_strength(card, Some(CardStrength::Strong))
        .unwrap();
    // An image whose asset is gone: the worker draws its placeholder.
    doc.boxes.get_mut(&alpha).unwrap().image = Some(ImageRef {
        asset: "gone".into(),
        aspect: 2.0,
    });
    let file = dir.path().join("map.svg");
    fakes.dialogs.set_export(file.clone());
    let options = ExportOptions {
        svg_outlines: false,
        ..ExportOptions::default()
    };
    h.state_mut().push(request(ExportFormat::Svg, options));
    h.run_steps(2);
    wait(&mut h);
    let svg = std::fs::read_to_string(&file).unwrap();
    for key in ["rating.strong.supports", "image.missing", "role.opposes"] {
        let (nl, en) = (t_lang("nl", key), t_lang("en", key));
        assert_ne!(nl, en, "{key} has a Dutch text");
        assert!(svg.contains(&format!(">{nl}<")), "{nl:?} in {svg}");
        assert!(!svg.contains(&format!(">{en}<")), "{en:?} in {svg}");
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

#[test]
fn export_argdown_via_ribbon() {
    use reasoncraft_core::ImageRef;

    let dir = tempfile::tempdir().unwrap();
    let (mut h, fakes, alpha) = harness(dir.path());
    let file = dir.path().join("map.argdown");
    fakes.dialogs.set_export(file.clone());
    open_export_tab(&mut h);
    click_button(&mut h, "export.argdown");
    wait(&mut h);
    assert!(h.state().modal.is_none());
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with(
            "===
"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "[Claim]: Claim
    <+ <R1>
    <- <O1>
"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "(1) [Alpha]: Alpha
    <+ <R2>
"
        ),
        "{text}"
    );
    assert_eq!(h.state().status_note, None, "no images, no notice");

    // A map with an image says the image is left out.
    let image = ImageRef {
        asset: "0000.png".into(),
        aspect: 0.5,
    };
    h.state_mut().tabs[0]
        .doc
        .set_image(alpha, Some(image))
        .unwrap();
    fakes.dialogs.set_export(file);
    let hold = Hold::default();
    h.state_mut().export.hold = Some(hold.clone());
    click_button(&mut h, "export.argdown");
    assert!(h.state().export.busy());
    assert_eq!(h.state().status_note, None, "said only once it is written");
    hold.release();
    wait(&mut h);
    assert_eq!(
        h.state().status_note.as_deref(),
        Some(t("export.argdown_no_images").as_str())
    );
}

/// With `RC_EXPORT_OUT=<dir>` set, writes `task-10-outline.docx` and
/// `task-10-essay.docx` (a Dutch sample with written paragraphs), for
/// looking at by eye.
#[test]
#[ignore = "manual: set RC_EXPORT_OUT=<folder>"]
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
#[ignore = "manual: set RC_EXPORT_SAMPLE=<file.rcraft> and RC_EXPORT_OUT=<folder>"]
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
