//! Example maps, templates (File → New, the building panel, Help) and the
//! recent-files list.

use crate::common::harness;
use std::path::{Path, PathBuf};

use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::app::RibbonTab;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::recovery;
use reasoncraft::settings::{MAX_RECENT, Settings};
use reasoncraft_core::Document;
use reasoncraft_core::examples::{Example, Template, example, template};

use crate::common::H;

fn root_text(doc: &Document) -> String {
    doc.boxes[&doc.root].text.clone()
}

/// A tab with work in it, so nothing opens in its place.
fn worked_on_tab() -> DocTab {
    DocTab::from_document(Document::new("My own claim"))
}

/// The tab is new, without a file, unsaved and has nothing to lose; it is
/// named by the map's title (an example), else "Untitled" (a template).
fn assert_clean_untitled(tab: &DocTab) {
    assert_eq!(tab.path, None);
    let name = match tab.doc.meta.title.trim() {
        "" => t("untitled"),
        title => title.to_owned(),
    };
    assert_eq!(tab.title(), name);
    assert!(!tab.is_dirty());
    assert!(!tab.has_unsaved_work());
    assert!(tab.history.undo.is_empty());
}

#[test]
fn opening_example_creates_untitled_clean_tab() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = ReasonCraftApp::new_for_test_with_recovery(dir.path().to_owned());
    // The untouched start tab makes way, like for File → Open.
    apply(&mut app, Action::OpenExample(Example::Reasoning));
    assert_eq!(app.tabs.len(), 1);
    let tab = app.active_tab();
    assert_clean_untitled(tab);
    assert_eq!(tab.doc.meta.language, "en");
    assert_eq!(
        root_text(&tab.doc),
        root_text(&example(Example::Reasoning, "en"))
    );
    // An example nobody touched stays: the next one gets a tab of its own.
    apply(&mut app, Action::OpenExample(Example::Grouping));
    assert_eq!((app.tabs.len(), app.active), (2, 1));
    assert_clean_untitled(app.active_tab());
    // Autosave leaves clean examples alone.
    recovery::tick(&mut app, 0.0);
    recovery::tick(&mut app, 1000.0);
    app.recovery.as_ref().unwrap().wait_idle();
    assert!(app.tabs.iter().all(|t| t.autosaved_revision.is_none()));
    let session = app.recovery.as_ref().unwrap().session_dir();
    let copies = std::fs::read_dir(session).map_or(0, |d| {
        let copy = |e: &std::fs::DirEntry| e.path().extension().is_some_and(|x| x == "rcraft");
        d.flatten().filter(copy).count()
    });
    assert_eq!(copies, 0, "no recovery copies");
    // Closing asks nothing.
    apply(&mut app, Action::CloseTab(1));
    assert!(app.modal.is_none());
    assert_eq!(app.tabs.len(), 1);
}

#[test]
fn building_panel_templates_insert_as_new_tab() {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = worked_on_tab();
    let mut h = harness(app);
    let mut args = FluentArgs::new();
    args.set("item", t("template.essay_plan"));
    let label = t_args("building.item_name", &args);
    h.get_by_label(&label).scroll_to_me();
    h.run();
    h.get_by_label(&label).click();
    h.run();
    let app = h.state();
    assert_eq!((app.tabs.len(), app.active), (2, 1));
    assert_eq!(root_text(&app.tabs[0].doc), "My own claim");
    let tab = app.active_tab();
    assert_clean_untitled(tab);
    let want = template(Template::EssayPlan, "en");
    assert_eq!(root_text(&tab.doc), root_text(&want));
    assert_eq!(tab.doc.cards.len(), want.cards.len());
}

/// File → New shows a blank map, the templates and the examples.
#[test]
fn file_new_page_offers_blank_templates_and_examples() {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = worked_on_tab();
    let mut h = harness(app);
    let open_new_page = |h: &mut H| {
        h.get_by_label(&t("ribbon.tab.file")).click();
        h.run();
        h.get_by_label(&t("action.new")).click();
        h.run();
    };
    open_new_page(&mut h);
    h.get_by_label(&t("template.peer_review")).click();
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home, "back to the map");
    assert_eq!(h.state().tabs.len(), 2);
    let want = template(Template::PeerReview, "en");
    assert_eq!(root_text(&h.state().active_tab().doc), root_text(&want));
    open_new_page(&mut h);
    h.get_by_label(&t("example.grouping")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 3);
    let want = example(Example::Grouping, "en");
    assert_eq!(h.state().active_tab().doc.meta.title, want.meta.title);
    open_new_page(&mut h);
    h.get_by_label(&t("backstage.new.blank")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 4);
    assert_eq!(root_text(&h.state().active_tab().doc), "");
}

/// Help → Example maps opens either example.
#[test]
fn help_ribbon_opens_examples() {
    let mut app = ReasonCraftApp::new_for_test();
    app.ribbon_tab = RibbonTab::Help;
    let mut h = harness(app);
    h.get_by_label(&t("example.reasoning")).click();
    h.run();
    let want = example(Example::Reasoning, "en");
    assert_eq!(h.state().active_tab().doc.cards.len(), want.cards.len());
}

