//! The find bar in a narrow window: with both side panels open,
//! every control of the bar stays inside the map area, and a real mouse
//! click on each one does what it says (before, the buttons on the right
//! sat under the Text panel and a click landed there).

use crate::common;

use eframe::egui::{self, Modifiers};
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent};

use crate::common::H;

/// The find bar with replace open, at `width`×700, finding "cat" in a
/// map with "cat one", "dog" and "black cat"; the boxes with a hit.
fn bar_at(width: f32) -> (H, [NodeId; 2]) {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let (_, one) = d.add_card(root, CardKind::Reason, "cat one").unwrap();
    d.add_card(root, CardKind::Reason, "dog").unwrap();
    let (_, two) = d.add_card(root, CardKind::Objection, "black cat").unwrap();
    let app = common::app_with_doc(d);
    let mut h = common::builder()
        .with_size(egui::vec2(width, 700.0))
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    assert!(h.state().left_panel && h.state().right_panel, "both panels");
    apply(h.state_mut(), Action::OpenFind { replace: true });
    apply(h.state_mut(), Action::SetFindQuery("cat".into()));
    apply(h.state_mut(), Action::SetReplaceText("cow".into()));
    h.run();
    (h, [one, two])
}

fn click_at(h: &mut H, pos: egui::Pos2) {
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run();
}

/// Clicks the control named `key` with the mouse, after checking that it
/// lies inside the map area's columns (between the side panels).
fn click(h: &mut H, key: &str, width: f32) {
    let label = t(key);
    let rect = h.get_by_label(&label).rect();
    let map = h.state().tabs[0].viewport.expect("the map is shown");
    assert!(
        map.min.x - 0.5 <= rect.min.x && rect.max.x <= map.max.x + 0.5,
        "{width}: {label:?} at {rect:?} is outside the map {map:?}"
    );
    click_at(h, rect.center());
}

fn hit(h: &H) -> Option<Item> {
    h.state().tabs[0].selection
}

#[test]
fn every_find_bar_control_is_clickable_in_a_narrow_window() {
    for width in [800.0, 1024.0] {
        let (mut h, [one, two]) = bar_at(width);
        for key in ["find.query", "find.replacement"] {
            let rect = h.get_by_label(&t(key)).rect();
            let map = h.state().tabs[0].viewport.unwrap();
            assert!(
                map.min.x - 0.5 <= rect.min.x && rect.max.x <= map.max.x + 0.5,
                "{width}: the field {key} at {rect:?} is outside the map {map:?}"
            );
        }
        for key in ["find.match_case.tip", "find.whole_word.tip"] {
            click(&mut h, key, width);
            let opts = h.state().tabs[0].find.opts;
            assert!(opts.match_case || opts.whole_word, "{width}: {key} on");
            click(&mut h, key, width);
        }
        let opts = h.state().tabs[0].find.opts;
        assert!(!opts.match_case && !opts.whole_word, "{width}: both off");

        assert_eq!(hit(&h), Some(Item::Box(one)), "{width}: first hit");
        click(&mut h, "find.next", width);
        assert_eq!(hit(&h), Some(Item::Box(two)), "{width}: Next");
        click(&mut h, "find.prev", width);
        assert_eq!(hit(&h), Some(Item::Box(one)), "{width}: Previous");

        click(&mut h, "find.replace", width);
        let boxes = &h.state().tabs[0].doc.boxes;
        assert_eq!(boxes[&one].text, "cow one", "{width}: Replace");
        assert_eq!(boxes[&two].text, "black cat", "{width}: only one");
        click(&mut h, "find.replace_all", width);
        let boxes = &h.state().tabs[0].doc.boxes;
        assert_eq!(boxes[&two].text, "black cow", "{width}: Replace all");

        click(&mut h, "find.close", width);
        assert!(!h.state().tabs[0].find.open, "{width}: Close find");
    }
}
