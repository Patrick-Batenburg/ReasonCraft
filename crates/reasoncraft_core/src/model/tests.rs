use super::*;

#[test]
fn new_document_has_root_claim() {
    let d = Document::new("Robots create more jobs");
    let root = &d.boxes[&d.root];
    assert_eq!(root.kind, BoxKind::Claim);
    assert_eq!(root.text, "Robots create more jobs");
    assert_eq!(d.schema_version, SCHEMA_VERSION);
    assert!(d.parking.is_empty());
    assert_eq!(d.map_kind(), MapKind::Empty);
}

#[test]
fn rebuttal_is_derived_from_structure() {
    // root <- objection(card A, premise p) <- objection(card B) => B rebuts
    let mut d = Document::new("C");
    let a = CardId::new();
    let p = NodeId::new();
    let b = CardId::new();
    let q = NodeId::new();
    d.boxes.insert(p, BoxNode::new(p, BoxKind::Premise, "P"));
    d.boxes.insert(q, BoxNode::new(q, BoxKind::Premise, "Q"));
    d.cards
        .insert(a, Card::new(a, CardKind::Objection, vec![p]));
    d.cards
        .insert(b, Card::new(b, CardKind::Objection, vec![q]));
    d.boxes
        .get_mut(&d.root)
        .unwrap()
        .children
        .push(Item::Card(a));
    d.boxes.get_mut(&p).unwrap().children.push(Item::Card(b));
    assert_eq!(d.card_role(a), Role::Opposes);
    assert_eq!(d.card_role(b), Role::Rebuts);
    assert_eq!(d.map_kind(), MapKind::Reasoning);
}

fn step1_json() -> String {
    let root = NodeId::new();
    let src = NodeId::new();
    format!(
        r#"{{"schema_version":1,
"meta":{{"title":"t","author":"a","created":"c","modified":"m","language":"en"}},
"root":"{root}",
"boxes":{{
 "{root}":{{"id":"{root}","kind":{{"type":"claim"}},"text":"C","children":[{{"Box":"{src}"}}]}},
 "{src}":{{"id":"{src}","kind":{{"type":"source","source_type":"publication"}},"text":"S","children":[]}}
}},
"cards":{{}},"parking":[]}}"#,
        root = root.0,
        src = src.0
    )
}

#[test]
fn source_type_compat_and_unknown_fallback() {
    let k: BoxKind =
        serde_json::from_str(r#"{"type":"source","source_type":"publication"}"#).unwrap();
    assert_eq!(
        k,
        BoxKind::Source {
            source_type: SourceType::Publication
        }
    );
    let k: BoxKind = serde_json::from_str(r#"{"type":"source","source_type":"hologram"}"#).unwrap();
    let BoxKind::Source { source_type } = k else {
        panic!("not a source")
    };
    assert_eq!(source_type, SourceType::Other("hologram".to_owned()));
    assert_eq!(source_type.key(), "source");
    assert_eq!(SourceType::ALL.len(), 17);
    assert!(!SourceType::ALL.contains(&source_type));
    for t in SourceType::ALL {
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, format!("\"{}\"", t.key()));
        assert_eq!(serde_json::from_str::<SourceType>(&json).unwrap(), t);
    }
}

#[test]
fn unknown_source_type_survives_resave() {
    let kind = r#"{"type":"source","source_type":"hologram"}"#;
    let k: BoxKind = serde_json::from_str(kind).unwrap();
    assert_eq!(serde_json::to_string(&k).unwrap(), kind);

    let json = step1_json().replace("\"publication\"", "\"hologram\"");
    let d: Document = serde_json::from_str(&json).unwrap();
    let out = serde_json::to_string(&d).unwrap();
    assert!(out.contains(r#""source_type":"hologram""#), "{out}");
    let again: Document = serde_json::from_str(&out).unwrap();
    assert_eq!(serde_json::to_string(&again).unwrap(), out);
}

#[test]
fn old_document_json_still_deserializes() {
    let d: Document = serde_json::from_str(&step1_json()).unwrap();
    let out = serde_json::to_string(&d).unwrap();
    let again: Document = serde_json::from_str(&out).unwrap();
    assert_eq!(d, again);
    assert!(d.extra.is_empty());
    assert!(d.boxes.values().all(|b| b.extra.is_empty()));
    assert_eq!(d.validate(), Ok(()));
    // No new keys are written for untouched boxes.
    for key in [
        "acceptance",
        "source_rating",
        "link",
        "comments",
        "image",
        "strength",
    ] {
        assert!(!out.contains(key), "{key} was written");
    }
}

#[test]
fn new_fields_do_not_leak_into_extra() {
    let mut d = Document::new("C");
    let root = d.root;
    let b = d.boxes.get_mut(&root).unwrap();
    b.acceptance = Some(Acceptance::Accept);
    b.link = Some("https://example.org".into());
    b.image = Some(ImageRef::new("a".into(), 0.5));
    b.comments.push(Comment {
        id: Uuid::new_v4(),
        author: "A".into(),
        text: "T".into(),
        verdict: Some(Verdict::NeedsWork),
        created: now_rfc3339(),
        extra: Default::default(),
    });
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("\"acceptance\":\"accept\""));
    let back: Document = serde_json::from_str(&json).unwrap();
    assert!(back.boxes[&root].extra.is_empty());
    assert_eq!(back, d);
}
