//! Connector geometry and stroke rules (spec §5): the three styles a user
//! can pick ([`ConnectorStyle`]), their paths, and the widths every style
//! keeps at any zoom.

use serde::{Deserialize, Serialize};

use crate::layout::ConnectorKind;

mod curved;
mod elbow;
mod stagger;
mod tapered;

pub use curved::curved_path;
pub use elbow::{ELBOW_RADIUS, elbow_path, elbow_path_at};
pub use stagger::{ELBOW_SPACING, elbow_turns};
pub use tapered::{tapered_band, tapered_widths};

/// Arrowhead size in screen px, independent of stroke width and zoom.
pub const ARROW_PX: f32 = 9.0;
/// Length of the straight stub every connector ends in, in screen px.
pub const STUB_PX: f32 = 16.0;
/// The thinnest a connector is ever drawn, in screen px, at every zoom
/// and in every style.
pub const MIN_VISIBLE_PX: f32 = 1.5;

/// How connectors are drawn: a personal preference, also used by exports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorStyle {
    /// An S-curve.
    #[default]
    Curved,
    /// Straight segments along the axes with rounded corners.
    Elbow,
    /// A filled band that widens toward the parent.
    Tapered,
}

impl ConnectorStyle {
    /// Every style, in menu order.
    pub const ALL: [Self; 3] = [Self::Curved, Self::Elbow, Self::Tapered];

    /// The style's name in command ids and settings.
    pub fn key(self) -> &'static str {
        match self {
            Self::Curved => "curved",
            Self::Elbow => "elbow",
            Self::Tapered => "tapered",
        }
    }
}

/// One piece of a connector path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Cubic([emath::Pos2; 4]),
    Line([emath::Pos2; 2]),
}

/// What is drawn at the parent end of a connector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndMarker {
    Arrow,
    Bar,
    None,
}

/// The weight a connector is drawn with, from the evaluation of what it
/// connects (`Normal` when unrated or with evaluation shading off).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Strong,
    Normal,
    Weak,
}

pub fn end_marker(kind: ConnectorKind) -> EndMarker {
    match kind {
        ConnectorKind::Supports => EndMarker::Arrow,
        ConnectorKind::Opposes | ConnectorKind::Rebuts => EndMarker::Bar,
        ConnectorKind::Neutral | ConnectorKind::Attachment => EndMarker::None,
    }
}

/// The design stroke width on screen, in px, at `zoom` (1.0 = 100 %): the
/// width at 100 % scaled by the zoom, with no floor. At 100 %: strong 3,
/// normal 2, weak 1.4, neutral 1.5, attachment 1. Drawing goes through
/// [`line_width`], which keeps every line at least [`MIN_VISIBLE_PX`], so
/// weak is drawn at 1.5 px even at 100 %, normal and weak look the same
/// below 75 % zoom, and every weight looks the same below 50 %.
pub fn stroke_px(kind: ConnectorKind, strength: Strength, zoom: f32) -> f32 {
    let base = match kind {
        ConnectorKind::Neutral => 1.5,
        ConnectorKind::Attachment => 1.0,
        _ => match strength {
            Strength::Strong => 3.0,
            Strength::Normal => 2.0,
            Strength::Weak => 1.4,
        },
    };
    base * zoom
}

/// The width a connector line is drawn at, in world units at `zoom`: its
/// [`stroke_px`], but never thinner than [`MIN_VISIBLE_PX`] on screen.
pub fn line_width(kind: ConnectorKind, strength: Strength, zoom: f32) -> f32 {
    stroke_px(kind, strength, zoom).max(MIN_VISIBLE_PX) / zoom
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `stroke_px` is the design width scaled by the zoom; only
    /// `line_width` applies the visible floor.
    #[test]
    fn stroke_px_is_the_design_width() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(close(
            stroke_px(ConnectorKind::Supports, Strength::Weak, 0.1),
            0.14
        ));
        assert!(close(
            stroke_px(ConnectorKind::Attachment, Strength::Normal, 0.1),
            0.1
        ));
        assert!(close(
            stroke_px(ConnectorKind::Neutral, Strength::Strong, 2.0),
            3.0
        ));
    }

    /// On screen: weak (1.4 px) is drawn at the 1.5 px floor even at 100 %;
    /// below 75 % normal reaches the floor too, and below 50 % strong does,
    /// so every weight looks the same.
    #[test]
    fn weights_merge_at_the_floor_when_zoomed_out() {
        let px = |s, zoom| line_width(ConnectorKind::Supports, s, zoom) * zoom;
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(close(px(Strength::Weak, 1.0), MIN_VISIBLE_PX));
        assert!(close(px(Strength::Normal, 0.7), px(Strength::Weak, 0.7)));
        assert!(px(Strength::Strong, 0.7) > px(Strength::Normal, 0.7));
        assert!(close(px(Strength::Strong, 0.4), MIN_VISIBLE_PX));
    }

    #[test]
    fn line_width_never_below_visible_floor() {
        for kind in [ConnectorKind::Attachment, ConnectorKind::Supports] {
            for zoom in [0.05, 0.5, 1.0, 4.0] {
                let w = line_width(kind, Strength::Weak, zoom) * zoom;
                assert!(w >= MIN_VISIBLE_PX - 1e-4, "{kind:?} at {zoom}: {w}");
            }
        }
        let normal = line_width(ConnectorKind::Supports, Strength::Normal, 1.0);
        assert_eq!(normal, 2.0);
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
    fn style_keys_round_trip_through_serde() {
        for style in ConnectorStyle::ALL {
            let json = serde_json::to_string(&style).unwrap();
            assert_eq!(json, format!("\"{}\"", style.key()));
            assert_eq!(
                serde_json::from_str::<ConnectorStyle>(&json).unwrap(),
                style
            );
        }
        assert_eq!(ConnectorStyle::default(), ConnectorStyle::Curved);
    }
}
