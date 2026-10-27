//! Tab titles and side panel widths.

mod common;

use std::path::PathBuf;

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::shell::panes::Pane;
use reasoncraft::shell::tabs::short_title;

type H = Harness<'static, ReasonCraftApp>;

fn harness() -> H {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        )
}

/// The text of every text shape painted in the last frame.
fn painted_texts(h: &H) -> Vec<String> {
    common::painted_texts(h)
        .into_iter()
        .map(|(t, _)| t)
        .collect()
}

#[test]
fn long_tab_title_truncates_with_tooltip() {
    assert_eq!(short_title("Short name"), "Short name");
    assert_eq!(short_title(&"x".repeat(24)), "x".repeat(24));
    let name = "Why cities should ban cars from their centres";
    let short = short_title(name);
    assert_eq!(short, "Why cities should ban ca…");
    assert_eq!(short_title(&format!("{name} •")), format!("{short} •"));

    let mut h = harness();
    h.state_mut().tabs[0].path = Some(PathBuf::from(format!("{name}.rcmap")));
    h.run();
    let whole = |h: &H| painted_texts(h).iter().filter(|t| *t == name).count();
    assert!(
        painted_texts(&h).contains(&short),
        "the chip shows {short:?}"
    );
    // Only the title bar shows the whole name.
    let before = whole(&h);
    // The tab keeps its full name for screen readers, and shows it on hover.
    let tab = h.get_by_role_and_label(egui::accesskit::Role::Tab, name);
    let at = tab.rect().center();
    h.event(egui::Event::PointerMoved(at));
    for _ in 0..40 {
        h.step();
    }
    assert_eq!(whole(&h), before + 1, "the tooltip shows the whole title");
}

/// Drag the panel's resize handle from `from` to `x`, by way of the
/// middle of the window (a drag starts after a few px of travel).
fn drag_handle(h: &mut H, from: egui::Pos2, x: f32) {
    let to = egui::pos2(x, from.y);
    h.event(egui::Event::PointerMoved(from));
    h.run();
    h.drag_at(from);
    h.run();
    for at in [egui::pos2(700.0, from.y), to] {
        h.event(egui::Event::PointerMoved(at));
        h.run();
    }
    // Released where the pointer stays, so the panel keeps the size.
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
}

#[test]
fn side_panel_min_width() {
    for pane in [Pane::Building, Pane::Text] {
        let mut h = harness();
        h.run();
        let rect = h.state().pane_rect(pane).expect("the panel shows");
        let (edge, inward) = match pane {
            Pane::Building => (rect.right(), 1.0),
            _ => (rect.left(), -1.0),
        };
        let handle = egui::pos2(edge - inward * 2.0, rect.center().y);
        // Just above the minimum: it follows the pointer.
        let fixed = edge - inward * rect.width();
        drag_handle(&mut h, handle, fixed + inward * 225.0);
        let rect = h.state().pane_rect(pane).expect("still open");
        assert!(
            (rect.width() - 225.0).abs() < 2.0,
            "{pane:?}: {}",
            rect.width()
        );
        // Below the minimum, it collapses to its toggle.
        let handle = egui::pos2(fixed + inward * (rect.width() - 2.0), rect.center().y);
        drag_handle(&mut h, handle, fixed + inward * 150.0);
        let open = match pane {
            Pane::Building => h.state().left_panel,
            _ => h.state().right_panel,
        };
        assert!(!open, "{pane:?} collapses below 220 px");
    }
}

/// Where the title bar shows `title`: the rect of the text shape for it
/// in the bar's height.
fn title_rect(h: &H, title: &str) -> egui::Rect {
    fn walk(s: &egui::epaint::Shape, title: &str, out: &mut Vec<egui::Rect>) {
        match s {
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, title, out)),
            egui::epaint::Shape::Text(t) if t.galley.text() == title => {
                out.push(t.galley.rect.translate(t.pos.to_vec2()));
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, title, &mut found);
    }
    found.retain(|r| r.top() < 36.0);
    let [rect] = found.as_slice() else {
        panic!("one title in the bar: {found:?}")
    };
    *rect
}

/// The document title sits centred in the title bar when there is room,
/// and otherwise stays between the quick buttons and the command search,
/// shortened, instead of running under the search box.
#[test]
fn window_title_stays_clear_of_the_search_box() {
    use reasoncraft::i18n::{FluentArgs, t, t_args};
    let name = "Why cities should ban cars from their centres and more";
    let mut args = FluentArgs::new();
    args.set("keys", "Alt+Q");
    let search = t_args("titlebar.search", &args);
    for width in [1400.0, 820.0] {
        let mut h = Harness::builder()
            .with_size(egui::vec2(width, 600.0))
            .with_max_steps(32)
            .build_ui_state(
                |ui, app: &mut ReasonCraftApp| app.show(ui),
                ReasonCraftApp::new_for_test(),
            );
        h.state_mut().tabs[0].path = Some(PathBuf::from(format!("{name}.rcraft")));
        h.run();
        let title = title_rect(&h, name);
        let search = h.get_by_label(&search).rect();
        // The quick Redo, in the bar (the Home tab has one too).
        let redo = h
            .query_all_by_label(&t("action.redo"))
            .map(|n| n.rect())
            .find(|r| r.top() < 36.0)
            .expect("the quick Redo");
        assert!(
            title.right() <= search.left(),
            "{width}: {title:?} {search:?}"
        );
        assert!(title.left() >= redo.right(), "{width}: {title:?} {redo:?}");
        if width > 1000.0 {
            assert!((title.center().x - width / 2.0).abs() < 1.0, "centred");
        }
    }
}
