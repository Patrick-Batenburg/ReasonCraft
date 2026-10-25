use super::tests::{card_under_root, publication};
use super::*;
use crate::ids::CardId;
use crate::model::{BoxNode, SourceType};

/// A root with one reason card `a` (premise `p`) whose premise has an
/// objection card `b` (premise `q`).
fn two_level() -> (Document, CardId, NodeId, CardId, NodeId) {
    let mut d = Document::new("C");
    let (a, p) = card_under_root(&mut d, "P");
    let (b, q) = d
        .add_card(Parent::Box(p), CardKind::Objection, "Q")
        .unwrap();
    (d, a, p, b, q)
}

#[test]
fn validate_accepts_edited_documents() {
    let (mut d, a, _, _, _) = two_level();
    d.add_card(Parent::Card(a), CardKind::Objection, "W")
        .unwrap();
    d.add_loose(NewLoose::Note, "n");
    assert_eq!(d.validate(), Ok(()));
}

#[test]
fn validate_rejects_damage() {
    let (d, a, p, b, q) = two_level();
    let root = d.root;
    let broken = |f: &dyn Fn(&mut Document)| {
        let mut x = d.clone();
        f(&mut x);
        x.validate()
    };
    // Dangling id.
    assert!(broken(&|x| x.parking.push(Item::Card(CardId::new()))).is_err());
    // Root missing, or not a claim, or owned.
    assert!(broken(&|x| x.root = NodeId::new()).is_err());
    assert!(broken(&|x| x.boxes[&root].kind = BoxKind::Note).is_err());
    assert!(broken(&|x| x.parking.push(Item::Box(root))).is_err());
    // Owned twice.
    assert!(broken(&|x| x.parking.push(Item::Card(b))).is_err());
    // Unowned.
    assert!(broken(&|x| x.boxes[&p].children.clear()).is_err());
    // A premise moved out of its card into a box or the parking area.
    let out_of_card = |to_parking: bool| {
        move |x: &mut Document| {
            x.cards[&b].premises.push(NodeId::new());
            let extra = *x.cards[&b].premises.last().unwrap();
            x.boxes
                .insert(extra, BoxNode::new(extra, BoxKind::Premise, "x"));
            x.cards[&b].premises.retain(|i| *i != q);
            if to_parking {
                x.parking.push(Item::Box(q));
            } else {
                x.boxes[&root].children.push(Item::Box(q));
            }
        }
    };
    assert!(broken(&out_of_card(true)).unwrap_err().contains("premise"));
    assert!(broken(&out_of_card(false)).unwrap_err().contains("premise"));
    // A non-premise box listed as a premise.
    assert!(broken(&|x| x.boxes[&q].kind = BoxKind::Note).is_err());
    // A card without premises.
    assert!(broken(&|x| x.cards[&b].premises.clear()).is_err());
    // A card removed from the map but still referenced.
    assert!(
        broken(&|x| {
            x.cards.shift_remove(&a);
        })
        .is_err()
    );
}

#[test]
fn validate_rejects_cycles_and_descendants_terminates() {
    let (mut d, a, p, b, _) = two_level();
    // Detach `a` from the root and hang it under its own descendant:
    // a -> p -> b -> q, and q now owns a.
    let root = d.root;
    d.boxes[&root].children.clear();
    let q = d.cards[&b].premises[0];
    d.boxes[&q].children.push(Item::Card(a));
    let err = d.validate().unwrap_err();
    assert!(err.contains("unreachable"), "{err}");
    let under = d.descendants(Item::Card(a));
    assert!(under.contains(&Item::Box(p)));
    assert!(under.contains(&Item::Card(b)));
    assert!(!under.contains(&Item::Card(a)));
}

/// Take a loose `item` out of the parking area and hang it under `owner`
/// without any rule check, to build a structurally damaged document.
fn force_under(d: &mut Document, owner: NodeId, item: Item) {
    d.parking.retain(|i| *i != item);
    d.boxes[&owner].children.push(item);
}

