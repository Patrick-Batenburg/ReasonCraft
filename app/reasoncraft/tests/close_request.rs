//! Closing the window from outside the app (the window's close button, the
//! OS, or Cmd+Q on macOS) asks about unsaved changes first.

use eframe::egui::{self, Key, Modifiers, ViewportEvent, ViewportId};
use egui_kittest::Harness;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::canvas::input::Os;
use reasoncraft::file_ops::Modal;
use reasoncraft_core::Item;

type H = Harness<'static, ReasonCraftApp>;

/// An app on `os` whose one tab has unsaved changes (when `dirty`).
fn harness(os: Os, dirty: bool) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.os = os;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    if dirty {
        let root = h.state().tabs[0].doc.root;
        h.state_mut().tabs[0].selection = Some(Item::Box(root));
        h.state_mut().push(Action::AddReason);
        h.run();
        h.state_mut().push(Action::StopEditing);
        h.run();
    }
    assert_eq!(h.state().tabs[0].has_unsaved_work(), dirty);
    h
}

/// The OS asks the window to close, as eframe reports it.
fn request_close(h: &mut H) {
    h.input_mut()
        .viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .events
        .push(ViewportEvent::Close);
    h.step();
}

fn window_prompt(app: &ReasonCraftApp) -> bool {
    matches!(
        app.modal,
        Some(Modal::Unsaved {
            tab: 0,
            window: true,
            ..
        })
    )
}

#[test]
fn close_request_prompts_when_unsaved() {
    let mut h = harness(Os::Windows, true);
    request_close(&mut h);
    assert!(window_prompt(h.state()), "{:?}", h.state().modal);
    assert!(!h.state().close_confirmed());
    // Nothing unsaved: the window closes without a question.
    let mut h = harness(Os::Windows, false);
    request_close(&mut h);
    assert!(h.state().modal.is_none());
    assert!(h.state().close_confirmed());
}

/// On macOS Cmd+Q closes the window the same way; elsewhere Ctrl+Q does
/// nothing.
#[test]
fn cmd_q_prompts_when_unsaved_on_mac() {
    let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
    let mut h = harness(Os::Mac, true);
    h.key_press_modifiers(cmd, Key::Q);
    h.run();
    assert!(window_prompt(h.state()), "{:?}", h.state().modal);
    assert!(!h.state().close_confirmed());

    let mut h = harness(Os::Mac, false);
    h.key_press_modifiers(cmd, Key::Q);
    h.run();
    assert!(h.state().close_confirmed());

    let mut h = harness(Os::Windows, true);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Q);
    h.run();
    assert!(h.state().modal.is_none());
    assert!(!h.state().close_confirmed());
}
