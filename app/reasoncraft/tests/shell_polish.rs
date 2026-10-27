//! Tab titles and side panel widths.

use std::path::PathBuf;

use eframe::egui::{self, epaint};
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
    fn flatten(s: &epaint::Shape, out: &mut Vec<String>) {
        match s {
            epaint::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            epaint::Shape::Text(t) => out.push(t.galley.text().to_owned()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        flatten(&clipped.shape, &mut out);
    }
    out
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
        let rect = h.state().panes.rect(pane).expect("the panel shows");
        let (edge, inward) = match pane {
            Pane::Building => (rect.right(), 1.0),
            _ => (rect.left(), -1.0),
        };
        let handle = egui::pos2(edge - inward * 2.0, rect.center().y);
        // Just above the minimum: it follows the pointer.
        let fixed = edge - inward * rect.width();
        drag_handle(&mut h, handle, fixed + inward * 225.0);
        let rect = h.state().panes.rect(pane).expect("still open");
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
