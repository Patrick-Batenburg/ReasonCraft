//! Rating visuals (spec §5): card tint and border by strength, the rating
//! label, and the ✓ / ? / × markers on rated boxes.

use eframe::egui::{Color32, Painter, Pos2, Rect, vec2};
use reasoncraft_core::connector::Strength;
use reasoncraft_core::{Acceptance, BoxNode, CardStrength, Role, SourceRating};

use reasoncraft_core::layout::BOX_TEXT_PAD;

use super::paint::role_colour;
use crate::i18n::t_lang;
use crate::scene::{PathEl, Shape, paint_shapes};
use crate::theme::Tokens;

/// Card frame fill alpha without evaluation shading (and when unrated).
pub const FRAME_ALPHA: f32 = 0.12;
/// Card frame fill alpha of a strong card: the strongest tint, used by the
/// label contrast test.
pub const STRONG_ALPHA: f32 = 0.22;
/// Marker diameter in world px; hidden below [`MARKER_MIN_ZOOM`].
const MARKER_SIZE: f32 = 16.0;
/// How far the marks on a box (rating marker, link glyph, comment badge)
/// sit inside its side edges, and apart from each other, in world px.
pub const CORNER_INSET: f32 = 4.0;

/// The y the marks of a box at `world` are centred on, given the height
/// `row` of its indicator row: halfway down the space above its text (the
/// row plus the text padding), so they never cover text.
pub fn indicator_centre(world: Rect, row: f32) -> f32 {
    world.top() + (row + BOX_TEXT_PAD) / 2.0
}
pub const MARKER_MIN_ZOOM: f32 = 0.35;

/// Fill alpha of a card frame.
pub fn frame_alpha(strength: Option<CardStrength>, shading: bool) -> f32 {
    if !shading {
        return FRAME_ALPHA;
    }
    match strength {
        Some(CardStrength::Strong) => STRONG_ALPHA,
        None => FRAME_ALPHA,
        Some(CardStrength::Weak) => 0.08,
        Some(CardStrength::Nil) => 0.06,
    }
}

/// Border width of a card frame in screen px.
pub fn frame_border(strength: Option<CardStrength>, shading: bool) -> f32 {
    if shading && strength == Some(CardStrength::Strong) {
        1.8
    } else {
        1.0
    }
}

/// Connector strength to draw: the evaluated one, or `Normal` without shading.
pub fn connector_strength(strength: Strength, shading: bool) -> Strength {
    if shading { strength } else { Strength::Normal }
}

/// The colour of a card's frame, label, premise borders and connector:
/// grey when rated nil, the role colour otherwise.
pub fn card_colour(t: &Tokens, role: Role, strength: Option<CardStrength>) -> Color32 {
    if strength == Some(CardStrength::Nil) {
        t.nil
    } else {
        role_colour(t, role)
    }
}

/// The label inside a card frame in `lang`, e.g. "strongly supports".
/// Its strength dots ([`rating_dots`]) are drawn beside it.
pub fn rating_label(lang: &str, role: Role, strength: Option<CardStrength>) -> String {
    let role = match role {
        Role::Supports => "supports",
        Role::Opposes => "opposes",
        Role::Rebuts => "rebuts",
    };
    let prefix = match strength {
        None => "role",
        Some(CardStrength::Strong) => "rating.strong",
        Some(CardStrength::Weak) => "rating.weak",
        Some(CardStrength::Nil) => "rating.nil",
    };
    t_lang(lang, &format!("{prefix}.{role}"))
}

/// How many strength dots lead a card's rating label: two for strong, one
/// for weak. They are drawn as shapes: the UI font has no glyph for them.
pub fn rating_dots(strength: Option<CardStrength>) -> usize {
    match strength {
        Some(CardStrength::Strong) => 2,
        Some(CardStrength::Weak) => 1,
        Some(CardStrength::Nil) | None => 0,
    }
}

