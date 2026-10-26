//! Connectors in a scene, in the user's [`ConnectorStyle`] (spec §5).
//! Every style ends in a straight stub with its arrowhead or bar, so
//! markers line up with their line whatever the style.

use eframe::egui::{Color32, Pos2, Rect, Vec2};
use reasoncraft_core::connector::{
    ARROW_PX, ELBOW_RADIUS, EndMarker, STUB_PX, Segment, curved_path, elbow_path_at, elbow_turns,
    end_marker, line_width, tapered_band, tapered_widths,
};
use reasoncraft_core::{Connector, ConnectorKind, ConnectorStyle};

use super::build::{Builder, polyline};
use super::{PathEl, Shape, StrokeSpec};
use crate::canvas::rating::connector_strength;
use crate::theme::Tokens;

/// Opposes/rebuts end bar: length and thickness in screen px.
const BAR_PX: f32 = 11.0;
const BAR_THICKNESS: f32 = 2.4;

fn connector_colour(t: &Tokens, kind: ConnectorKind) -> Color32 {
    match kind {
        ConnectorKind::Supports => t.reason,
        ConnectorKind::Opposes => t.objection,
        ConnectorKind::Rebuts => t.rebuttal,
        ConnectorKind::Neutral => t.text_muted,
        ConnectorKind::Attachment => t.attachment_line,
    }
}

impl Builder<'_> {
    /// Where each of `connectors` turns when drawn as an elbow (staggered
    /// so runs into one parent never overlap); empty in the other styles.
    pub(super) fn elbow_turns(&self, connectors: &[Connector]) -> Vec<f32> {
        let Some(first) = connectors.first() else {
            return Vec::new();
        };
        if self.opts.connector_style != ConnectorStyle::Elbow {
            return Vec::new();
        }
        let ends: Vec<_> = connectors
            .iter()
            .map(|c| (c.from, c.to, self.stub_len(c)))
            .collect();
        elbow_turns(&ends, first.dir)
    }

    /// Connector `c`; an elbow turns at main coordinate `turn` (see
    /// [`Self::elbow_turns`]).
    pub(super) fn connector(&mut self, c: &Connector, turn: Option<f32>) {
        if !self.visible(Rect::from_two_pos(c.from, c.to)) {
            return;
        }
        let tk = self.opts.tokens;
        let colour = if c.nil {
            tk.nil
        } else {
            connector_colour(&tk, c.kind)
        };
        let strength = connector_strength(c.strength, self.opts.eval_shading);
        let zoom = self.opts.zoom;
        let toward = c.dir.toward_parent();
        let stub = self.stub_len(c);
        let marker = end_marker(c.kind);
        let style = self.opts.connector_style;
        if style == ConnectorStyle::Tapered {
            // The band ends at the stub, which is a line as wide as the
            // band's narrow end.
            let (w_child, w_parent) = tapered_widths(c.kind, strength, zoom);
            let (left, right) = tapered_band(c.from, c.to, stub, w_child, w_parent, c.dir);
            self.shapes.push(Shape::Strip {
                left,
                right,
                fill: colour,
            });
            let start = c.to - toward * stub;
            self.stub(start, c.to, toward, (w_child, colour), marker);
            return;
        }
        let stroke = (line_width(c.kind, strength, zoom), colour);
        let mut path = match style {
            ConnectorStyle::Elbow => {
                let mid = |p: Pos2| p.to_vec2().dot(toward.abs());
                let start = c.to - toward * stub;
                let turn = turn.unwrap_or((mid(c.from) + mid(start)) / 2.0);
                elbow_path_at(c.from, c.to, stub, ELBOW_RADIUS, turn, c.dir)
            }
            _ => curved_path(c.from, c.to, stub, c.dir),
        };
        // Every path ends in its stub (a straight elbow is all stub).
        let Some(Segment::Line([a, b])) = path.pop() else {
            return;
        };
        if !path.is_empty() {
            let mut points = vec![PathEl::MoveTo(c.from)];
            points.extend(path.into_iter().map(|seg| match seg {
                Segment::Cubic([_, p, q, end]) => PathEl::CubicTo(p, q, end),
                Segment::Line([_, end]) => PathEl::LineTo(end),
            }));
            self.shapes.push(Shape::Path {
                points,
                stroke: Some(stroke),
                fill: None,
                dashed: false,
            });
        }
        self.stub(a, b, toward, stroke, marker);
    }

    /// The length of `c`'s stub: a full one when there is room, otherwise
    /// half the drop (the distance from the port to the child along the
    /// main axis).
    fn stub_len(&self, c: &Connector) -> f32 {
        let full = STUB_PX * self.px;
        let drop = -(c.from - c.to).dot(c.dir.toward_parent());
        if drop >= 2.0 * full {
            full
        } else {
            (drop / 2.0).max(0.0)
        }
    }

    /// The straight end of a connector from `a` to the port `b`, with its
    /// marker oriented along it (along `toward`, toward the parent, when the
    /// stub has no length).
    fn stub(&mut self, a: Pos2, b: Pos2, toward: Vec2, stroke: StrokeSpec, marker: EndMarker) {
        let px = self.px;
        let d = b - a;
        let dir = if d.length() > 1e-3 * px {
            d.normalized()
        } else {
            toward
        };
        let side = dir.rot90();
        match marker {
            EndMarker::Arrow => {
                // Stop the line just inside the arrowhead so it cannot poke out of the tip.
                let len = d.length() - (ARROW_PX - 1.0) * px;
                if len > 0.0 {
                    self.line(&[a, a + dir * len], stroke);
                }
                let base = b - dir * ARROW_PX * px;
                let half = ARROW_PX * 0.5 * px;
                let mut points = polyline(&[b, base + side * half, base - side * half]);
                points.push(PathEl::Close);
                self.shapes.push(Shape::Path {
                    points,
                    stroke: None,
                    fill: Some(stroke.1),
                    dashed: false,
                });
            }
            EndMarker::Bar => {
                // The bar's far edge touches the port.
                let centre = b - dir * (BAR_THICKNESS * px / 2.0);
                self.line(&[a, centre], stroke);
                let half = side * (BAR_PX * px / 2.0);
                self.line(
                    &[centre - half, centre + half],
                    (BAR_THICKNESS * px, stroke.1),
                );
            }
            EndMarker::None => self.line(&[a, b], stroke),
        }
    }
}
