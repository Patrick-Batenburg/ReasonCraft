//! Comments from the Home ribbon: popover, verdicts, long text.

use super::*;

/// A claim, selected, with its comment popover opened from the Home ribbon.
fn comments_harness(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    let root = doc.root;
    let mut h = harness_with(doc);
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(reasoncraft_core::Item::Box(root));
    h.run();
    h.get_by_label(&t("action.comment")).click();
    h.run();
    assert_eq!(h.state().tabs[0].comments_open, Some(root));
    h
}

#[test]
fn preset_adds_comment_and_badge_shows_count() {
    use reasoncraft_core::{Document, Verdict};
    let mut h = comments_harness(Document::new("Claim"));
    let root = h.state().tabs[0].doc.root;
    let add_preset = |h: &mut Harness<'static, ReasonCraftApp>, key: &str| {
        h.get_by_label(&t("comment.presets")).click();
        h.run();
        h.get_by_label(&t(key)).click();
        h.run();
    };
    add_preset(&mut h, "comment.preset.great_work");
    {
        let tab = &h.state().tabs[0];
        let comments = &tab.doc.boxes[&root].comments;
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, t("comment.preset.great_work"));
        assert_eq!(comments[0].verdict, Some(Verdict::Positive));
        assert_eq!(comments[0].author, t("comment.anonymous"));
        assert_eq!(tab.history.undo.len(), 1);
        // Picking from the menu keeps the popover open.
        assert_eq!(tab.comments_open, Some(root));
    }
    assert!(h.query_by_label("1 comment").is_some());
    add_preset(&mut h, "comment.preset.unclear");
    assert_eq!(
        h.state().tabs[0].doc.boxes[&root].comments[1].verdict,
        Some(Verdict::Problem)
    );
    assert!(h.query_by_label("2 comments").is_some());

    // Esc closes the popover; the badge opens it again.
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(h.state().tabs[0].comments_open, None);
    h.state_mut().tabs[0].selection = None;
    h.run();
    h.get_by_label("2 comments").click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.comments_open, Some(root));
    assert_eq!(tab.selection, Some(reasoncraft_core::Item::Box(root)));

    // A press on the empty canvas closes it.
    let corner = tab.viewport.unwrap().min + egui::vec2(4.0, 4.0);
    click_at(&mut h, corner);
    assert_eq!(h.state().tabs[0].comments_open, None);
}

#[test]
fn long_comment_wraps_in_popover() {
    use reasoncraft::canvas::comments::{POPOVER_WIDTH, popover_id};
    use reasoncraft_core::Document;
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let long: String = "Every claim deserves a reason and a source. "
        .repeat(14)
        .chars()
        .take(600)
        .collect();
    assert_eq!(long.chars().count(), 600);
    doc.add_comment(root, "Ann", &long, None).unwrap();
    doc.add_comment(root, "Bob", &"x".repeat(600), None)
        .unwrap();
    let mut h = comments_harness(doc);
    h.run();
    let area = egui::AreaState::load(&h.ctx, popover_id())
        .expect("popover shown")
        .rect();
    assert!(
        (area.width() - POPOVER_WIDTH).abs() < 1.0,
        "popover width {}",
        area.width()
    );
    for text in [long.as_str(), &"x".repeat(600)] {
        let label = h.get_by_label(text).rect();
        assert!(label.left() >= area.left() && label.right() <= area.right());
        assert!(label.width() <= POPOVER_WIDTH - 20.0, "{}", label.width());
    }
    let first = h.get_by_label(long.as_str()).rect();
    assert!(first.height() > 60.0, "wrapped over many rows: {first:?}");
}

#[test]
fn typed_comment_takes_the_picked_verdict_and_blank_is_refused() {
    use reasoncraft_core::{Document, Verdict};
    let mut h = comments_harness(Document::new("Claim"));
    let root = h.state().tabs[0].doc.root;
    let add = t("comment.add");
    let disabled =
        |h: &Harness<'static, ReasonCraftApp>| h.get_by_label(&add).accesskit_node().is_disabled();
    assert!(disabled(&h));
    let input = || egui::accesskit::Role::MultilineTextInput;
    h.get_by_role(input()).focus();
    h.run();
    h.get_by_role(input()).type_text("   ");
    h.run();
    assert!(disabled(&h), "blank text cannot be added");
    h.get_by_label(&t("comment.verdict.needs_work")).click();
    h.run();
    h.get_by_role(input()).focus();
    h.run();
    h.get_by_role(input()).type_text("Say who found this.");
    h.run();
    assert!(!disabled(&h));
    h.get_by_label(&add).click();
    h.run();
    let tab = &h.state().tabs[0];
    let c = &tab.doc.boxes[&root].comments;
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].text, "Say who found this.");
    assert_eq!(c[0].verdict, Some(Verdict::NeedsWork));
    assert_eq!(tab.history.undo.len(), 1);
    // The input is cleared; delete removes the comment again.
    assert!(disabled(&h));
    h.get_by_label(&t("comment.delete")).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert!(tab.doc.boxes[&root].comments.is_empty());
    assert_eq!(tab.history.undo.len(), 2);
}

#[test]
fn long_paste_is_cut_with_notice() {
    let doc = reasoncraft_core::Document::new("");
    let root = doc.root;
    let mut h = harness_with(doc);
    h.state_mut().tabs[0].begin_edit(root, false);
    h.run();
    h.event(egui::Event::Paste("word ".repeat(12_000)));
    h.run();
    let text = &h.state().tabs[0].doc.boxes[&root].text;
    assert_eq!(text.chars().count(), 50_000);
    assert_eq!(h.state().status_note, Some(t("status.text_cut")));
}
