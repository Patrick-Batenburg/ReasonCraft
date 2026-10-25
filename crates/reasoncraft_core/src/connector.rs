//! Connector geometry and stroke rules.

use emath::Pos2;

use crate::layout::ConnectorKind;

/// Arrowhead size in screen px, independent of stroke width and zoom.
pub const ARROW_PX: f32 = 9.0;
/// Length of the straight stub every connector ends in, in screen px.
pub const STUB_PX: f32 = 16.0;

/// One piece of a connector path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Cubic([Pos2; 4]),
    Line([Pos2; 2]),
}

/// What is drawn at the parent end of a connector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndMarker {
    Arrow,
    Bar,
    None,
}

/// Evaluated strength of a connector (only `Normal` is used in step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Strong,
    Normal,
    Weak,
}

/// Path from the child top `from` to the parent port `to`: an S-curve that
/// ends `stub` px below `to`, followed by a straight vertical stub up to `to`.
pub fn curved_path(from: Pos2, to: Pos2, stub: f32) -> Vec<Segment> {
    let curve_end = Pos2::new(to.x, to.y + stub);
    let mid = (from.y + curve_end.y) / 2.0;
    vec![
        Segment::Cubic([
            from,
            Pos2::new(from.x, mid),
            Pos2::new(to.x, mid),
            curve_end,
        ]),
        Segment::Line([curve_end, to]),
    ]
}

pub fn end_marker(kind: ConnectorKind) -> EndMarker {
    match kind {
        ConnectorKind::Supports => EndMarker::Arrow,
        ConnectorKind::Opposes | ConnectorKind::Rebuts => EndMarker::Bar,
        ConnectorKind::Neutral | ConnectorKind::Attachment => EndMarker::None,
    }
}

/// On-screen stroke width in px at the given zoom (1.0 = 100%).
pub fn stroke_px(kind: ConnectorKind, strength: Strength, zoom: f32) -> f32 {
    let (base, min) = match kind {
        ConnectorKind::Neutral => (1.5, 1.25),
        ConnectorKind::Attachment => (1.0, 1.0),
        _ => (
            match strength {
                Strength::Strong => 3.0,
                Strength::Normal => 2.0,
                Strength::Weak => 1.4,
            },
            1.25,
        ),
    };
    (base * zoom).max(min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curved_path_ends_with_vertical_stub() {
        let to = Pos2::new(100.0, 50.0);
        let path = curved_path(Pos2::new(40.0, 200.0), to, STUB_PX);
        assert_eq!(
            path.last(),
            Some(&Segment::Line([Pos2::new(100.0, 66.0), to]))
        );
    }

    #[test]
    fn stroke_never_below_minimum() {
        assert!(stroke_px(ConnectorKind::Supports, Strength::Weak, 0.1) >= 1.25);
        assert!(stroke_px(ConnectorKind::Attachment, Strength::Normal, 0.1) >= 1.0);
    }

    #[test]
    fn strong_thicker_than_weak() {
        assert!(
            stroke_px(ConnectorKind::Supports, Strength::Strong, 1.0)
                > stroke_px(ConnectorKind::Supports, Strength::Weak, 1.0)
        );
    }

    #[test]
    fn markers_by_kind() {
        assert_eq!(end_marker(ConnectorKind::Supports), EndMarker::Arrow);
        assert_eq!(end_marker(ConnectorKind::Opposes), EndMarker::Bar);
        assert_eq!(end_marker(ConnectorKind::Rebuts), EndMarker::Bar);
        assert_eq!(end_marker(ConnectorKind::Neutral), EndMarker::None);
        assert_eq!(end_marker(ConnectorKind::Attachment), EndMarker::None);
    }

    #[test]
    fn path_valid_when_child_not_below_parent() {
        let path = curved_path(Pos2::new(0.0, 10.0), Pos2::new(5.0, 20.0), STUB_PX);
        assert_eq!(path.len(), 2);
    }
}
