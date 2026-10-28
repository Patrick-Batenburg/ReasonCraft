//! UI tests for the Outline tab of the Text panel.

use std::time::{Duration, Instant};

use eframe::egui::{self, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::{t, t_lang};
use reasoncraft::settings::{OutlineToggle, Settings};
use reasoncraft::{ReasonCraftApp, actions::Action};
use reasoncraft_core::fixtures::sample_iot_like;
use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::outline_md::outline_plain_text_lang;
use reasoncraft_core::{CardKind, Document, Item, Parent, SourceType};

fn harness_with(doc: Document) -> Harness<'static, ReasonCraftApp> {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        // Real frame times, so the 2 s "Copied" notice can be timed.
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

/// Claim, a reason "Robots help", and an objection "Robots break".
fn small_doc() -> Document {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    d.add_card(root, CardKind::Reason, "Robots help").unwrap();
    d.add_card(root, CardKind::Objection, "Robots break")
        .unwrap();
    d
}

fn box_with_text(doc: &Document, text: &str) -> reasoncraft_core::NodeId {
    *doc.boxes.iter().find(|(_, b)| b.text == text).unwrap().0
}

fn has_row(h: &Harness<'static, ReasonCraftApp>, label: &str) -> bool {
    h.query_by_role_and_label(Role::ListItem, label).is_some()
}

fn row_count(h: &Harness<'static, ReasonCraftApp>) -> usize {
    h.query_all_by_role(Role::ListItem).count()
}

#[test]
fn outline_click_selects_box() {
    let doc = small_doc();
    let premise = box_with_text(&doc, "Robots break");
    let mut h = harness_with(doc);
    h.run();
    assert!(has_row(&h, "Claim"));
    assert!(has_row(&h, "1 because Robots help"));
    let row = || "2 but Robots break";
    assert!(!toggled(&h, row()));

    // Pan the map far away, so the click must bring the box back into view.
    h.state_mut().tabs[0].camera.offset = egui::vec2(-20_000.0, -20_000.0);
    let before = h.state().tabs[0].camera.offset;
    h.get_by_role_and_label(Role::ListItem, row()).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.selection, Some(Item::Box(premise)));
    assert_ne!(tab.camera.offset, before, "the canvas revealed the box");
    assert!(toggled(&h, row()), "the selected row is highlighted");
    assert!(!toggled(&h, "Claim"));
}

fn toggled(h: &Harness<'static, ReasonCraftApp>, label: &str) -> bool {
    h.get_by_role_and_label(Role::ListItem, label)
        .accesskit_node()
        .toggled()
        == Some(egui::accesskit::Toggled::True)
}

#[test]
fn outline_toggles_persist() {
    let mut h = harness_with(small_doc());
    h.run();
    let s = &h.state().settings;
    assert_eq!(s.outline, OutlineOptions::default());
    assert!(s.outline_indent && s.outline_colour);
    for key in [
        "outline.toggle.numbering",
        "outline.toggle.labels",
        "outline.toggle.indent",
        "outline.toggle.colour",
        "outline.toggle.notes",
        "outline.copy_all",
        "outline.copied",
        "outline.empty",
    ] {
        assert_ne!(t(key), key, "missing ftl entry");
    }

    h.get_by_label(&t("outline.toggle.numbering")).click();
    h.run();
    assert!(!h.state().settings.outline.numbering);
    assert!(has_row(&h, "because Robots help"));
    h.get_by_label(&t("outline.toggle.labels")).click();
    h.run();
    assert!(has_row(&h, "Robots help"));
    for key in [
        "outline.toggle.indent",
        "outline.toggle.colour",
        "outline.toggle.notes",
    ] {
        h.get_by_label(&t(key)).click();
        h.run();
    }
    let s = h.state().settings.clone();
    assert_eq!(
        s.outline,
        OutlineOptions {
            numbering: false,
            labels: false,
            notes: true
        }
    );
    assert!(!s.outline_indent && !s.outline_colour);

    // Settings survive a save and restore.
    let json = serde_json::to_string(&s).unwrap();
    let restored: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, s);
    // Settings saved before the outline existed get its defaults.
    let old: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(old.outline, OutlineOptions::default());
    assert!(old.outline_indent && old.outline_colour);

    // The action sets, rather than flips.
    h.state_mut()
        .pending
        .push(Action::SetOutlineOption(OutlineToggle::Notes, true));
    h.run();
    assert!(h.state().settings.outline.notes);
}

