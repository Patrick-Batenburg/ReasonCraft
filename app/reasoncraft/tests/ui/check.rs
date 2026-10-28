//! UI tests for the Check tab of the Text panel and the status-bar issue
//! count.

use crate::common::{app_with_doc, build};
use eframe::egui::{self, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::settings::{Settings, TextTab};
use reasoncraft::{ReasonCraftApp, actions::Action};
use reasoncraft_core::checks::{CheckKind, CheckSet};
use reasoncraft_core::{BoxKind, CardKind, Document, Item, Parent, SourceType};

fn harness_with(doc: Document, tab: TextTab) -> Harness<'static, ReasonCraftApp> {
    let mut app = app_with_doc(doc);
    app.settings.text_tab = tab;
    build(app)
}

/// Claim "Robots help people" with one reason whose premise is backed by
/// a source, so the map has no issues.
fn clean_doc() -> Document {
    let mut d = Document::new("Robots help people");
    let (_, premise) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Robots help people")
        .unwrap();
    let source = BoxKind::Source {
        source_type: SourceType::Web,
    };
    d.add_box(Parent::Box(premise), source, "robots.example")
        .unwrap();
    d
}

fn chip(count: usize) -> String {
    let mut args = FluentArgs::new();
    args.set("count", count);
    t_args("status.issues", &args)
}

fn heading(kind: &str, count: usize) -> String {
    let mut args = FluentArgs::new();
    args.set("kind", t(&format!("check.kind.{kind}")));
    args.set("count", count);
    t_args("check.heading", &args)
}

fn has(h: &Harness<'static, ReasonCraftApp>, label: &str) -> bool {
    h.query_by_label(label).is_some()
}

#[test]
fn issues_refresh_after_undo() {
    let mut h = harness_with(clean_doc(), TextTab::Check);
    h.run();
    assert!(has(&h, &t("check.all_clear")));
    assert!(has(&h, &chip(0)));

    let root = h.state().tabs[0].doc.root;
    let premise = h.state_mut().tabs[0]
        .edit(|d| {
            let (_, p) = d.add_card(
                Parent::Box(root),
                CardKind::Reason,
                "Do robots help people?",
            )?;
            let source = BoxKind::Source {
                source_type: SourceType::Web,
            };
            d.add_box(Parent::Box(p), source, "robots.example")?;
            Ok(p)
        })
        .unwrap();
    h.run();
    let set = CheckSet::default();
    let issues = h.state_mut().tabs[0].issues(&set).to_vec();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].kind, CheckKind::Question);
    assert_eq!(issues[0].item, Item::Box(premise));
    assert!(has(&h, &heading("question", 1)));
    assert!(has(&h, &t("check.explain.question")));
    assert!(!has(&h, &t("check.all_clear")));
    assert!(has(&h, &chip(1)));

    h.state_mut().pending.push(Action::Undo);
    h.run();
    assert!(h.state_mut().tabs[0].issues(&set).is_empty());
    assert!(!has(&h, &heading("question", 1)));
    assert!(has(&h, &t("check.all_clear")));
    assert!(has(&h, &chip(0)));
    // The cache follows the document's revision.
    let tab = &h.state().tabs[0];
    assert_eq!(tab.issue_cache.as_ref().unwrap().0, tab.revision());
}

#[test]
fn issue_click_selects_item() {
    let mut d = clean_doc();
    let parked = d.add_loose(reasoncraft_core::NewLoose::Category, "Parked thought");
    let mut h = harness_with(d, TextTab::Check);
    h.run();
    assert!(has(&h, &heading("not_connected", 1)));
    assert_eq!(
        h.state().tabs[0].selection,
        Some(Item::Box(h.state().tabs[0].doc.root))
    );

    // Pan the map far away, so the click must bring the item back into view.
    h.state_mut().tabs[0].camera.offset = egui::vec2(-20_000.0, -20_000.0);
    let before = h.state().tabs[0].camera.offset;
    h.get_by_role_and_label(Role::ListItem, "Parked thought")
        .click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.selection, Some(parked));
    assert_ne!(tab.camera.offset, before, "the canvas revealed the item");
    assert!(tab.history.undo.is_empty(), "a check never edits the map");
}

