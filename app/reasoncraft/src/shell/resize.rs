//! Resizing the borderless window by its edges and corners.
//!
//! On Windows and Linux the window has no system frame, so the OS offers no
//! resize border. Thin invisible zones along the window edges take over:
//! they show the resize cursor and hand a primary press to the OS as a
//! window resize. macOS keeps its native frame, and a maximized or
//! full-screen window cannot be resized, so neither gets zones.

use eframe::egui::{
    self, Area, CursorIcon, Id, Order, Rect, ResizeDirection, Sense, ViewportCommand, vec2,
};

/// Thickness of the grab zone along each edge, in points.
const EDGE: f32 = 5.0;
/// Size of the square grab zone at each corner, in points.
const CORNER: f32 = 10.0;

/// The grab zones of a window whose content fills `screen`: the four
/// corners first, then the edges between them.
pub fn zones(screen: Rect) -> [(ResizeDirection, Rect); 8] {
    use ResizeDirection as D;
    let (min, max) = (screen.min, screen.max);
    let corner = |x: f32, y: f32| Rect::from_min_size(egui::pos2(x, y), vec2(CORNER, CORNER));
    let (left, right) = (min.x, max.x - CORNER);
    let (top, bottom) = (min.y, max.y - CORNER);
    let inner_x = (min.x + CORNER)..=(max.x - CORNER);
    let inner_y = (min.y + CORNER)..=(max.y - CORNER);
    let horizontal = |y: f32| Rect::from_x_y_ranges(inner_x.clone(), y..=y + EDGE);
    let vertical = |x: f32| Rect::from_x_y_ranges(x..=x + EDGE, inner_y.clone());
    [
        (D::NorthWest, corner(left, top)),
        (D::NorthEast, corner(right, top)),
        (D::SouthWest, corner(left, bottom)),
        (D::SouthEast, corner(right, bottom)),
        (D::North, horizontal(min.y)),
        (D::South, horizontal(max.y - EDGE)),
        (D::West, vertical(min.x)),
        (D::East, vertical(max.x - EDGE)),
    ]
}

fn cursor(dir: ResizeDirection) -> CursorIcon {
    use ResizeDirection as D;
    match dir {
        D::North => CursorIcon::ResizeNorth,
        D::South => CursorIcon::ResizeSouth,
        D::East => CursorIcon::ResizeEast,
        D::West => CursorIcon::ResizeWest,
        D::NorthEast => CursorIcon::ResizeNorthEast,
        D::NorthWest => CursorIcon::ResizeNorthWest,
        D::SouthEast => CursorIcon::ResizeSouthEast,
        D::SouthWest => CursorIcon::ResizeSouthWest,
    }
}

/// Show the grab zones on top of everything else (Windows and Linux,
/// while the window is neither maximized nor full screen).
pub fn show(ctx: &egui::Context) {
    if cfg!(target_os = "macos") {
        return;
    }
    let (fixed, pressed) = ctx.input(|i| {
        let vp = i.viewport();
        let fixed = vp.maximized.unwrap_or(false) || vp.fullscreen.unwrap_or(false);
        (fixed, i.pointer.primary_pressed())
    });
    if fixed {
        return;
    }
    for (i, (dir, rect)) in zones(ctx.content_rect()).into_iter().enumerate() {
        let response = Area::new(Id::unique(("window_resize", i)))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            // Mouse only, like an OS window frame: Tab skips the zones.
            .sense(Sense::DRAG)
            .show(ctx, |ui| ui.allocate_space(rect.size()))
            .response;
        if response.hovered() || response.dragged() {
            ctx.set_cursor_icon(cursor(dir));
        }
        if response.hovered() && pressed {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_hug_the_edges_without_overlap() {
        let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(800.0, 500.0));
        let z = zones(screen);
        let at = |x: f32, y: f32| {
            let hits: Vec<_> = z
                .iter()
                .filter(|(_, r)| r.contains(egui::pos2(x, y)))
                .map(|(d, _)| *d)
                .collect();
            assert!(hits.len() <= 1, "({x}, {y}) is in {hits:?}");
            hits.first().copied()
        };
        use ResizeDirection as D;
        assert_eq!(at(1.0, 1.0), Some(D::NorthWest));
        assert_eq!(at(799.0, 1.0), Some(D::NorthEast));
        assert_eq!(at(1.0, 499.0), Some(D::SouthWest));
        assert_eq!(at(799.0, 499.0), Some(D::SouthEast));
        assert_eq!(at(400.0, 2.0), Some(D::North));
        assert_eq!(at(400.0, 498.0), Some(D::South));
        assert_eq!(at(2.0, 250.0), Some(D::West));
        assert_eq!(at(798.0, 250.0), Some(D::East));
        // Just inside the edge zones: the window's own content.
        assert_eq!(at(400.0, EDGE + 1.0), None);
        assert_eq!(at(EDGE + 1.0, 250.0), None);
        assert_eq!(at(400.0, 250.0), None);
        for (_, r) in z {
            assert!(screen.contains_rect(r), "{r:?}");
        }
    }
}
