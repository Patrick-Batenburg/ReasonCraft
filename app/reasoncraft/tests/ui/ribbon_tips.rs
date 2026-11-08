//! Ribbon tooltips: every command shows "label (shortcut)" as
//! Search commands writes it, a greyed-out add command also says why,
//! and an icon-only (compact) button keeps its tooltip.

use crate::common;

use eframe::egui;
use egui_kittest::kittest::{NodeT, Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, Item, Parent};

use crate::common::H;

/// Rests the pointer on the ribbon button `label` long enough for its
/// tooltip and returns the tooltip's painted text (the label, then a
/// shortcut or a reason); the same text is also the button's accessible
/// description.
fn tip(h: &mut H, label: &str) -> Option<String> {
    let at = button(h, label).rect().center();
    h.event(egui::Event::PointerMoved(at));
    for _ in 0..40 {
        h.step();
    }
    let painted = common::painted_texts(h)
        .into_iter()
        .map(|(text, _)| text)
        .find(|text| {
            text.strip_prefix(label)
                .is_some_and(|rest| rest.starts_with(" (") || rest.starts_with('\n'))
        })?;
    let description = button(h, label).accesskit_node().description();
    assert_eq!(description.as_deref(), Some(painted.as_str()), "{label}");
    Some(painted)
}

/// The ribbon's button `label` (the title bar repeats Undo and Redo
/// above it).
fn button<'a>(h: &'a H, label: &'a str) -> egui_kittest::Node<'a> {
    h.query_all_by_label(label)
        .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap_or_else(|| panic!("no {label:?}"))
}

#[test]
fn ribbon_commands_show_their_shortcut() {
    let mut h = common::harness_with_doc(Document::new("Claim"));
    for (key, want) in [
        ("action.paste", "Paste (Ctrl+V)"),
        ("action.cut", "Cut (Ctrl+X)"),
        ("action.undo", "Undo (Ctrl+Z)"),
        ("action.connect", "Move to… (Ctrl+Shift+K)"),
    ] {
        assert_eq!(tip(&mut h, &t(key)).as_deref(), Some(want));
    }
    h.state_mut().ribbon_tab = RibbonTab::View;
    h.run();
    let fit = t("action.zoom_fit");
    let shown = tip(&mut h, &fit).unwrap_or_default();
    assert!(shown.starts_with(&format!("{fit} (")), "{shown:?}");
}

#[test]
fn a_greyed_out_add_command_says_why() {
    // The claim is selected and has a reason: no co-premise (no card
    // selected) and no category (a claim with cards).
    let mut d = Document::new("Claim");
    let root = d.root;
    d.add_card(Parent::Box(root), CardKind::Reason, "r")
        .unwrap();
    let mut app = common::app_with_doc(d);
    app.tabs[0].selection = Some(Item::Box(root));
    let mut h = common::harness(app);
    let co = t("action.add_co_premise");
    let want = format!("{co} (Ctrl+Enter)\n{}", t("ribbon.why.add_co_premise"));
    assert_eq!(tip(&mut h, &co), Some(want));
    let shown = tip(&mut h, &t("action.add_category")).unwrap_or_default();
    let why = format!("\n{}", t("ribbon.why.claim_has_cards"));
    assert!(shown.ends_with(&why), "{shown:?}");
    // An enabled add command gives no reason.
    let shown = tip(&mut h, &t("action.add_reason")).unwrap_or_default();
    assert!(!shown.contains('\n'), "{shown:?}");
}

/// In a window too narrow for the Home tab, the right-hand groups drop
/// their small buttons' labels; Undo keeps its tooltip.
#[test]
fn a_compact_button_keeps_its_tooltip() {
    let label = t("action.undo");
    for width in (700..=1400).rev().step_by(25) {
        let app = ReasonCraftApp::new_for_test();
        let mut h = common::builder()
            .with_size(egui::vec2(width as f32, 900.0))
            .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
        h.run();
        h.run();
        let Some(undo) = h
            .query_all_by_label(&label)
            .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        else {
            continue; // its group is collapsed
        };
        if undo.rect().width() > 30.0 {
            continue; // labels still shown
        }
        let want = format!("{label} (Ctrl+Z)");
        assert_eq!(tip(&mut h, &label), Some(want), "at {width}");
        return;
    }
    panic!("no width shows Undo compact");
}