#[test]
fn status_chip_opens_check_tab() {
    let mut d = clean_doc();
    d.add_loose(reasoncraft_core::NewLoose::Category, "Parked");
    let mut h = harness_with(d, TextTab::Outline);
    h.state_mut().right_panel = false;
    h.run();
    assert!(!has(&h, &t("check.all_clear")));
    h.get_by_role_and_label(Role::Button, &chip(1)).click();
    h.run();
    let app = h.state();
    assert_eq!(app.settings.text_tab, TextTab::Check);
    assert!(app.right_panel, "the Text panel opened");
    assert!(has(&h, &heading("not_connected", 1)));

    // Clicking again keeps the panel open (it shows, never toggles).
    h.get_by_role_and_label(Role::Button, &chip(1)).click();
    h.run();
    assert!(h.state().right_panel);
}

#[test]
fn disabled_check_hidden_in_tab_and_count() {
    let mut d = clean_doc();
    let root = d.root;
    let (_, p) = d
        .add_card(
            Parent::Box(root),
            CardKind::Reason,
            "Do robots help people?",
        )
        .unwrap();
    let source = BoxKind::Source {
        source_type: SourceType::Web,
    };
    d.add_box(Parent::Box(p), source, "robots.example").unwrap();
    d.add_loose(reasoncraft_core::NewLoose::Category, "Parked");
    let mut h = harness_with(d, TextTab::Check);
    h.run();
    assert!(has(&h, &heading("question", 1)));
    assert!(has(&h, &chip(2)));

    h.state_mut()
        .pending
        .push(Action::SetCheckEnabled(CheckKind::Question, false));
    h.run();
    assert!(!has(&h, &heading("question", 1)));
    assert!(has(&h, &heading("not_connected", 1)));
    assert!(has(&h, &chip(1)));
    let s = h.state().settings.clone();
    assert!(!s.checks.is_on(CheckKind::Question));
    // Turning a check off twice keeps one entry; on again removes it.
    h.state_mut()
        .pending
        .push(Action::SetCheckEnabled(CheckKind::Question, false));
    h.run();
    assert_eq!(h.state().settings.checks.disabled, [CheckKind::Question]);

    // The setting survives a save and restore; older settings run every check.
    let json = serde_json::to_string(&s).unwrap();
    let restored: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.checks, s.checks);
    let old: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(old.checks, CheckSet::default());

    // The backstage Settings page has one checkbox per check.
    h.state_mut().ribbon_tab = reasoncraft::app::RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    assert!(has(&h, &t("settings.checks")));
    for kind in CheckKind::ALL {
        let key = reasoncraft::shell::text_panel::check_key(kind);
        assert!(has(&h, &t(&format!("check.kind.{key}"))), "{key}");
    }
    h.get_by_role_and_label(Role::CheckBox, &t("check.kind.question"))
        .click();
    h.run();
    assert!(h.state().settings.checks.is_on(CheckKind::Question));
    assert!(has(&h, &chip(2)));
}

#[test]
fn every_check_has_text() {
    for kind in CheckKind::ALL {
        let key = reasoncraft::shell::text_panel::check_key(kind);
        for id in [
            format!("check.kind.{key}"),
            format!("check.help.{key}"),
            format!("check.explain.{key}"),
        ] {
            assert_ne!(t(&id), id, "missing ftl entry");
        }
    }
    for key in ["check.all_clear", "check.heading", "settings.checks"] {
        assert_ne!(t(key), key, "missing ftl entry");
    }
    // The count chip uses plural forms.
    assert_eq!(chip(1), "1 issue");
    assert_eq!(chip(3), "3 issues");
}

/// The step-3 review flow: click an issue, fix the item it selected, and
/// the issue goes away (the map ends up clean).
#[test]
fn resolve_issue_by_clicking_then_fixing() {
    let mut d = Document::new("Robots help people");
    let (_, premise) = d
        .add_card(
            Parent::Box(d.root),
            CardKind::Reason,
            "Robots help people lift heavy loads",
        )
        .unwrap();
    let mut h = harness_with(d, TextTab::Check);
    h.run();
    assert!(has(&h, &heading("unsupported", 1)));
    assert!(has(&h, &chip(1)));

    h.get_by_role_and_label(Role::ListItem, "Robots help people lift heavy loads")
        .click();
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(premise)));
    // With the premise selected, a source goes under it.
    for a in [
        Action::AddSource(SourceType::Statistic),
        Action::TypeText("Robot use survey 2024".into()),
        Action::StopEditing,
    ] {
        h.state_mut().pending.push(a);
    }
    h.run();
    assert!(has(&h, &t("check.all_clear")));
    assert!(has(&h, &chip(0)));
}