/// The marker on a rated box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// ✓ accepted claim or premise, solid source.
    Yes,
    /// ? unsure, shaky source.
    Maybe,
    /// × rejected, nil source.
    No,
}

/// The marker a box shows, if it is rated.
pub fn mark_for(node: &BoxNode) -> Option<Mark> {
    let from_acceptance = node.acceptance.map(|a| match a {
        Acceptance::Accept => Mark::Yes,
        Acceptance::Unsure => Mark::Maybe,
        Acceptance::Reject => Mark::No,
    });
    let from_source = node.source_rating.map(|r| match r {
        SourceRating::Solid => Mark::Yes,
        SourceRating::Shaky => Mark::Maybe,
        SourceRating::Nil => Mark::No,
    });
    from_acceptance.or(from_source)
}

/// Centre and radius (world units) of the marker of a box at `world`.
///
/// The circle sits at the right end of the box's indicator row (`row`
/// high), [`CORNER_INSET`] from the right edge: inside the box, so it never
/// reaches into a neighbouring box or onto a connector's arrowhead (which
/// end outside the box), and above the text.
pub fn marker_circle(world: Rect, row: f32) -> (Pos2, f32) {
    let r = MARKER_SIZE / 2.0;
    let x = world.right() - CORNER_INSET - r;
    (Pos2::new(x, indicator_centre(world, row)), r)
}

/// A ✓ / ? / × marker: a circle of radius `r` (screen px) centred on `c`,
/// with strokes scaled to the canvas marker's at that size.
pub fn paint_mark(painter: &Painter, c: Pos2, r: f32, mark: Mark, t: &Tokens) {
    paint_shapes(painter, &mark_shapes(c, r, mark, t, 1.0));
}

