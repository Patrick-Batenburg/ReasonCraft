use eframe::egui::{Align, Rect};
use reasoncraft_core::{
    BoxKind, CardKind, Comment, Document, Item, Layout, LayoutParams, Measure, NewLoose, Parent,
    Uuid, Verdict, layout,
};

use super::*;
use crate::canvas::comments::badge_rect;
use crate::canvas::links::link_rect;
use crate::i18n::t;

const LINE: f32 = 15.0;

/// Splits text at "|" into rows, ignoring the width; measures accordingly.
struct Fake;

impl Measure for Fake {
    fn text_height(&self, text: &str, width: f32, kind: &BoxKind) -> f32 {
        self.rows(text, width, kind).len() as f32 * LINE
    }
}

impl TextRows for Fake {
    fn rows(&self, text: &str, _width: f32, _kind: &BoxKind) -> Vec<String> {
        text.split('|').map(str::to_owned).collect()
    }
    fn line_height(&self, _size: f32) -> f32 {
        LINE
    }
    fn text_width(&self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.5
    }
}

fn lay_out(doc: &Document) -> Layout {
    layout(doc, &Fake, &LayoutParams::default())
}

fn opts() -> SceneOptions {
    SceneOptions::export(Tokens::dark())
}

fn texts(scene: &Scene) -> Vec<String> {
    scene
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Text { rows, .. } => Some(rows.join("|")),
            _ => None,
        })
        .collect()
}

/// Rect shapes exactly at `r`.
fn rects_at(scene: &Scene, r: Rect) -> usize {
    scene
        .shapes
        .iter()
        .filter(|s| matches!(s, Shape::Rect { rect, .. } if *rect == r))
        .count()
}

fn curves(scene: &Scene) -> usize {
    scene
        .shapes
        .iter()
        .filter(|s| match s {
            Shape::Path { points, .. } => points.iter().any(|p| matches!(p, PathEl::CubicTo(..))),
            _ => false,
        })
        .count()
}

#[test]
fn scene_has_no_overlays_for_export() {
    let mut doc = Document::new("");
    let root = doc.root;
    let (card, premise) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let node = doc.boxes.get_mut(&premise).unwrap();
    node.link = Some("https://example.org".into());
    node.comments.push(Comment {
        id: Uuid::new_v4(),
        author: "A".into(),
        text: "Nice".into(),
        verdict: Some(Verdict::Positive),
        created: String::new(),
    });
    doc.add_loose(NewLoose::Note, "Parked");
    let layout = lay_out(&doc);
    let mut o = opts();
    o.for_export = true;
    o.include_parking = false;
    let scene = build_scene(&doc, &layout, &o, &Fake);
    let tk = o.tokens;

    let world = layout.boxes[&premise];
    let row = layout.indicator_row(premise);
    let badge = badge_rect(world, row, 0.0, 1);
    let glyph = link_rect(world, row, 0.0);
    for s in &scene.shapes {
        let b = s.bounds();
        assert!(!badge.contains_rect(b), "badge shape: {s:?}");
        assert!(!glyph.contains_rect(b), "link glyph shape: {s:?}");
        let colours = match s {
            Shape::Rect { fill, stroke, .. } => vec![Some(*fill), stroke.map(|s| s.1)],
            Shape::Path { fill, stroke, .. } | Shape::Circle { fill, stroke, .. } => {
                vec![*fill, stroke.map(|s| s.1)]
            }
            Shape::Text { color, .. } => vec![Some(*color)],
            Shape::Strip { fill, .. } => vec![Some(*fill)],
            Shape::Image { .. } => vec![],
        };
        assert!(
            !colours.contains(&Some(tk.accent)),
            "selection ring or glyph: {s:?}"
        );
        assert!(
            !matches!(s, Shape::Path { dashed: true, .. }),
            "parking frame"
        );
    }
    let shown = texts(&scene);
    assert!(shown.contains(&"Premise".to_owned()), "{shown:?}");
    assert!(!shown.contains(&"1".to_owned()), "badge count: {shown:?}");
    assert!(!shown.contains(&t("canvas.empty_claim")), "screen hint");
    assert!(!shown.contains(&t("parking.title")), "{shown:?}");
    assert!(!shown.contains(&"Parked".to_owned()), "parked items");
    assert!(layout.cards.contains_key(&card));

    // On screen the hint and the parking area are there.
    o.for_export = false;
    o.include_parking = true;
    let screen = texts(&build_scene(&doc, &layout, &o, &Fake));
    assert!(screen.contains(&t("canvas.empty_claim")), "{screen:?}");
    assert!(screen.contains(&t("parking.title")), "{screen:?}");
    assert!(screen.contains(&"Parked".to_owned()), "{screen:?}");
}

