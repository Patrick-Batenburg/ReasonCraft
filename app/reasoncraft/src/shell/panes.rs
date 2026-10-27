//! F6 / Shift+F6: move the keyboard focus to the next or previous pane
//! of the window, from any control: ribbon, Building panel, map, Text
//! panel, status bar, and round again; with the File tab open, ribbon,
//! backstage, status bar. Hidden panels are skipped. The focus lands on
//! the pane's first control (the map itself for the map).

use eframe::egui::{self, Key, LayerId, Modifiers, Rect};

use crate::actions::Action;
use crate::canvas::canvas_id;

/// A part of the window F6 moves between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Ribbon,
    Building,
    Map,
    /// The File tab's page, shown instead of the panels and the map.
    Backstage,
    Text,
    Status,
}

/// The order F6 goes in.
const ORDER: [Pane; 6] = [
    Pane::Ribbon,
    Pane::Building,
    Pane::Map,
    Pane::Backstage,
    Pane::Text,
    Pane::Status,
];

/// Where each pane is shown this frame, and a move asked for.
#[derive(Default)]
pub struct Panes {
    rects: Vec<(Pane, Rect)>,
    /// `Some(true)` for F6, `Some(false)` for Shift+F6, done on the next
    /// frame once every pane has been drawn.
    pub(crate) request: Option<bool>,
}

impl Panes {
    /// Starts a frame: no pane drawn yet.
    pub fn start_frame(&mut self) {
        self.rects.clear();
    }

    /// Notes that `pane` fills `rect` this frame.
    pub fn shown(&mut self, pane: Pane, rect: Rect) {
        self.rects.push((pane, rect));
    }

    /// Where `pane` was shown this frame (so far), if it was.
    pub(crate) fn rect(&self, pane: Pane) -> Option<Rect> {
        self.rects.iter().find(|(p, _)| *p == pane).map(|(_, r)| *r)
    }

    /// Does the move asked for, if any: call once every pane is drawn.
    pub fn finish_frame(&mut self, ctx: &egui::Context) {
        let Some(forward) = self.request.take() else {
            return;
        };
        let shown: Vec<(Pane, Rect)> = ORDER
            .iter()
            .filter_map(|p| self.rects.iter().find(|(q, _)| q == p).copied())
            .collect();
        let n = shown.len();
        if n == 0 {
            return;
        }
        // The pane holding the focus; with none (or the find bar above the
        // map), the map (or the backstage) has the keys.
        let at = ctx
            .memory(|m| m.focused())
            .and_then(|id| ctx.read_response(id))
            .map(|r| r.rect.center());
        let mut i = at
            .and_then(|pos| shown.iter().position(|(_, r)| r.contains(pos)))
            .or_else(|| {
                let main = |p: &Pane| matches!(p, Pane::Map | Pane::Backstage);
                shown.iter().position(|(p, _)| main(p))
            })
            .unwrap_or(n - 1);
        for _ in 0..n {
            i = if forward {
                (i + 1) % n
            } else {
                (i + n - 1) % n
            };
            if let Some(id) = first_control(ctx, shown[i]) {
                ctx.memory_mut(|m| m.request_focus(id));
                ctx.request_repaint();
                return;
            }
        }
    }
}

/// The first enabled control Tab could reach in `pane`, in drawing order.
fn first_control(ctx: &egui::Context, (pane, rect): (Pane, Rect)) -> Option<egui::Id> {
    if pane == Pane::Map {
        return Some(canvas_id());
    }
    ctx.viewport(|v| {
        v.this_pass
            .widgets
            .get_layer(LayerId::background())
            .find(|w| w.sense.is_focusable() && w.enabled && rect.contains(w.rect.center()))
            .map(|w| w.id)
    })
}

/// F6 / Shift+F6 (next / previous pane) taken out of the input, for the
/// backstage, which has no other keys outside its fields.
pub fn pane_keys(ctx: &egui::Context) -> Vec<Action> {
    let mut out = Vec::new();
    ctx.input_mut(|i| {
        for (mods, action) in [
            (Modifiers::SHIFT, Action::PrevPane),
            (Modifiers::NONE, Action::NextPane),
        ] {
            if i.consume_key(mods, Key::F6) {
                out.push(action);
            }
        }
    });
    out
}