#[test]
fn recent_files_capped_and_deduplicated() {
    let mut s = Settings::default();
    // Absolute paths are kept as they are.
    let maps = std::env::temp_dir().join("maps");
    let path = |i: usize| maps.join(format!("{i}.rcraft"));
    for i in 0..MAX_RECENT + 2 {
        s.push_recent(&path(i));
    }
    assert_eq!(s.recent_files.len(), MAX_RECENT);
    assert_eq!(s.recent_files[0], path(MAX_RECENT + 1), "newest first");
    assert!(!s.recent_files.contains(&path(0)), "the oldest go");
    s.push_recent(&path(5));
    assert_eq!(s.recent_files[0], path(5));
    assert_eq!(s.recent_files.len(), MAX_RECENT, "no duplicates");
    assert_eq!(s.recent_files.iter().filter(|p| **p == path(5)).count(), 1);
    // Another spelling of a file already listed replaces it.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    let file = dir.path().join("same.rcraft");
    std::fs::write(&file, b"x").unwrap();
    s.push_recent(&file);
    s.push_recent(&dir.path().join("sub").join("..").join("same.rcraft"));
    let same = |p: &PathBuf| p.file_name() == file.file_name();
    assert_eq!(s.recent_files.iter().filter(|p| same(p)).count(), 1);
    // Stored resolved, in the spelling people know (no `\\?\` on Windows).
    let stored = s.recent_files[0].to_string_lossy().into_owned();
    assert!(!stored.starts_with(r"\\?\"), "{stored}");
    assert!(!stored.contains(".."), "{stored}");
    s.push_recent(&file);
    assert_eq!(s.recent_files.iter().filter(|p| same(p)).count(), 1);
    // A path the settings cannot store (not UTF-8) is left out, and one
    // put in the list anyway is not written: the rest still saves.
    let odd = non_utf8_path();
    s.push_recent(&odd);
    assert!(!s.recent_files.contains(&odd));
    let mut written = s.clone();
    written.recent_files.insert(0, odd);
    let mut storage = MemStorage::default();
    eframe::set_value(&mut storage, reasoncraft::settings::SETTINGS_KEY, &written);
    let back: Settings = eframe::get_value(&storage, reasoncraft::settings::SETTINGS_KEY)
        .expect("the settings were written");
    assert_eq!(back.recent_files, s.recent_files);
    // A relative path (from the command line) is kept as an absolute one.
    s.push_recent(Path::new("relative.rcraft"));
    assert!(s.recent_files[0].is_absolute());

    // A list of another shape (a hand-edited file) is dropped alone.
    let s: Settings = serde_json::from_str(r#"{"recent_files":5,"author":"Ann"}"#).unwrap();
    assert!(s.recent_files.is_empty());
    assert_eq!(s.author, "Ann");

    // Saving and opening put the file first.
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.tabs[0] = worked_on_tab();
    let saved = dir.path().join("a.rcraft");
    fake.set_save(saved.clone());
    apply(&mut app, Action::Save);
    assert_eq!(app.settings.recent_files, vec![saved.clone()]);
    let other = dir.path().join("b.rcraft");
    reasoncraft_core::file::save(&Document::new("B"), &other).unwrap();
    fake.set_open(other.clone());
    apply(&mut app, Action::Open);
    assert_eq!(app.settings.recent_files, vec![other, saved]);
}

/// A file name that is not valid UTF-8.
fn non_utf8_path() -> PathBuf {
    #[cfg(windows)]
    let name = {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&[0xD800])
    };
    #[cfg(unix)]
    let name = {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(vec![0xff])
    };
    std::env::temp_dir().join(name)
}

/// eframe storage held in memory.
#[derive(Default)]
struct MemStorage(std::collections::HashMap<String, String>);

impl eframe::Storage for MemStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

/// The backstage Recent page names each file with its folder.
fn recent_label(path: &Path) -> String {
    let mut args = FluentArgs::new();
    args.set(
        "name",
        path.file_name().unwrap().to_string_lossy().into_owned(),
    );
    args.set("folder", path.parent().unwrap().display().to_string());
    t_args("backstage.recent.item", &args)
}

#[test]
fn missing_recent_file_removed_with_error() {
    let dir = tempfile::tempdir().unwrap();
    let present = dir.path().join("here.rcraft");
    reasoncraft_core::file::save(&Document::new("Here"), &present).unwrap();
    let missing = dir.path().join("gone.rcraft");
    let mut app = ReasonCraftApp::new_for_test();
    app.settings.recent_files = vec![missing.clone(), present.clone()];
    let mut h = harness(app);
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.recent")).click();
    h.run();
    h.get_by_label(&recent_label(&missing)).click();
    h.run();
    let mut args = FluentArgs::new();
    args.set("file", "gone.rcraft");
    let title = t_args("error.open.title", &args);
    assert!(
        matches!(&h.state().modal, Some(Modal::Error { title: t, .. }) if *t == title),
        "{:?}",
        h.state().modal
    );
    assert_eq!(h.state().settings.recent_files, vec![present.clone()]);
    assert_eq!(h.state().tabs.len(), 1);
    // A file that is there opens.
    apply(h.state_mut(), Action::DismissModal);
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.recent")).click();
    h.run();
    h.get_by_label(&recent_label(&present)).click();
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
    assert_eq!(
        h.state().active_tab().path.as_deref(),
        Some(present.as_path())
    );
}