#[test]
fn scene_selection_subtree_only() {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (a, pa) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "A")
        .unwrap();
    let (b, pb) = doc
        .add_card(Parent::Box(pa), CardKind::Objection, "B")
        .unwrap();
    let (c, pc) = doc
        .add_card(Parent::Box(root), CardKind::Objection, "C")
        .unwrap();
    let layout = lay_out(&doc);
    let mut o = opts();
    o.only = Some(Item::Card(a));
    let scene = build_scene(&doc, &layout, &o, &Fake);

    for id in [pa, pb] {
        assert_eq!(rects_at(&scene, layout.boxes[&id]), 1, "box kept");
    }
    for id in [root, pc] {
        assert_eq!(rects_at(&scene, layout.boxes[&id]), 0, "box left out");
    }
    for id in [a, b] {
        assert_eq!(rects_at(&scene, layout.cards[&id]), 1, "frame kept");
    }
    assert_eq!(rects_at(&scene, layout.cards[&c]), 0, "frame left out");
    // Only B's connector up to A's premise; A's own one leads out.
    assert_eq!(curves(&scene), 1);
    let shown = texts(&scene);
    assert!(shown.contains(&"A".to_owned()) && shown.contains(&"B".to_owned()));
    assert!(!shown.contains(&"Claim".to_owned()) && !shown.contains(&"C".to_owned()));
    // The bounds cover the subtree, not the whole map.
    let sub = layout.cards[&a].union(layout.cards[&b]);
    assert!(scene.bounds.contains_rect(sub));
    assert!(!scene.bounds.contains_rect(layout.boxes[&root]));
}

#[test]
fn scene_text_rows_from_measure() {
    let mut doc = Document::new("first row|second row");
    let root = doc.root;
    doc.add_box(
        Parent::Box(root),
        BoxKind::Source {
            source_type: reasoncraft_core::SourceType::Web,
        },
        "cite|more",
    )
    .unwrap();
    let layout = lay_out(&doc);
    let scene = build_scene(&doc, &layout, &opts(), &Fake);
    let world = layout.boxes[&root];
    let claim = scene
        .shapes
        .iter()
        .find_map(|s| match s {
            Shape::Text {
                origin,
                rows,
                align,
                line_height,
                ..
            } if rows[0] == "first row" => Some((*origin, rows.clone(), *align, *line_height)),
            _ => None,
        })
        .expect("claim text");
    let (origin, rows, align, line_height) = claim;
    assert_eq!(rows, ["first row", "second row"]);
    assert_eq!(align, Align::Center);
    assert_eq!(line_height, LINE);
    assert_eq!(origin.x, world.center().x);
    // Vertically centred: two rows around the box centre.
    assert!((origin.y - (world.center().y - LINE)).abs() < 1e-4);
    let all = texts(&scene);
    assert!(all.contains(&"cite|more".to_owned()), "{all:?}");
    assert!(all.contains(&t("source.web")), "bold type name");
}

#[test]
fn view_culls_offscreen_boxes() {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (_, p) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "A")
        .unwrap();
    let layout = lay_out(&doc);
    let mut o = opts();
    o.view = Some(layout.boxes[&root].expand(2.0));
    let scene = build_scene(&doc, &layout, &o, &Fake);
    assert_eq!(rects_at(&scene, layout.boxes[&root]), 1);
    assert_eq!(rects_at(&scene, layout.boxes[&p]), 0);
}

