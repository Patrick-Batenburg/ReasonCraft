//! Starting the app: a document path handed over by the OS (as a
//! double-click does) and `--version`. The installers' checks are in the
//! `packaging` test binary.

use reasoncraft::ReasonCraftApp;

#[test]
fn cli_path_opens_file() {
    // The association passes the path quoted, as one argument; it may hold
    // spaces and non-ASCII letters.
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Mijn kaarten");
    std::fs::create_dir(&folder).unwrap();
    let path = folder.join("Één kaart.rcraft");
    let doc = reasoncraft_core::Document::new("Robots will take our jobs");
    reasoncraft_core::file::save(&doc, &path).unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    reasoncraft::file_ops::open_paths(&mut app, [path.clone().into_os_string()]);
    assert!(app.modal.is_none());
    let tab = app.tabs.iter().find(|t| t.path.as_deref() == Some(&path));
    let tab = tab.expect("the file opens in a tab");
    assert_eq!(
        tab.doc.boxes[&tab.doc.root].text,
        "Robots will take our jobs"
    );
}

#[test]
fn version_flag_prints_version() {
    use std::io::Read as _;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    // Should `--version` ever open the window, it does so on a throw-away
    // profile and is stopped, never touching the real settings.
    let profile = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_reasoncraft"))
        .arg("--version")
        .env(reasoncraft::profile::PROFILE_DIR_VAR, profile.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the app starts");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(60) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("`reasoncraft --version` did not exit (it opened the window?)");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    assert!(status.success(), "{status:?}");
    assert_eq!(
        stdout,
        format!("ReasonCraft {}\n", env!("CARGO_PKG_VERSION"))
    );
}

/// `--version` with nowhere to print (stdout closed, as when the
/// reader went away) exits quietly instead of panicking.
#[test]
fn version_flag_survives_a_closed_stdout() {
    use std::process::{Command, Stdio};

    let profile = tempfile::tempdir().unwrap();
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_reasoncraft"))
        .arg("--version")
        .env(reasoncraft::profile::PROFILE_DIR_VAR, profile.path())
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .expect("the app starts");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{:?}: {stderr}", output.status);
    assert!(!stderr.contains("panicked"), "{stderr}");
}
