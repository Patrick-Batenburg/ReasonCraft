//! APP-1 (owner decision D2): two launches of the built app with one
//! profile folder. The second hands its file to the first and exits; the
//! first opens it. Debug builds only: the first copy notes what it opens
//! in a file (`REASONCRAFT_DEBUG_OPENED_LOG`), as a test cannot look
//! inside its window. Where no window can open (a headless machine) the
//! test is skipped.
#![cfg(debug_assertions)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reasoncraft::instance::OPENED_LOG_VAR;
use reasoncraft::profile::PROFILE_DIR_VAR;

/// Kills the app it holds when dropped, also when the test fails.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn launch(profile: &Path, file: &Path, log: Option<&Path>) -> Running {
    let mut command = Command::new(env!("CARGO_BIN_EXE_reasoncraft"));
    command
        .arg(file)
        .env(PROFILE_DIR_VAR, profile)
        .env_remove(OPENED_LOG_VAR)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(log) = log {
        command.env(OPENED_LOG_VAR, log);
    }
    Running(command.spawn().expect("the app starts"))
}

fn opened(log: &Path, file: &Path) -> bool {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    text.lines().any(|line| Path::new(line) == file)
}

fn map(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let doc = reasoncraft_core::Document::new(name);
    reasoncraft_core::file::save(&doc, &path).unwrap();
    path
}

#[test]
fn a_second_launch_hands_its_file_to_the_first() {
    let profile = tempfile::tempdir().unwrap();
    let maps = tempfile::tempdir().unwrap();
    let (a, b) = (map(maps.path(), "a.rcraft"), map(maps.path(), "b.rcraft"));
    let log = profile.path().join("opened.log");

    let mut first = launch(profile.path(), &a, Some(&log));
    let start = Instant::now();
    while !opened(&log, &a) {
        if let Some(status) = first.0.try_wait().unwrap() {
            eprintln!("skipped: the app could not open a window here ({status})");
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "the first copy did not start"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    let mut second = launch(profile.path(), &b, None);
    let start = Instant::now();
    let status = loop {
        if let Some(status) = second.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the second launch did not exit (it opened a window of its own?)"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "{status:?}");

    let start = Instant::now();
    while !opened(&log, &b) {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "the first copy did not open b"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        first.0.try_wait().unwrap().is_none(),
        "the first copy still runs"
    );
}
