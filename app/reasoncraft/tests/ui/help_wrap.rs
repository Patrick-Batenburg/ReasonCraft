//! The help text keeps its wrap from frame to frame: moving the pointer
//! over the text, over the scroll bar's strip or off the panel, or any
//! other repaint, never re-wraps a paragraph.

use eframe::egui::{self, accesskit::Role, epaint::Shape};
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::help::{self, HelpPage};
use reasoncraft::shell::panes::Pane;

use crate::common::H;

/// One visual line of painted text: its text and where it starts.
#[derive(Clone, Debug, PartialEq)]
struct Line {
    text: String,
    /// Where its first glyph is painted.
    at: egui::Pos2,
    first: Option<char>,
}

/// Every visual line of text painted in the Text panel in the last frame.
fn panel_lines(h: &H) -> Vec<Line> {
    fn walk(s: &Shape, out: &mut Vec<Line>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Text(t) => {
                for row in &t.galley.rows {
                    let Some(glyph) = row.glyphs.first() else {
                        continue;
                    };
                    out.push(Line {
                        text: row.text(),
                        at: t.pos + row.pos.to_vec2() + glyph.pos.to_vec2(),
                        first: Some(glyph.chr),
                    });
                }
            }
            _ => {}
        }
    }
    let panel = h
        .state()
        .pane_rect(Pane::Text)
        .expect("the Text panel shows");
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out.retain(|l| l.at.x >= panel.left() && l.at.y >= panel.top());
    out
}

/// The app at `ppp` pixels per point, in a window `height` px high, with
/// the Text panel `width` px wide (dragged there by its edge) and the help
/// topic `topic` open.
fn harness(ppp: f32, width: f32, height: f32, topic: &'static str) -> H {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1360.0, height))
        .with_pixels_per_point(ppp)
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        );
    h.run();
    apply(h.state_mut(), Action::OpenHelp(HelpPage::Topic(topic)));
    h.run();
    let rect = h.state().pane_rect(Pane::Text).unwrap();
    let from = egui::pos2(rect.left() + 2.0, rect.center().y);
    let to = egui::pos2(rect.right() - width, from.y);
    h.event(egui::Event::PointerMoved(from));
    h.run();
    h.drag_at(from);
    h.run();
    for at in [egui::pos2(700.0, from.y), to] {
        h.event(egui::Event::PointerMoved(at));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    h
}

/// Scroll the help page until its example paragraph is in the middle of
/// the panel (in a short window the page then has a scroll bar); returns
/// how many steps it scrolled.
fn scroll_to_example(h: &mut H) -> usize {
    let panel = h.state().pane_rect(Pane::Text).unwrap();
    for step in 0..40 {
        let Some(line) = panel_lines(h)
            .into_iter()
            .find(|l| l.text.starts_with("An example."))
        else {
            return step;
        };
        if line.at.y < panel.center().y {
            return step;
        }
        h.event(egui::Event::PointerMoved(panel.center()));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -60.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::default(),
        });
        h.run();
    }
    40
}

/// Where the pointer goes: over the text, over the scroll bar's strip at
/// the panel's right edge, just inside the panel's left edge, and off the
/// panel over the map, back and forth.
fn pointer_path(panel: egui::Rect) -> Vec<egui::Pos2> {
    let y = |i: usize| panel.top() + 120.0 + (37.0 * i as f32) % (panel.height() - 200.0);
    let spots = [
        |p: egui::Rect| p.center().x,
        |p: egui::Rect| p.right() - 2.0,
        |p: egui::Rect| p.right() - 7.0,
        |p: egui::Rect| p.left() + 20.0,
        |_: egui::Rect| 600.0,
    ];
    (0..20)
        .map(|i| egui::pos2(spots[i % spots.len()](panel), y(i)))
        .collect()
}

/// At the owner's scale (147 %) and panel widths around the one shown, and
/// at other scales, with the page scrolled (a scroll bar) or not, every
/// frame paints the panel's text in the same lines at the same places, and
/// the panel keeps its width.
#[test]
fn help_text_keeps_its_wrap_across_frames() {
    for (ppp, height) in [(1.0, 2400.0), (1.25, 2400.0), (1.47, 2400.0), (1.47, 760.0)] {
        for width in [230.0, 252.0, 269.0, 284.0, 300.0, 320.0] {
            let mut h = harness(ppp, width, height, "checks");
            let scrolled = scroll_to_example(&mut h);
            assert_eq!(scrolled > 0, height < 1000.0, "{ppp} {width}: scrolled");
            let panel = h.state().pane_rect(Pane::Text).unwrap();
            let first = panel_lines(&h);
            assert!(
                first.iter().any(|l| l.text.starts_with("An example.")),
                "{ppp} {width}: the example paragraph shows"
            );
            for (frame, at) in pointer_path(panel).into_iter().enumerate() {
                h.event(egui::Event::PointerMoved(at));
                h.step();
                let now = h.state().pane_rect(Pane::Text).unwrap();
                assert_eq!(
                    now, panel,
                    "ppp {ppp}, width {width}, frame {frame}: the panel keeps its size"
                );
                let lines = panel_lines(&h);
                assert!(
                    lines == first,
                    "ppp {ppp}, width {width}, frame {frame} (pointer at {at:?}): \
                     the text re-wrapped\nwas: {first:#?}\nnow: {lines:#?}"
                );
            }
        }
    }
}

/// No visual line of a help paragraph starts with a space: the words of a
/// paragraph (bold ones too) wrap as one text.
#[test]
fn no_help_line_starts_with_a_space() {
    for ppp in [1.0, 1.47] {
        for width in [230.0, 252.0, 269.0, 284.0, 300.0, 320.0] {
            let h = harness(ppp, width, 2400.0, "checks");
            let lines = panel_lines(&h);
            let example = lines
                .iter()
                .position(|l| l.text.starts_with("An example."))
                .expect("the example paragraph shows");
            let left = lines[example].at.x;
            for line in &lines[example..] {
                if (line.at.x - left).abs() < 1.0 {
                    assert_ne!(
                        line.first,
                        Some(' '),
                        "ppp {ppp}, width {width}: a line starts with a space: {:?}",
                        line.text
                    );
                }
            }
        }
    }
}

/// A link inside a paragraph is still a named link that opens its topic.
#[test]
fn paragraph_links_stay_named_and_clickable() {
    let mut h = harness(1.47, 269.0, 760.0, "checks");
    assert!(scroll_to_example(&mut h) > 0);
    let title = help::title("en", "co-premises");
    h.get_by_role_and_label(Role::Link, &title).click();
    h.run();
    assert_eq!(
        h.state().help.page,
        HelpPage::Topic(help::topic("co-premises").unwrap())
    );
}
