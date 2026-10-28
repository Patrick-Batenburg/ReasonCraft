//! What changes without the focus moving is read out: the find count,
//! status notes, "Saving…", the Move to… hint, and the misspelled word
//! (said in words, not only by colour) (ROB-12).

use super::*;
use egui_kittest::Node;

/// The polite live regions shown now, with the text inside each.
fn live_texts(h: &H) -> Vec<String> {
    fn text(node: &Node<'_>, out: &mut String) {
        let n = node.accesskit_node();
        if let Some(v) = n.value().or_else(|| n.label()) {
            out.push_str(&v);
            out.push(' ');
        }
        for child in node.children() {
            text(&child, out);
        }
    }
    h.root()
        .children_recursive()
        .filter(|n| n.accesskit_node().live() == accesskit::Live::Polite)
        .map(|n| {
            let mut out = String::new();
            text(&n, &mut out);
            out
        })
        .collect()
}

fn read_out(h: &H, what: &str) -> bool {
    live_texts(h).iter().any(|t| t.contains(what))
}

#[test]
fn find_count_is_read_out() {
    let (mut h, _) = sample_harness();
    act(&mut h, Action::OpenFind { replace: false });
    act(&mut h, Action::SetFindQuery("They".into()));
    let mut args = FluentArgs::new();
    args.set("current", 1);
    args.set("total", 2);
    assert!(
        read_out(&h, &t_args("find.count", &args)),
        "{:?}",
        live_texts(&h)
    );
    act(&mut h, Action::SetFindQuery("zebra".into()));
    assert!(read_out(&h, &t("find.none")));
}

#[test]
fn status_notes_and_the_move_hint_are_read_out() {
    let (mut h, reason) = sample_harness();
    h.state_mut().status_note = Some("Autosave failed".into());
    h.run();
    assert!(read_out(&h, "Autosave failed"), "{:?}", live_texts(&h));
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::StartConnect);
    assert!(read_out(&h, &t("connect.hint")));
}

#[test]
fn the_misspelled_word_is_said_in_words() {
    let (mut h, reason) = sample_harness();
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::OpenSpelling);
    wait_dictionary(&mut h, "en");
    let mut args = FluentArgs::new();
    args.set("word", "recieve");
    let said = t_args("spell.not_in_dictionary", &args);
    assert!(read_out(&h, &said), "{:?}", live_texts(&h));
}