/// A ✓ / ? / × marker as shapes: a circle of radius `r` centred on `c`,
/// strokes scaled to the canvas marker's at that size. `px` is the world
/// size of a screen px: strokes stay at least about one screen px wide.
pub fn mark_shapes(c: Pos2, r: f32, mark: Mark, t: &Tokens, px: f32) -> Vec<Shape> {
    let z = r / (MARKER_SIZE / 2.0);
    let colour = match mark {
        Mark::Yes => t.reason,
        Mark::Maybe => t.rebuttal,
        Mark::No => t.objection,
    };
    let width = (1.6 * z).max(1.25 * px);
    let stroke = Some((width, colour));
    // Glyph coordinates in units of the radius, centred on `c`.
    let at = |x: f32, y: f32| c + vec2(x, y) * r;
    let line = |pts: &[Pos2]| Shape::Path {
        points: pts
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                if i == 0 {
                    PathEl::MoveTo(p)
                } else {
                    PathEl::LineTo(p)
                }
            })
            .collect(),
        stroke,
        fill: None,
        dashed: false,
    };
    let mut shapes = vec![Shape::Circle {
        center: c,
        radius: r,
        fill: Some(t.box_fill),
        stroke: Some(((1.2 * z).max(px), colour)),
    }];
    match mark {
        Mark::Yes => {
            shapes.push(line(&[at(-0.45, 0.02), at(-0.12, 0.35), at(0.45, -0.32)]));
        }
        Mark::No => {
            let d = 0.36;
            shapes.push(line(&[at(-d, -d), at(d, d)]));
            shapes.push(line(&[at(-d, d), at(d, -d)]));
        }
        Mark::Maybe => {
            // The hook of a "?": an arc over the top, then the stem.
            let (cy, rad) = (-0.2, 0.28);
            let mut points: Vec<Pos2> = (0..=8)
                .map(|i| {
                    let a = (160.0 - i as f32 * 26.0_f32).to_radians();
                    at(a.cos() * rad, cy - a.sin() * rad)
                })
                .collect();
            points.push(at(0.0, 0.18));
            shapes.push(line(&points));
            shapes.push(Shape::Circle {
                center: at(0.0, 0.46),
                radius: width * 0.65,
                fill: Some(colour),
                stroke: None,
            });
        }
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn rating_label_variants() {
        use CardStrength::*;
        let cases = [
            (Role::Supports, Some(Strong), "strongly supports"),
            (Role::Opposes, Some(Strong), "strongly opposes"),
            (Role::Rebuts, Some(Strong), "strongly rebuts"),
            (Role::Supports, Some(Weak), "weakly supports"),
            (Role::Opposes, Some(Weak), "weakly opposes"),
            (Role::Rebuts, Some(Weak), "weakly rebuts"),
            (Role::Supports, None, "supports"),
            (Role::Opposes, None, "opposes"),
            (Role::Rebuts, None, "rebuts"),
            (Role::Supports, Some(Nil), "does not support"),
            (Role::Opposes, Some(Nil), "does not oppose"),
            (Role::Rebuts, Some(Nil), "does not rebut"),
        ];
        for (role, strength, want) in cases {
            let label = rating_label("en", role, strength);
            assert_eq!(label, want, "{role:?} {strength:?}");
        }
    }

    #[test]
    fn rating_dots_drawn_by_count_not_parsed() {
        use CardStrength::*;
        assert_eq!(rating_dots(Some(Strong)), 2);
        assert_eq!(rating_dots(Some(Weak)), 1);
        assert_eq!(rating_dots(Some(Nil)), 0);
        assert_eq!(rating_dots(None), 0);
        let ftl = [
            include_str!("../../i18n/en.ftl"),
            include_str!("../../i18n/nl.ftl"),
        ];
        for text in ftl {
            assert!(!text.contains('●'), "the dots are drawn, not translated");
        }
    }

    #[test]
    fn frame_alpha_by_strength_and_shading() {
        use CardStrength::*;
        assert_eq!(frame_alpha(Some(Strong), true), 0.22);
        assert_eq!(frame_alpha(None, true), 0.12);
        assert_eq!(frame_alpha(Some(Weak), true), 0.08);
        assert_eq!(frame_alpha(Some(Nil), true), 0.06);
        for s in [Some(Strong), None, Some(Weak), Some(Nil)] {
            assert_eq!(frame_alpha(s, false), 0.12);
        }
        assert_eq!(frame_border(Some(Strong), true), 1.8);
        assert_eq!(frame_border(Some(Strong), false), 1.0);
        assert_eq!(frame_border(Some(Weak), true), 1.0);
        assert_eq!(
            connector_strength(Strength::Strong, false),
            Strength::Normal
        );
        assert_eq!(connector_strength(Strength::Weak, true), Strength::Weak);
    }

    #[test]
    fn nil_cards_are_grey() {
        let t = Tokens::dark();
        assert_eq!(
            card_colour(&t, Role::Opposes, Some(CardStrength::Nil)),
            t.nil
        );
        assert_eq!(card_colour(&t, Role::Opposes, None), t.objection);
    }

    #[test]
    fn marker_sits_at_the_right_of_the_indicator_row() {
        let world = Rect::from_min_size(pos2(100.0, 50.0), vec2(156.0, 40.0));
        let (c, r) = marker_circle(world, 16.0);
        assert_eq!(c.x + r, world.right() - CORNER_INSET);
        assert_eq!(c.y, world.top() + 11.0);
        // Above the text, which starts below the row and its padding.
        assert!(c.y + r <= world.top() + 16.0 + BOX_TEXT_PAD);
    }

    #[test]
    fn marks_from_acceptance_and_source_rating() {
        let mut b = BoxNode::new(
            reasoncraft_core::NodeId::new(),
            reasoncraft_core::BoxKind::Premise,
            "p",
        );
        assert_eq!(mark_for(&b), None);
        b.acceptance = Some(Acceptance::Unsure);
        assert_eq!(mark_for(&b), Some(Mark::Maybe));
        b.acceptance = None;
        b.source_rating = Some(SourceRating::Nil);
        assert_eq!(mark_for(&b), Some(Mark::No));
    }
}
