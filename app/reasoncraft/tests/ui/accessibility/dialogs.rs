//! Dialogs take the keyboard focus on their default button, are named
//! after their heading, and Enter answers with the default (ROB-7).

use super::*;

/// The role and name of the control with the keyboard focus.
fn focused(h: &H) -> Option<(accesskit::Role, String)> {
    h.root()
        .children_recursive()
        .map(|n| n.accesskit_node())
        .find(|n| n.is_focused())
        .map(|n| (n.role(), name(&n)))
}

/// Whether a dialog named `heading` is in the tree.
fn dialog_named(h: &H, heading: &str) -> bool {
    h.query_by_role_and_label(accesskit::Role::Dialog, heading)
        .is_some()
}

/// "Reset all settings?" focuses Cancel (the safe answer); Enter takes it.
#[test]
fn reset_prompt_focuses_cancel_and_enter_answers() {
    let (mut h, _) = sample_harness();
    h.state_mut().settings.show_numbers = !Settings::default().show_numbers;
    act(&mut h, Action::ResetSettings);
    assert!(dialog_named(&h, &t("settings.reset.title")));
    assert_eq!(
        focused(&h),
        Some((accesskit::Role::Button, t("settings.reset.cancel")))
    );
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().modal.is_none(), "Enter answered");
    assert_ne!(
        h.state().settings.show_numbers,
        Settings::default().show_numbers,
        "Cancel kept the settings"
    );
}

/// "Save changes?" focuses Save; Tab moves on to the other buttons,
/// and Enter presses the one with the focus.
#[test]
fn unsaved_prompt_focuses_save() {
    let (mut h, reason) = sample_harness();
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::Delete);
    act(&mut h, Action::CloseTab(0));
    let mut args = FluentArgs::new();
    args.set("name", h.state().tabs[0].name());
    assert!(dialog_named(&h, &t_args("unsaved.title", &args)));
    assert_eq!(
        focused(&h),
        Some((accesskit::Role::Button, t("unsaved.save_changes")))
    );
    h.key_press(Key::Tab);
    h.run();
    let other = focused(&h).expect("a button keeps the focus").1;
    assert_ne!(other, t("unsaved.save_changes"));
    assert!(h.state().modal.is_some(), "Tab does not answer");
}

/// An error message focuses OK, and Enter closes it.
#[test]
fn error_focuses_ok() {
    let (mut h, _) = sample_harness();
    h.state_mut().modal = Some(Modal::Error {
        title: "Could not open".into(),
        message: "Gone".into(),
    });
    h.run();
    assert!(dialog_named(&h, "Could not open"));
    assert_eq!(focused(&h), Some((accesskit::Role::Button, t("modal.ok"))));
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().modal.is_none());
}

/// Closing the window asks about two "Untitled" tabs in turn: the second
/// prompt has the same heading, but is a new dialog, so Save takes the
/// focus again (Enter must not discard it too).
#[test]
fn next_unsaved_prompt_with_the_same_heading_focuses_save_again() {
    let mut app = ReasonCraftApp::new_for_test();
    let mut tab = DocTab::new();
    tab.edit(|d| d.set_text(d.root, "One")).unwrap();
    app.tabs[0] = tab;
    let mut tab = DocTab::new();
    tab.edit(|d| d.set_text(d.root, "Two")).unwrap();
    app.tabs.push(tab);
    let mut h = harness(app);
    act(&mut h, Action::CloseWindow);
    let save = Some((accesskit::Role::Button, t("unsaved.save_changes")));
    assert_eq!(focused(&h), save);
    let discard = Some((accesskit::Role::Button, t("unsaved.discard")));
    for _ in 0..3 {
        if focused(&h) == discard {
            break;
        }
        h.key_press(Key::Tab);
        h.run();
    }
    assert_eq!(focused(&h), discard);
    h.key_press(Key::Enter);
    h.run();
    let key = h.state().tabs[1].recovery_key;
    assert!(matches!(h.state().modal, Some(Modal::Unsaved { tab, .. }) if tab == key));
    assert_eq!(focused(&h), save, "the second prompt focuses Save");
}