/// Unrated items are one row; clicking it goes to the first of them.
#[test]
fn incomplete_evaluation_is_one_row_that_goes_to_the_next() {
    let mut d = clean_doc();
    d.set_acceptance(d.root, Some(reasoncraft_core::Acceptance::Accept))
        .unwrap();
    let card = d.boxes[&d.root].children[0];
    let mut h = harness_with(d, TextTab::Check);
    h.run();
    let issues = h.state_mut().tabs[0].issues(&CheckSet::default()).to_vec();
    let incomplete: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == CheckKind::IncompleteEvaluation)
        .collect();
    assert_eq!(incomplete.len(), 1, "{issues:?}");
    assert_eq!(incomplete[0].item, card);
    // The card, its premise and the source.
    assert!(has(&h, &heading("incomplete_evaluation", 3)));
    let mut args = FluentArgs::new();
    args.set("count", 3);
    assert!(has(
        &h,
        &t_args("check.explain.incomplete_evaluation", &args)
    ));

    let row = reasoncraft::shell::text_panel::item_text(&h.state().tabs[0].doc, card);
    h.get_by_role_and_label(Role::ListItem, &row).click();
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(card));
}

/// The accessible name of the Ignore button of `item`'s "not connected"
/// hint.
fn ignore_name(item: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("item", item);
    args.set("kind", t("check.kind.not_connected"));
    t_args("check.ignore_name", &args)
}

#[test]
fn ignore_button_hides_hint_and_footer_restores_it() {
    let mut d = clean_doc();
    d.add_loose(reasoncraft_core::NewLoose::Category, "Parked thought");
    let mut h = harness_with(d, TextTab::Check);
    h.run();
    assert!(has(&h, &heading("not_connected", 1)));

    h.get_by_role_and_label(Role::Button, &ignore_name("Parked thought"))
        .click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.ignored_hints.len(), 1);
    // The button takes its own click: the row did not select its item.
    assert_eq!(tab.selection, Some(Item::Box(tab.doc.root)));
    assert!(has(&h, &t("check.all_clear")));
    let mut args = FluentArgs::new();
    args.set("count", 1);
    let restore = t_args("check.restore", &args);
    h.get_by_role_and_label(Role::Button, &restore).click();
    h.run();
    assert!(h.state().tabs[0].doc.ignored_hints.is_empty());
    assert!(has(&h, &heading("not_connected", 1)));
}

#[test]
fn rows_do_not_overlap() {
    let mut d = clean_doc();
    d.add_loose(reasoncraft_core::NewLoose::Category, "First parked");
    d.add_loose(reasoncraft_core::NewLoose::Category, "Second parked");
    d.add_loose(reasoncraft_core::NewLoose::Category, "Third parked");
    let mut h = harness_with(d, TextTab::Check);
    h.run();
    let mut rows: Vec<_> = ["First parked", "Second parked", "Third parked"]
        .iter()
        .map(|l| h.get_by_role_and_label(Role::ListItem, l).rect())
        .collect();
    rows.sort_by(|a, b| a.top().total_cmp(&b.top()));
    for pair in rows.windows(2) {
        assert!(pair[1].top() >= pair[0].bottom() - 0.5, "{pair:?}");
    }
    // The footer does not overlap the last row either.
    h.get_by_role_and_label(Role::Button, &ignore_name("First parked"))
        .click();
    h.run();
    let mut args = FluentArgs::new();
    args.set("count", 1);
    let footer = h
        .get_by_role_and_label(Role::Button, &t_args("check.restore", &args))
        .rect();
    let last = h
        .get_all_by_role(Role::ListItem)
        .map(|n| n.rect().bottom())
        .fold(0.0_f32, f32::max);
    assert!(footer.top() >= last - 0.5, "{footer:?} vs {last}");
}
