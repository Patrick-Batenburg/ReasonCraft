//! Scrolling the map while something is dragged near its edge, so a drop
//! target out of view can be reached.

use eframe::egui::{self, DragAndDrop, Pos2, Rect, Response, Vec2};

use super::DragState;
use crate::actions::Insertable;
use crate::canvas::Camera;

/// How near an edge of the map, in screen px, the pointer scrolls it.
pub const EDGE_PX: f32 = 32.0;
/// How fast the map scrolls, in screen px per second.
pub const SPEED_PX: f32 = 600.0;

/// The camera pan for one frame of `dt` seconds with the pointer at
/// `pointer` over the map `view`: [`SPEED_PX`] per second away from each
/// edge within [`EDGE_PX`], so what lies beyond that edge comes into view.
pub fn autoscroll_pan(view: Rect, pointer: Pos2, dt: f32) -> Vec2 {
    if !view.contains(pointer) {
        return Vec2::ZERO;
    }
    let axis = |lo: f32, hi: f32, at: f32| {
        if at < lo + EDGE_PX {
            1.0
        } else if at > hi - EDGE_PX {
            -1.0
        } else {
            0.0
        }
    };
    let x = axis(view.left(), view.right(), pointer.x);
    let y = axis(view.top(), view.bottom(), pointer.y);
    Vec2::new(x, y) * SPEED_PX * dt
}

/// Scroll the map while an item of it or of a panel is dragged near its
/// edge. A dragged item's ghost stays under the pointer.
pub(in crate::canvas) fn autoscroll(ui: &egui::Ui, response: &Response, cam: &mut Camera) {
    let id = response.id.with("drag");
    let mut state: Option<DragState> = ui.data(|d| d.get_temp(id));
    let item_drag = state.is_some_and(|s| s.active);
    let panel_drag = DragAndDrop::has_payload_of_type::<Insertable>(ui.ctx());
    // A pass that will be discarded runs again this frame: pan once.
    if !(item_drag || panel_drag) || ui.ctx().will_discard() {
        return;
    }
    let (pointer, dt) = ui.input(|i| (i.pointer.latest_pos(), i.stable_dt.min(0.1)));
    let Some(pointer) = pointer else {
        return;
    };
    let pan = autoscroll_pan(response.rect, pointer, dt);
    if pan == Vec2::ZERO {
        return;
    }
    cam.offset += pan;
    if let Some(s) = state.as_mut().filter(|s| s.active) {
        s.origin += pan;
        ui.data_mut(|d| d.insert_temp(id, *s));
    }
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    #[test]
    fn pans_toward_the_near_edges_only() {
        let view = Rect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 600.0));
        let dt = 0.5;
        let step = SPEED_PX * dt;
        assert_eq!(autoscroll_pan(view, view.center(), dt), Vec2::ZERO);
        assert_eq!(
            autoscroll_pan(view, pos2(110.0, 300.0), dt),
            vec2(step, 0.0)
        );
        assert_eq!(
            autoscroll_pan(view, pos2(890.0, 640.0), dt),
            vec2(-step, -step)
        );
        let inside = view.left() + EDGE_PX + 1.0;
        assert_eq!(autoscroll_pan(view, pos2(inside, 300.0), dt), Vec2::ZERO);
        assert_eq!(autoscroll_pan(view, pos2(95.0, 300.0), dt), Vec2::ZERO);
    }
}
