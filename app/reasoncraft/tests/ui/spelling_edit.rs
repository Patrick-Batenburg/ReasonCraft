//! The spelling pass (F7) waits while a box is being edited inline: the
//! selection stays on the box being edited until the edit ends.

use crate::common::{app_with_doc, harness};
use crate::spelling::{current_word, map, wait_dictionary};
use reasoncraft::actions::{Action, apply};
use reasoncraft_core::Item;

#[test]
fn pass_waits_while_its_box_is_edited() {
    let (d, ids) = map("Teh claim", &["A wrod here"]);
    let mut h = harness(app_with_doc(d));
    apply(h.state_mut(), Action::Select(Some(Item::Box(ids[0]))));
    apply(h.state_mut(), Action::OpenSpelling);
    h.run();
    wait_dictionary(&mut h, "en");
    assert_eq!(current_word(&h).as_deref(), Some("Teh"));
    // Fix the word by hand in the inline editor.
    for action in [
        Action::Edit,
        Action::EditText {
            node: ids[0],
            text: "The claim".into(),
        },
    ] {
        h.state_mut().pending.push(action);
    }
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.editing(), Some(ids[0]));
    assert_eq!(
        tab.selection,
        Some(Item::Box(ids[0])),
        "the selection stays on the box being edited"
    );
    // The edit ends: the pass walks on to the next misspelling.
    apply(h.state_mut(), Action::StopEditing);
    h.run();
    assert_eq!(current_word(&h).as_deref(), Some("wrod"));
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(ids[1])));
}