/// Layout parameters with the numbers on, as the settings make them.
fn numbered_params() -> LayoutParams {
    let mut p = LayoutParams::default();
    p.indicators.numbers = true;
    p
}

/// The muted one-row texts of `scene` (the numbers), with their origins.
fn number_texts(scene: &Scene, tk: &Tokens) -> Vec<(String, eframe::egui::Pos2)> {
    scene
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Text {
                origin,
                rows,
                color,
                align: Align::Min,
                size,
                ..
            } if *color == tk.text_muted && *size == NUMBER_SIZE => Some((rows.join("|"), *origin)),
            _ => None,
        })
        .collect()
}

#[test]
fn numbers_match_outline() {
    use reasoncraft_core::fixtures::sample_iot_like;
    use reasoncraft_core::outline::{OutlineOptions, build_outline};
    let doc = sample_iot_like(40);
    let labels = number_labels(&doc);
    let outline = build_outline(
        &doc,
        &OutlineOptions {
            numbering: true,
            ..OutlineOptions::default()
        },
    );
    let numbered: Vec<_> = outline
        .iter()
        .filter(|l| !l.number.is_empty() && matches!(l.item, Item::Box(_)))
        .collect();
    assert!(numbered.len() > 20, "a well-numbered sample");
    assert_eq!(labels.len(), numbered.len());
    assert!(labels.keys().all(|i| matches!(i, Item::Box(_))), "no cards");
    // The map draws each number in its own box.
    let layout = layout(&doc, &Fake, &numbered_params());
    let mut o = opts();
    o.numbers = true;
    o.include_parking = true;
    let drawn = number_texts(&build_scene(&doc, &layout, &o, &Fake), &o.tokens);
    for line in numbered {
        assert_eq!(labels[&line.item], line.number);
        let Item::Box(id) = line.item else { continue };
        let world = layout.boxes[&id];
        assert!(
            drawn
                .iter()
                .any(|(n, at)| *n == line.number && world.contains(*at)),
            "{} not drawn in its box",
            line.number
        );
    }
    // Numbers off: none drawn.
    o.numbers = false;
    let off = build_scene(&doc, &layout, &o, &Fake);
    assert!(number_texts(&off, &o.tokens).is_empty());
}

#[test]
fn numbers_never_overlap_text() {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (_, premise) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Premise text")
        .unwrap();
    let plain = layout(&doc, &Fake, &LayoutParams::default());
    let numbered = layout(&doc, &Fake, &numbered_params());
    assert_eq!(plain.indicator_row(premise), 0.0);
    assert!(
        numbered.indicator_row(premise) > 0.0,
        "a numbered box gets the row"
    );
    let node = doc.boxes.get_mut(&premise).unwrap();
    node.link = Some("https://example.org".into());
    let layout = layout(&doc, &Fake, &numbered_params());
    let row = layout.indicator_row(premise);
    let mut o = opts();
    o.numbers = true;
    let scene = build_scene(&doc, &layout, &o, &Fake);
    let world = layout.boxes[&premise];
    let number = number_rect(world, row, "1", &Fake, 1.0);
    let drawn = number_texts(&scene, &o.tokens);
    assert_eq!(drawn, [("1".to_owned(), number.min)]);
    let first_row = scene
        .shapes
        .iter()
        .find_map(|s| match s {
            Shape::Text {
                origin, rows, size, ..
            } if rows[0] == "Premise text" => {
                let w = Fake.text_width(&rows[0], *size);
                Some(Rect::from_min_size(
                    *origin - eframe::egui::vec2(w / 2.0, 0.0),
                    eframe::egui::vec2(w, LINE),
                ))
            }
            _ => None,
        })
        .expect("premise text");
    assert!(world.contains_rect(number));
    assert!(!number.intersects(first_row), "{number:?} {first_row:?}");
    // The link glyph moves right of the number.
    let leads = marks_leads(&doc, &layout, &Fake, 1.0);
    let glyph = link_rect(world, row, leads[&premise]);
    assert!(glyph.left() > number.right());
    assert!(!glyph.intersects(first_row));
}