#[test]
fn copy_all_puts_plain_text_on_clipboard() {
    let mut doc = small_doc();
    let premise = box_with_text(&doc, "Robots help");
    doc.add_box(
        Parent::Box(premise),
        reasoncraft_core::BoxKind::Source {
            source_type: SourceType::Web,
        },
        "robots.example",
    )
    .unwrap();
    // The same text as the Outline (text) export, in the document language.
    let opts = OutlineOptions::default();
    let lang = doc.meta.language.clone();
    let want = outline_plain_text_lang(&build_outline(&doc, &opts), &opts, &lang, &|k| {
        t_lang(&lang, k)
    });
    assert!(want.contains("[Web] robots.example"), "{want}");

    let mut h = harness_with(doc);
    h.run();
    assert!(h.query_by_label(&t("outline.copied")).is_none());
    h.get_by_label(&t("outline.copy_all")).click();
    h.step();
    let copied: Vec<&String> = h
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(text) => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(copied, [&want]);
    h.run();
    h.get_by_label(&t("outline.copied"));
    // The notice survives later frames (its id is stable across frames)…
    let toolbar = h.get_by_label(&t("outline.copy_all")).rect().center();
    for dx in [40.0, 80.0] {
        h.event(egui::Event::PointerMoved(toolbar + egui::vec2(0.0, dx)));
        h.step();
        h.step();
        h.get_by_label(&t("outline.copied"));
    }
    // …and is gone once 2 s have passed.
    let now = h.ctx.input(|i| i.time);
    h.input_mut().time = Some(now + 1.5);
    h.run();
    h.get_by_label(&t("outline.copied"));
    h.input_mut().time = Some(now + 2.1);
    h.run();
    assert!(h.query_by_label(&t("outline.copied")).is_none());
}

/// Clicking another box's row while editing a box commits the edit as one
/// history step and moves the selection; no text goes to the clicked box.
#[test]
fn outline_click_while_editing_commits_edit() {
    let doc = small_doc();
    let a = box_with_text(&doc, "Robots help");
    let b = box_with_text(&doc, "Robots break");
    let mut h = harness_with(doc);
    h.run();
    for action in [
        Action::Select(Some(Item::Box(a))),
        Action::Edit,
        Action::EditText {
            node: a,
            text: "Robots help a lot".into(),
        },
    ] {
        h.state_mut().pending.push(action);
    }
    h.run();
    let before = h.state().tabs[0].history.undo.len();
    assert_eq!(h.state().tabs[0].editing(), Some(a));
    h.get_by_role_and_label(Role::ListItem, "2 but Robots break")
        .click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&a].text, "Robots help a lot");
    assert_eq!(tab.doc.boxes[&b].text, "Robots break");
    assert_eq!(tab.history.undo.len(), before + 1);
    assert_eq!(tab.selection, Some(Item::Box(b)));
    assert_eq!(tab.editing(), None);
    assert!(!tab.has_pending_session());
}

#[test]
fn outline_rows_are_virtualised() {
    let doc = sample_iot_like(500);
    let lines = build_outline(&doc, &OutlineOptions::default()).len();
    let mut h = harness_with(doc);
    h.run();
    let shown = row_count(&h);
    assert!(shown > 10 && shown < lines / 4, "{shown} of {lines} rows");
    assert!(has_row(&h, &claim_row(&h)));
    let at = over_rows(&h);
    wheel(&mut h, at, -2000.0);
    h.run();
    assert!(!has_row(&h, &claim_row(&h)), "scrolled");
    assert!(row_count(&h) < lines / 4);
}

/// The claim's row label: its text (no number, no label).
fn claim_row(h: &Harness<'static, ReasonCraftApp>) -> String {
    let tab = &h.state().tabs[0];
    tab.doc.boxes[&tab.doc.root].text.clone()
}

/// A point over the outline rows (below the claim's row, which must show).
fn over_rows(h: &Harness<'static, ReasonCraftApp>) -> egui::Pos2 {
    let label = claim_row(h);
    let row = h.get_by_role_and_label(Role::ListItem, &label).rect();
    row.center() + egui::vec2(0.0, 200.0)
}

/// Move the pointer to `at` and scroll the wheel by `dy` points there.
fn wheel(h: &mut Harness<'static, ReasonCraftApp>, at: egui::Pos2, dy: f32) {
    h.event(egui::Event::PointerMoved(at));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, dy),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    });
}

#[test]
#[ignore = "timing: run in release"]
fn outline_scrolls_large_map() {
    let mut h = harness_with(sample_iot_like(500));
    h.run();
    let at = over_rows(&h);
    let mut worst = Duration::ZERO;
    for _ in 0..3 {
        wheel(&mut h, at, -600.0);
        let start = Instant::now();
        h.step();
        worst = worst.max(start.elapsed());
    }
    h.run();
    assert!(!has_row(&h, &claim_row(&h)), "scrolled");
    eprintln!("worst frame while scrolling the outline: {worst:?}");
    assert!(worst < Duration::from_millis(50), "took {worst:?}");
}

#[test]
fn outline_refreshes_after_edit() {
    let mut h = harness_with(small_doc());
    h.run();
    assert!(has_row(&h, "Claim"));
    let root = h.state().tabs[0].doc.root;
    for action in [
        Action::Select(Some(Item::Box(root))),
        Action::Edit,
        Action::EditText {
            node: root,
            text: "Changed".into(),
        },
    ] {
        h.state_mut().pending.push(action);
    }
    h.run();
    // Live inline edits show at once.
    assert!(has_row(&h, "Changed"));
    assert!(!has_row(&h, "Claim"));
    h.state_mut().pending.push(Action::StopEditing);
    h.state_mut().pending.push(Action::Undo);
    h.run();
    assert!(has_row(&h, "Claim"));
    let tab = &h.state().tabs[0];
    let (rev, _, _) = tab.outline_cache.as_ref().expect("cached");
    assert_eq!(*rev, tab.revision());
}
