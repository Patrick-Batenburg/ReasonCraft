//! The single-instance channel, over the platform's real transport (each
//! test with a scope of its own), and the window taking what arrives.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use super::*;
use crate::actions::apply;
use reasoncraft_core::Document;

/// A scope no other test (or running app) uses.
pub(super) fn unique_scope() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("test-{}-{n}", std::process::id())
}

const QUICK: Timeouts = Timeouts {
    forward: Duration::from_secs(5),
    answer: Duration::from_secs(3),
};

/// The first copy for a fresh scope, listening.
fn first(scope: &str) -> Inbox {
    match claim(scope, &[], QUICK) {
        Claim::First(listener) => listener.start(QUICK),
        _ => panic!("the first launch is not the first copy"),
    }
}

/// A later launch with `paths`, on its own thread (it waits for the
/// answer).
fn launch(scope: &str, paths: Vec<PathBuf>, timeouts: Timeouts) -> thread::JoinHandle<Claim> {
    let scope = scope.to_owned();
    thread::spawn(move || claim(&scope, &paths, timeouts))
}

/// The next handover arriving at `inbox`.
fn next(inbox: &Inbox) -> Handover {
    inbox
        .rx
        .recv_timeout(Duration::from_secs(10))
        .expect("a handover arrives")
}

fn file(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

#[test]
fn scopes_follow_the_settings_file() {
    let a = scope(Some(Path::new("/profiles/a/app.ron")));
    assert_eq!(a, scope(Some(Path::new("/profiles/a/app.ron"))));
    assert_ne!(a, scope(Some(Path::new("/profiles/b/app.ron"))));
    assert_eq!(scope(None), "default");
    assert!(a.chars().all(|c| c.is_ascii_alphanumeric()), "{a}");
}

#[test]
fn a_later_launch_hands_its_files_over_and_exits() {
    let scope = unique_scope();
    let inbox = first(&scope);
    let paths = vec![file("één kaart.rcraft"), file("b.rcraft")];
    let later = launch(&scope, paths.clone(), QUICK);
    let handover = next(&inbox);
    assert_eq!(handover.paths, paths);
    handover.taken.send(()).unwrap();
    assert!(matches!(later.join().unwrap(), Claim::Forwarded));
    // Without files too: the window just comes to the front.
    let later = launch(&scope, Vec::new(), QUICK);
    let handover = next(&inbox);
    assert!(handover.paths.is_empty());
    handover.taken.send(()).unwrap();
    assert!(matches!(later.join().unwrap(), Claim::Forwarded));
    platform::forget(&scope);
}

#[test]
fn another_profile_is_another_copy() {
    let (a, b) = (unique_scope(), unique_scope());
    let _inbox = first(&a);
    assert!(matches!(claim(&b, &[], QUICK), Claim::First(_)));
    platform::forget(&a);
    platform::forget(&b);
}

/// A first copy whose window does not answer (hung, or still starting):
/// the launch starts its own window, after the first copy's timeout.
#[test]
fn a_launch_does_not_wait_for_a_hung_copy() {
    let scope = unique_scope();
    let short = Timeouts {
        forward: Duration::from_secs(3),
        answer: Duration::from_millis(300),
    };
    let inbox = match claim(&scope, &[], short) {
        Claim::First(listener) => listener.start(short),
        _ => panic!("not the first copy"),
    };
    let start = Instant::now();
    let later = launch(&scope, vec![file("a.rcraft")], short);
    assert!(matches!(later.join().unwrap(), Claim::Alone));
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "{:?}",
        start.elapsed()
    );
    // The window looks later: the launch gave up, so it takes nothing.
    let handover = next(&inbox);
    assert!(handover.taken.send(()).is_err());
    platform::forget(&scope);
}

/// Once the first copy is gone (here: closed before listening), the next
/// launch becomes the first copy.
#[test]
fn the_next_launch_takes_over_after_the_first_copy_ends() {
    let scope = unique_scope();
    match claim(&scope, &[], QUICK) {
        Claim::First(listener) => drop(listener),
        _ => panic!("not the first copy"),
    }
    assert!(matches!(claim(&scope, &[], QUICK), Claim::First(_)));
    platform::forget(&scope);
}

/// A launch whose window is gone gets no answer, and runs on its own.
#[test]
fn a_closed_window_answers_no() {
    let scope = unique_scope();
    let inbox = first(&scope);
    drop(inbox);
    let later = launch(&scope, vec![file("a.rcraft")], QUICK);
    assert!(matches!(later.join().unwrap(), Claim::Alone));
    platform::forget(&scope);
}

/// Garbage on the channel is dropped; the next launch still gets through.
#[test]
fn a_malformed_message_is_dropped() {
    use std::io::{Read as _, Write as _};
    let scope = unique_scope();
    let inbox = first(&scope);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut conn = platform::connect(&scope, deadline).unwrap();
    platform::set_timeouts(&conn, Duration::from_secs(5));
    conn.write_all(b"RCRAFT\x00\x01\xff\xff\xff\xff").unwrap();
    let mut answer = Vec::new();
    let _ = conn.read_to_end(&mut answer);
    assert!(answer.is_empty(), "{answer:?}");
    drop(conn);
    let later = launch(&scope, vec![file("a.rcraft")], QUICK);
    next(&inbox).taken.send(()).unwrap();
    assert!(matches!(later.join().unwrap(), Claim::Forwarded));
    platform::forget(&scope);
}

