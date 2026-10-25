//! The "+" button at the selected item's bottom-right corner and its menu
//! of things to add there.

use eframe::egui::{self, Button, Popup, Rect, Role, Sense, Stroke, Vec2, WidgetInfo, vec2};
use reasoncraft_core::{BoxKind, CardKind, Item, Layout, SourceType};

use crate::actions::{Action, connect_item, is_enabled};
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::shell::source_icons::source_atom;
use crate::theme::Tokens;

/// Diameter of the "+" button, in screen px.
const BUTTON_PX: f32 = 20.0;

/// The menu entries for the current selection: label key and action.
/// Entries that do not apply to this kind of item are left out; entries the
/// structure rules refuse are listed but disabled.
pub fn entries(tab: &DocTab) -> Vec<(&'static str, Action)> {
    let kind = match tab.selection {
        Some(Item::Box(b)) => tab.doc.boxes.get(&b).map(|n| n.kind.clone()),
        _ => None,
    };
    let premise = kind == Some(BoxKind::Premise);
    let grouping = matches!(kind, Some(BoxKind::Claim | BoxKind::Category));
    let mut out = vec![
        ("action.add_reason", Action::AddReason),
        ("action.add_objection", Action::AddObjection),
    ];
    if premise {
        out.push(("action.add_co_premise", Action::AddCoPremise));
    }
    if grouping {
        out.push(("action.add_category", Action::AddCategory));
    }
    out.push(("action.add_note", Action::AddNote));
    if premise {
        out.push((
            "plus.whole_card_reason",
            Action::TargetWholeCard(CardKind::Reason),
        ));
        out.push((
            "plus.whole_card_objection",
            Action::TargetWholeCard(CardKind::Objection),
        ));
    }
    if connect_item(tab).is_some() {
        out.push(("action.connect", Action::StartConnect));
    }
    out
}

/// The types in the "Source ▸" submenu: all of them when a claim or
/// premise is selected, otherwise none (no submenu).
pub fn source_entries(tab: &DocTab) -> Vec<SourceType> {
    let takes_sources = match tab.selection {
        Some(Item::Box(b)) => tab
            .doc
            .boxes
            .get(&b)
            .is_some_and(|n| matches!(n.kind, BoxKind::Claim | BoxKind::Premise)),
        _ => false,
    };
    if !takes_sources {
        return Vec::new();
    }
    SourceType::ALL.to_vec()
}

/// A menu entry for adding (or switching to) source type `st`.
pub fn source_button(st: &SourceType) -> Button<'static> {
    Button::new((source_atom(st, 16.0), t(&format!("source.{}", st.key()))))
}

/// Show the "+" button for the selection (not while editing) and, when it
/// has been clicked, its menu.
pub(super) fn show(
    ui: &mut egui::Ui,
    tab: &DocTab,
    layout: &Layout,
    tk: &Tokens,
    pending: &mut Vec<Action>,
) {
    if tab.editing.is_some() {
        return;
    }
    let world = match tab.selection {
        Some(Item::Box(b)) => layout.boxes.get(&b),
        Some(Item::Card(c)) => layout.cards.get(&c),
        None => None,
    };
    let Some(&world) = world else { return };
    let corner = tab.camera.world_rect_to_screen(world).right_bottom();
    let rect = Rect::from_center_size(corner, Vec2::splat(BUTTON_PX));
    let label = t("plus.open");
    let response = ui.interact(
        rect,
        ui.scope_id().with(("plus_menu", tab.selection)),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));

    let painter = ui.painter();
    let fill = if response.hovered() {
        tk.accent.lerp_to_gamma(tk.box_text, 0.25)
    } else {
        tk.accent
    };
    painter.circle(
        rect.center(),
        BUTTON_PX / 2.0,
        fill,
        Stroke::new(1.5, tk.canvas),
    );
    let arm = BUTTON_PX * 0.25;
    let plus = Stroke::new(2.0, tk.box_fill);
    let c = rect.center();
    painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], plus);
    painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], plus);
    let response = response.on_hover_text(&label);

    Popup::menu(&response).show(|ui| {
        for (key, action) in entries(tab) {
            let enabled = is_enabled(tab, &action);
            if ui.add_enabled(enabled, Button::new(t(key))).clicked() {
                pending.push(action);
            }
        }
        let sources = source_entries(tab);
        if !sources.is_empty() {
            ui.menu_button(t("plus.source"), |ui| {
                for st in sources {
                    let action = Action::AddSource(st.clone());
                    let enabled = is_enabled(tab, &action);
                    if ui.add_enabled(enabled, source_button(&st)).clicked() {
                        pending.push(action);
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use reasoncraft_core::{BoxKind, CardKind, Document, Item, Parent, SourceType};

    use super::source_entries;
    use crate::doc_tab::DocTab;

    #[test]
    fn source_submenu_for_premise_and_claim_only() {
        let mut d = Document::new("Claim");
        let root = d.root;
        let (card, premise) = d
            .add_card(Parent::Box(root), CardKind::Reason, "p")
            .unwrap();
        let note = d.add_box(Parent::Box(premise), BoxKind::Note, "n").unwrap();
        let mut tab = DocTab::from_document(d);
        for (item, offered) in [
            (Item::Box(root), true),
            (Item::Box(premise), true),
            (Item::Card(card), false),
            (Item::Box(note), false),
        ] {
            tab.selection = Some(item);
            let got = source_entries(&tab);
            if offered {
                assert_eq!(got, SourceType::ALL.to_vec());
            } else {
                assert!(got.is_empty(), "{item:?}");
            }
        }
    }
}
