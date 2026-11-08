//! One running copy per user and profile (owner decision D2, ruling
//! RR-2): a later launch hands its files to the running copy, which opens
//! them in tabs (an open one is only brought forward) and comes to the
//! front where the system allows it, and the launch exits.
//!
//! The first copy listens on a channel only its own user can reach (a
//! named pipe on Windows, a Unix socket elsewhere; see the platform
//! modules), scoped by the settings file, so copies with another profile
//! folder (`REASONCRAFT_PROFILE_DIR`) stay independent. A launch that
//! cannot hand over within [`TIMEOUTS`] (the first copy hangs, or is
//! closing) starts a window of its own, as one without a channel does.
//! Nothing here runs on the UI thread but [`poll`], which never waits.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::actions::Action;
use crate::app::ReasonCraftApp;

mod protocol;
#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

/// How long a handover may take.
#[derive(Clone, Copy, Debug)]
pub struct Timeouts {
    /// A launch gives up handing over after this (then starts its own
    /// window).
    pub forward: Duration,
    /// The first copy waits this long for its window to take the files;
    /// shorter than `forward`, so a launch hears "no" before it gives up.
    pub answer: Duration,
}

/// The timeouts the app uses.
pub const TIMEOUTS: Timeouts = Timeouts {
    forward: Duration::from_secs(5),
    answer: Duration::from_secs(3),
};

/// How long the first copy waits for a connected launch's message.
const READ_TIMEOUT: Duration = Duration::from_secs(2);

/// What a launch is.
pub enum Claim {
    /// The first copy: it listens for later launches (see
    /// [`Listener::start`]).
    First(Listener),
    /// The files went to the running copy: this launch exits.
    Forwarded,
    /// No handover (no channel, or the running copy did not answer): this
    /// launch runs on its own.
    Alone,
}

/// Who serves a scope, as a platform tells it.
enum Elected<S> {
    Serve(S),
    Taken,
}

/// The channel's scope for the settings file `settings_file` (`None`:
/// the per-user default): copies share a window only when they share
/// their settings.
pub fn scope(settings_file: Option<&Path>) -> String {
    let Some(file) = settings_file else {
        return "default".to_owned();
    };
    // FNV-1a: stable across builds, unlike std's hasher.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in file.as_os_str().as_encoded_bytes() {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("p{hash:016x}")
}

/// Find out what this launch is: the first copy for `scope`, or one that
/// handed `paths` over (all within `timeouts.forward`).
pub fn claim(scope: &str, paths: &[PathBuf], timeouts: Timeouts) -> Claim {
    let Some(message) = protocol::encode(paths) else {
        return Claim::Alone;
    };
    let (tx, rx) = mpsc::channel();
    let scope = scope.to_owned();
    let deadline = Instant::now() + timeouts.forward;
    let worker = std::thread::Builder::new()
        .name("reasoncraft-claim".into())
        .spawn(move || {
            let _ = tx.send(claim_blocking(&scope, &message, deadline));
        });
    if let Err(e) = worker {
        eprintln!("warning: no single-instance check: {e}");
        return Claim::Alone;
    }
    // A worker stuck on a hung copy is left behind; this launch goes on.
    rx.recv_timeout(timeouts.forward + Duration::from_millis(200))
        .unwrap_or(Claim::Alone)
}

fn claim_blocking(scope: &str, message: &[u8], deadline: Instant) -> Claim {
    loop {
        match platform::elect(scope) {
            Ok(Elected::Serve(server)) => return Claim::First(Listener(server)),
            Ok(Elected::Taken) => {}
            Err(e) => {
                eprintln!("warning: no single-instance channel: {e}");
                return Claim::Alone;
            }
        }
        match forward(scope, message, deadline) {
            Ok(true) => return Claim::Forwarded,
            Ok(false) => return Claim::Alone,
            // The first copy is starting, or just ended: look again.
            Err(e) if platform::not_there(&e) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                eprintln!("warning: could not reach the running ReasonCraft: {e}");
                return Claim::Alone;
            }
        }
    }
}

/// Send `message` to the copy serving `scope`; whether its window took
/// the files.
fn forward(scope: &str, message: &[u8], deadline: Instant) -> std::io::Result<bool> {
    use std::io::{Read as _, Write as _};
    let mut conn = platform::connect(scope, deadline)?;
    platform::set_timeouts(&conn, deadline.saturating_duration_since(Instant::now()));
    conn.write_all(message)?;
    let mut answer = [0u8];
    let read = conn.read(&mut answer)?;
    Ok(read == 1 && answer[0] == protocol::TAKEN)
}

/// The first copy's end of the channel, before it listens.
pub struct Listener(platform::Server);