/// An app whose inbox is fed by the returned sender, as the listener does.
fn app_with_inbox() -> (ReasonCraftApp, Sender<Handover>) {
    let (tx, rx) = mpsc::channel();
    let mut app = ReasonCraftApp::new_for_test();
    app.incoming.inbox = Some(Inbox {
        rx,
        waker: Arc::default(),
    });
    (app, tx)
}

/// Hand `paths` over to `app` as a launch would; whether it was taken.
fn hand_over(app: &mut ReasonCraftApp, tx: &Sender<Handover>, paths: Vec<PathBuf>) -> bool {
    let (taken, was_taken) = mpsc::sync_channel(1);
    tx.send(Handover { paths, taken }).unwrap();
    poll(app);
    for action in std::mem::take(&mut app.pending) {
        apply(app, action);
    }
    was_taken.try_recv().is_ok()
}

fn saved(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    reasoncraft_core::file::save(&Document::new(text), &path).unwrap();
    path
}

#[test]
fn handed_over_files_open_in_tabs_and_an_open_one_is_activated() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (
        saved(dir.path(), "a.rcraft", "A"),
        saved(dir.path(), "b.rcraft", "B"),
    );
    let (mut app, tx) = app_with_inbox();
    assert!(hand_over(&mut app, &tx, vec![a.clone(), b.clone()]));
    assert_eq!(app.tabs.len(), 2, "a into the blank tab, b into a new one");
    assert_eq!(app.active_tab().path.as_deref(), Some(b.as_path()));
    assert!(hand_over(&mut app, &tx, vec![a.clone()]));
    assert_eq!(app.tabs.len(), 2, "an open file is not opened twice");
    assert_eq!(app.active_tab().path.as_deref(), Some(a.as_path()));
}

#[test]
fn handed_over_files_wait_for_an_open_dialog() {
    let dir = tempfile::tempdir().unwrap();
    let a = saved(dir.path(), "a.rcraft", "A");
    let (mut app, tx) = app_with_inbox();
    app.ribbon_tab = crate::app::RibbonTab::File;
    app.modal = Some(crate::file_ops::Modal::ResetSettings);
    assert!(hand_over(&mut app, &tx, vec![a.clone()]), "taken at once");
    assert_eq!(app.tabs[0].path, None, "opened behind the dialog");
    app.modal = None;
    poll(&mut app);
    for action in std::mem::take(&mut app.pending) {
        apply(&mut app, action);
    }
    assert_eq!(app.active_tab().path.as_deref(), Some(a.as_path()));
    assert_eq!(app.ribbon_tab, crate::app::RibbonTab::Home, "the map shows");
}

#[test]
fn a_handed_over_file_that_fails_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, tx) = app_with_inbox();
    assert!(hand_over(
        &mut app,
        &tx,
        vec![dir.path().join("gone.rcraft")]
    ));
    assert!(matches!(
        app.modal,
        Some(crate::file_ops::Modal::Error { .. })
    ));
}

/// Start closing the window of `app` with one tab of unsaved work: the
/// unsaved-changes prompt shows.
fn closing_with_unsaved_work(app: &mut ReasonCraftApp) {
    let root = app.active_tab().doc.root;
    apply(app, Action::Edit);
    apply(app, Action::EditText {
        node: root,
        text: "unsaved".into(),
    });
    apply(app, Action::StopEditing);
    apply(app, Action::CloseWindow);
    assert!(matches!(
        app.modal,
        Some(crate::file_ops::Modal::Unsaved { window: true, .. })
    ));
}

#[test]
fn a_closing_window_takes_no_files() {
    use crate::actions::UnsavedChoice;
    let dir = tempfile::tempdir().unwrap();
    let a = saved(dir.path(), "a.rcraft", "A");
    let (mut app, tx) = app_with_inbox();
    closing_with_unsaved_work(&mut app);
    assert!(
        !hand_over(&mut app, &tx, vec![a.clone()]),
        "the launch opens its own window instead"
    );
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Discard));
    assert!(app.close_confirmed());
    // Waiting for running exports is still part of the close.
    let (mut app, tx) = app_with_inbox();
    app.modal = Some(crate::file_ops::Modal::ExportsRunning);
    assert!(!hand_over(&mut app, &tx, vec![a]));
}

#[test]
fn files_handed_over_during_a_cancelled_close_open_after_it() {
    use crate::actions::UnsavedChoice;
    let dir = tempfile::tempdir().unwrap();
    let a = saved(dir.path(), "a.rcraft", "A");
    let (mut app, tx) = app_with_inbox();
    closing_with_unsaved_work(&mut app);
    let (taken, was_taken) = mpsc::sync_channel(1);
    tx.send(Handover {
        paths: vec![a.clone()],
        taken,
    })
    .unwrap();
    poll(&mut app);
    assert!(was_taken.try_recv().is_err(), "not taken while closing");
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Cancel));
    poll(&mut app);
    for action in std::mem::take(&mut app.pending) {
        apply(&mut app, action);
    }
    assert!(was_taken.try_recv().is_ok(), "taken once the close is off");
    assert!(app.tabs.iter().any(|t| t.path.as_deref() == Some(a.as_path())));
}