/// A grouping map: a Category, a Source and a Note under the root.
fn grouping() -> (Document, NodeId, NodeId, NodeId) {
    let mut d = Document::new("Topic");
    let root = Parent::Box(d.root);
    let category = d.add_box(root, BoxKind::Category, "Industry").unwrap();
    let source = d.add_box(root, publication(), "Report").unwrap();
    let note = d.add_box(root, BoxKind::Note, "Remark").unwrap();
    d.validate().unwrap();
    (d, category, source, note)
}

#[test]
fn validate_rejects_key_id_mismatch() {
    let (mut d, ..) = two_level();
    let id = *d.boxes.keys().nth(1).unwrap();
    let n = d.boxes.shift_remove(&id).unwrap();
    d.boxes.insert(NodeId::new(), n);
    assert!(d.validate().unwrap_err().contains("key"));

    let (mut d, a, ..) = two_level();
    let card = d.cards.shift_remove(&a).unwrap();
    d.cards.insert(CardId::new(), card);
    assert!(d.validate().unwrap_err().contains("key"));
}

#[test]
fn validate_rejects_source_under_category() {
    let (mut d, category, ..) = grouping();
    let loose = d.add_loose(NewLoose::Source(SourceType::Law), "Act");
    force_under(&mut d, category, loose);
    assert!(d.validate().unwrap_err().contains("cannot"));
}

#[test]
fn validate_rejects_card_under_category() {
    let (mut d, category, ..) = grouping();
    let loose = d.add_loose(NewLoose::Card(CardKind::Reason), "Because");
    force_under(&mut d, category, loose);
    assert!(d.validate().unwrap_err().contains("cannot"));
}

#[test]
fn validate_rejects_category_under_premise() {
    let (mut d, _, p, ..) = two_level();
    let loose = d.add_loose(NewLoose::Category, "Group");
    force_under(&mut d, p, loose);
    assert!(d.validate().unwrap_err().contains("cannot"));
}

#[test]
fn validate_rejects_note_with_children() {
    let (mut d, _, _, note) = grouping();
    let loose = d.add_loose(NewLoose::Note, "Nested");
    force_under(&mut d, note, loose);
    assert!(d.validate().unwrap_err().contains("cannot"));
}

#[test]
fn validate_rejects_source_with_non_note_children() {
    let (mut d, _, source, _) = grouping();
    let loose = d.add_loose(NewLoose::Source(SourceType::Law), "Act");
    force_under(&mut d, source, loose);
    assert!(d.validate().is_err());
    let (mut d, _, source, _) = grouping();
    let loose = d.add_loose(NewLoose::Card(CardKind::Reason), "Because");
    force_under(&mut d, source, loose);
    assert!(d.validate().is_err());
}

#[test]
fn validate_rejects_root_mixing_cards_and_categories() {
    let (mut d, ..) = grouping();
    let root = d.root;
    let loose = d.add_loose(NewLoose::Card(CardKind::Reason), "Because");
    force_under(&mut d, root, loose);
    assert!(d.validate().unwrap_err().contains("mix"));
}

#[test]
fn validate_accepts_every_attachment_the_rules_allow() {
    let (mut d, category, source, _) = grouping();
    let inner = d.add_loose(NewLoose::Category, "Inner");
    force_under(&mut d, category, inner);
    let n1 = d.add_loose(NewLoose::Note, "n1");
    force_under(&mut d, category, n1);
    let n2 = d.add_loose(NewLoose::Note, "n2");
    force_under(&mut d, source, n2);
    d.validate().unwrap();
}

#[test]
fn validate_accepts_every_fixture() {
    for n in [1, 10, 300] {
        crate::fixtures::sample_iot_like(n).validate().unwrap();
    }
}

#[test]
fn validate_reports_a_dangling_child_after_a_valid_one_without_panicking() {
    let (mut d, ..) = grouping();
    let root = d.root;
    d.boxes[&root].children.push(Item::Box(NodeId::new()));
    assert!(d.validate().unwrap_err().contains("missing"));
    let (mut d, ..) = two_level();
    let root = d.root;
    d.boxes[&root].children.push(Item::Card(CardId::new()));
    assert!(d.validate().unwrap_err().contains("missing"));
}
