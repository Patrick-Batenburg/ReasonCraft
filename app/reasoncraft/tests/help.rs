//! UI tests for the offline help (the Text panel's Help tab, F1, the Help
//! ribbon tab) and the About page.

use eframe::egui::{self, Key, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::app::RibbonTab;
use reasoncraft::canvas::input::{Os, shortcut_text, shortcuts};
use reasoncraft::help::{self, HelpPage};
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;

type H = Harness<'static, ReasonCraftApp>;

fn harness_with(app: ReasonCraftApp) -> H {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

fn harness() -> H {
    harness_with(ReasonCraftApp::new_for_test())
}

fn act(h: &mut H, action: Action) {
    apply(h.state_mut(), action);
    h.run();
}

/// The accessible names of every node shown (a text's is its value).
fn labels(h: &H) -> Vec<String> {
    h.root()
        .children_recursive()
        .filter_map(|n| {
            let node = n.accesskit_node();
            node.label().or_else(|| node.value())
        })
        .collect()
}

fn click_button(h: &mut H, key: &str) {
    let label = t(key);
    h.get_by_role_and_label(Role::Button, &label).click();
    h.run();
}

fn shows_topic(h: &H, topic: &str) -> bool {
    h.state().settings.text_tab == TextTab::Help
        && h.state().right_panel
        && h.state().help.page == HelpPage::Topic(help::topic(topic).unwrap())
        && h.query_all_by_label(&help::title("en", topic))
            .any(|n| n.accesskit_node().role() == Role::Label)
}

#[test]
fn f1_opens_contextual_topic() {
    let mut h = harness();
    // The map has the keys: the general topic.
    h.state_mut().right_panel = false;
    h.key_press(Key::F1);
    h.run();
    assert!(shows_topic(&h, "getting-started"));
    // From a control in the Check or Essay tab: that tab's topic.
    for (tab, key, topic) in [
        (TextTab::Check, "text.tab.check", "checks"),
        (TextTab::Snippets, "text.tab.snippets", "snippets"),
        (TextTab::Outline, "text.tab.outline", "outline-and-essay"),
        (TextTab::Essay, "text.tab.essay", "outline-and-essay"),
    ] {
        act(&mut h, Action::SetTextTab(tab));
        h.get_by_role_and_label(Role::Tab, &t(key)).focus();
        h.run();
        h.key_press(Key::F1);
        h.run();
        assert!(shows_topic(&h, topic), "F1 from the {tab:?} tab");
    }
    // While a box is being edited F1 works too (and ends the edit).
    act(&mut h, Action::Edit);
    assert!(h.state().tabs[0].editing().is_some());
    h.key_press(Key::F1);
    h.run();
    assert!(shows_topic(&h, "getting-started"));
    assert!(h.state().tabs[0].editing().is_none());
    // Back returns to the topic shown before.
    click_button(&mut h, "help.back");
    assert!(shows_topic(&h, "outline-and-essay"));
    click_button(&mut h, "help.back");
    assert!(shows_topic(&h, "snippets"));
}

#[test]
fn topic_links_and_contents_navigate() {
    let mut h = harness();
    act(&mut h, Action::OpenHelp(HelpPage::Contents));
    assert_eq!(h.state().settings.text_tab, TextTab::Help);
    // The contents list every topic by its title.
    for topic in help::TOPICS {
        h.get_by_role_and_label(Role::Link, &help::title("en", topic));
    }
    h.get_by_role_and_label(Role::Link, &help::title("en", "co-premises"))
        .click();
    h.run();
    assert!(shows_topic(&h, "co-premises"));
}

#[test]
fn help_ribbon_tab_opens_help_and_about() {
    let mut h = harness();
    h.get_by_role_and_label(Role::Tab, &t("ribbon.tab.help"))
        .click();
    h.run();
    assert!(h.query_by_label(&t("ribbon.coming_soon")).is_none());
    click_button(&mut h, "help.cmd.topics");
    assert_eq!(h.state().help.page, HelpPage::Contents);
    click_button(&mut h, "help.cmd.keyboard");
    assert!(shows_topic(&h, "keyboard"));
    click_button(&mut h, "help.cmd.about");
    assert_eq!(h.state().ribbon_tab, RibbonTab::File);
    h.get_by_label(&format!("{} {}", t("app.name"), env!("CARGO_PKG_VERSION")));
}

/// The keyboard topic lists the keys of the platform the app runs on,
/// from the shortcut table itself.
#[test]
fn keyboard_topic_lists_live_shortcuts() {
    for os in [Os::Windows, Os::Mac] {
        let mut app = ReasonCraftApp::new_for_test();
        app.os = os;
        let mut h = harness_with(app);
        act(&mut h, Action::OpenHelp(HelpPage::Topic("keyboard")));
        assert!(shows_topic(&h, "keyboard"));
        let shown = labels(&h);
        for (s, _) in shortcuts(os) {
            let keys = shortcut_text(*s, os);
            assert!(
                shown.iter().any(|l| l.contains(&keys)),
                "{os:?}: {keys} is listed"
            );
        }
    }
    // F1 itself is in the table.
    let f1 = shortcuts(Os::Windows)
        .iter()
        .find(|(s, _)| s.key == Key::F1)
        .map(|(_, ctor)| ctor());
    assert_eq!(f1, Some(Action::ContextHelp));
}

#[test]
fn about_shows_third_party_notices() {
    let mut h = harness();
    act(&mut h, Action::ShowAbout);
    assert_eq!(h.state().ribbon_tab, RibbonTab::File);
    let shown = labels(&h);
    let has = |text: &str| shown.iter().any(|l| l.contains(text));
    assert!(has(&format!(
        "{} {}",
        t("app.name"),
        env!("CARGO_PKG_VERSION")
    )));
    assert!(has("MIT OR Apache-2.0"));
    // Every heading and every line of THIRD-PARTY.md shows.
    let notices = include_str!("../../../THIRD-PARTY.md");
    for line in notices.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let text = line.trim_start_matches(['#', '-', ' ']).replace('`', "");
        let start: String = text.chars().take(30).collect();
        assert!(has(&start), "About shows `{start}`");
    }
    // The dictionary projects are credited.
    assert!(has("OpenTaal") && has("SCOWL"));
}

/// At the panel's default width the five Text panel tabs fit on one line.
#[test]
fn text_tabs_fit_on_one_line() {
    let h = harness();
    let keys = [
        "text.tab.outline",
        "text.tab.essay",
        "text.tab.check",
        "text.tab.snippets",
    ];
    let mut tops: Vec<f32> = keys
        .iter()
        .map(|k| h.get_by_role_and_label(Role::Tab, &t(k)).rect().top())
        .collect();
    let help = reasoncraft::a11y::in_group(&t("action.panel_right"), &t("text.tab.help"));
    tops.push(h.get_by_role_and_label(Role::Tab, &help).rect().top());
    assert!(tops.iter().all(|y| *y == tops[0]), "{tops:?}");
}
