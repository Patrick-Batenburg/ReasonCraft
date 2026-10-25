//! Outline Markdown tests (step 4, task 6).

use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::outline_md::{outline_markdown, outline_plain_text_lang};
use reasoncraft_core::{BoxKind, CardKind, Document, NewLoose, Parent, SourceType};

fn sample(claim: &str, reason: &str) -> Document {
    let mut d = Document::new(claim);
    let root = Parent::Box(d.root);
    let (_, p) = d.add_card(root, CardKind::Reason, reason).unwrap();
    let src = BoxKind::Source {
        source_type: SourceType::Law,
    };
    d.add_box(Parent::Box(p), src, "art. 1").unwrap();
    d.add_card(root, CardKind::Objection, "O2").unwrap();
    d.add_loose(NewLoose::Card(CardKind::Reason), "Loose");
    d
}

fn md(d: &Document, opts: &OutlineOptions, lang: &str) -> String {
    outline_markdown(&build_outline(d, opts), opts, lang)
}

#[test]
fn markdown_matches_expected() {
    let d = sample("Claim", "R1");
    let o = OutlineOptions::default();
    assert_eq!(
        md(&d, &o, "en"),
        "# Claim\n\n- **1** *because* R1\n  - _Law:_ art. 1\n- **2** *but* O2\n\n## Not connected\n\n- **P1** *because* Loose"
    );
    assert_eq!(
        md(&d, &o, "nl"),
        "# Claim\n\n- **1** *omdat* R1\n  - _Wet:_ art. 1\n- **2** *maar* O2\n\n## Niet verbonden\n\n- **P1** *omdat* Loose"
    );
    let bare = OutlineOptions {
        numbering: false,
        labels: false,
        notes: false,
    };
    assert_eq!(
        md(&d, &bare, "en"),
        "# Claim\n\n- R1\n  - _Law:_ art. 1\n- O2\n\n## Not connected\n\n- Loose"
    );
    let plain = outline_plain_text_lang(&build_outline(&d, &o), &o, "nl");
    assert!(plain.contains("1 omdat R1") && plain.contains("[Wet] art. 1"));
}

#[test]
fn markdown_escapes_special_chars() {
    let d = sample("A *b* # c", "x_y `z` [l] <t> \\\nline two\n- item\n3. num");
    let o = OutlineOptions::default();
    let expected = [
        r"# A \*b\* \# c",
        "",
        r"- **1** *because* x\_y \`z\` \[l\] \<t\> \\",
        "  line two",
        r"  \- item",
        r"  3\. num",
        "  - _Law:_ art. 1",
        "- **2** *but* O2",
        "",
        "## Not connected",
        "",
        "- **P1** *because* Loose",
    ];
    assert_eq!(md(&d, &o, "en"), expected.join("\n"));
}

#[test]
fn leading_list_markers_escaped_without_prefix() {
    let d = sample("Claim", "- dash");
    let o = OutlineOptions {
        numbering: false,
        labels: false,
        notes: false,
    };
    assert!(md(&d, &o, "en").contains("\n- \\- dash\n"));
}

#[test]
fn markdown_escapes_tables_entities_strike_and_trims_blank_lines() {
    let d = sample("Claim", "\n  \na | b & ~c~\n\nafter gap\n \n");
    let o = OutlineOptions::default();
    let out = md(&d, &o, "en");
    assert!(
        out.contains("- **1** *because* a \\| b \\& \\~c\\~\n\n  after gap\n  - _Law:_"),
        "{out}"
    );
}
