//! Running and observing child processes: a timeout wrapper around
//! `Command::output()`, and the handle for a non-blocking `wltile` spawn.

use std::process::{Child, ExitStatus, Output};
use std::thread;
use std::time::Duration;

use super::poll_until;

/// Runs a `Command` bounded by `timeout`.
///
/// Since `Command::output()` has no built-in timeout, this helper runs it Bounded
/// instead of waiting forever.
pub(super) fn run_bounded<F>(timeout: Duration, label: String, run: F) -> Output
where
    F: FnOnce() -> std::io::Result<Output> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        // The receiver may already be gone if we timed out; ignore.
        let _ = tx.send(run());
    });

    rx.recv_timeout(timeout)
        .unwrap_or_else(|_| panic!("{label} did not complete within {timeout:?}"))
        .unwrap_or_else(|err| panic!("{label} failed to run: {err}"))
}

/// Handle to a `wltile` process spawned via [`Compositor::spawn_wltile`].
///
/// [`Compositor::spawn_wltile`]: super::Compositor::spawn_wltile
pub struct WltileChild {
    child: Child,
}

impl WltileChild {
    pub(super) fn new(child: Child) -> Self {
        Self { child }
    }

    /// Polls for exit until `timeout` elapses. A clean exit (returned here)
    /// flushes the process's coverage profile; `None` means it's still running.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<ExitStatus> {
        poll_until(timeout, || self.child.try_wait().ok().flatten())
    }
}

impl Drop for WltileChild {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}