/// Files a launch handed over, and where to say that the window took them.
struct Handover {
    paths: Vec<PathBuf>,
    taken: SyncSender<()>,
}

/// The window's end: the handovers that arrived, and the context to wake
/// for them.
pub struct Inbox {
    rx: Receiver<Handover>,
    waker: Arc<Mutex<Option<egui::Context>>>,
}

impl Listener {
    /// Listen for later launches on a thread of its own; the window takes
    /// what they hand over from the returned inbox (see [`attach`]).
    pub fn start(self, timeouts: Timeouts) -> Inbox {
        let (tx, rx) = mpsc::channel();
        let waker = Arc::new(Mutex::new(None));
        let wake = Arc::clone(&waker);
        let server = self.0;
        let started = std::thread::Builder::new()
            .name("reasoncraft-instance".into())
            .spawn(move || serve(server, &tx, &wake, timeouts.answer));
        if let Err(e) = started {
            eprintln!("warning: not listening for later launches: {e}");
        }
        Inbox { rx, waker }
    }
}

/// Answer launches until the window is gone (or the channel fails
/// repeatedly). A malformed message is dropped unanswered.
fn serve(
    mut server: platform::Server,
    tx: &Sender<Handover>,
    waker: &Mutex<Option<egui::Context>>,
    answer: Duration,
) {
    use std::io::Write as _;
    let mut failures = 0;
    loop {
        let mut conn = match server.accept() {
            Ok(conn) => {
                failures = 0;
                conn
            }
            Err(e) => {
                failures += 1;
                if failures > 10 {
                    eprintln!("warning: stopped listening for later launches: {e}");
                    return;
                }
                continue;
            }
        };
        platform::set_timeouts(&conn, READ_TIMEOUT);
        let Ok(paths) = protocol::read_request(&mut conn) else {
            continue;
        };
        let (taken, was_taken) = mpsc::sync_channel(1);
        if tx.send(Handover { paths, taken }).is_err() {
            // The window is gone: no answer, the launch starts its own.
            return;
        }
        if let Some(ctx) = waker.lock().ok().and_then(|w| w.clone()) {
            ctx.request_repaint();
        }
        if was_taken.recv_timeout(answer).is_ok() {
            let _ = conn.write_all(&[protocol::TAKEN]);
            platform::finish(&conn);
        }
    }
}

/// Files handed over by later launches, for the window.
#[derive(Default)]
pub struct Incoming {
    inbox: Option<Inbox>,
    /// Taken, waiting to open until no dialog is open.
    waiting: Vec<PathBuf>,
}

/// Let the window take what later launches hand over through `inbox`;
/// they wake `ctx`.
pub fn attach(app: &mut ReasonCraftApp, inbox: Inbox, ctx: &egui::Context) {
    if let Ok(mut waker) = inbox.waker.lock() {
        *waker = Some(ctx.clone());
    }
    app.incoming.inbox = Some(inbox);
}

/// Take what later launches handed over: answer them, bring the window to
/// the front, and open the files once no dialog is open (one acts on the
/// active tab, which opening changes). While the window is closing nothing
/// is taken: a launch not answered in time opens its own window, and one
/// still waiting when the close is cancelled is taken then. Run once per
/// frame.
pub fn poll(app: &mut ReasonCraftApp) {
    let mut arrived = false;
    if let Some(inbox) = &app.incoming.inbox
        && !crate::file_ops::window_closing(app)
    {
        while let Ok(handover) = inbox.rx.try_recv() {
            // A launch that gave up already starts its own window.
            if handover.taken.send(()).is_ok() {
                app.incoming.waiting.extend(handover.paths);
                arrived = true;
            }
        }
    }
    if arrived && let Some(ctx) = &app.egui_ctx {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    if !app.incoming.waiting.is_empty() && !app.dialog_open() {
        let paths = std::mem::take(&mut app.incoming.waiting);
        app.push(Action::OpenFiles(paths));
    }
}

/// The environment variable that, in debug builds, names a file each
/// opened map's path is added to (a line each): the single-instance test
/// cannot look inside the window.
pub const OPENED_LOG_VAR: &str = "REASONCRAFT_DEBUG_OPENED_LOG";

/// Note that `path` was opened (debug builds, see [`OPENED_LOG_VAR`]).
pub(crate) fn log_opened(path: &Path) {
    #[cfg(debug_assertions)]
    if let Some(log) = std::env::var_os(OPENED_LOG_VAR) {
        use std::io::Write as _;
        let file = std::fs::File::options().create(true).append(true).open(log);
        if let Ok(mut file) = file {
            let _ = writeln!(file, "{}", path.display());
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = path;
}

#[cfg(test)]
mod tests;
