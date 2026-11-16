//! Handle for a `wltile daemon` child process: signalling it and observing its exit.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, ExitStatus};
use std::time::Duration;

use super::poll_until;

/// How long to wait for a daemon to exit after SIGTERM at teardown before
/// falling back to SIGKILL.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Handle to a `wltile daemon` process spawned via [`Compositor::spawn_daemon`].
///
/// [`Compositor::spawn_daemon`]: super::Compositor::spawn_daemon
pub struct Daemon {
    child: Child,
    pid: u32,
    /// Where `spawn_daemon` redirected the daemon's stdout and stderr.
    log_path: PathBuf,
}

impl Daemon {
    pub(super) fn new(child: Child, log_path: PathBuf) -> Self {
        Self {
            pid: child.id(),
            child,
            log_path,
        }
    }

    /// Sends SIGHUP, asking the daemon to reload and reapply its config file.
    pub fn reload(&self) {
        self.signal(libc::SIGHUP);
    }

    /// Sends SIGTERM, asking the daemon to shut down gracefully.
    pub fn terminate(&self) {
        self.signal(libc::SIGTERM);
    }

    /// Polls for process exit until `timeout` elapses.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<ExitStatus> {
        poll_until(timeout, || self.child.try_wait().ok().flatten())
    }

    /// Whether the process is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Everything the daemon has written to its log so far.
    pub fn log(&self) -> String {
        fs::read_to_string(&self.log_path).unwrap_or_default()
    }

    /// Polls the daemon's log until `predicate` accepts it, reporting whether
    /// that happened before `timeout` elapsed.
    ///
    /// Lets a test observe something the daemon only *reports* — a config file
    /// it refused to load — as the positive control for "the reload actually
    /// ran and handled it", rather than inferring it from an absence of change.
    pub fn wait_for_log(&self, timeout: Duration, predicate: impl Fn(&str) -> bool) -> bool {
        poll_until(timeout, || predicate(&self.log()).then_some(())).is_some()
    }

    fn signal(&self, sig: libc::c_int) {
        // SAFETY: `self.pid` is this process's own child; signaling it is safe.
        let pid: libc::pid_t = self.pid.cast_signed();
        unsafe {
            libc::kill(pid, sig);
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // Shut down with SIGTERM (graceful) rather than SIGKILL: a normal exit
        // flushes the daemon's coverage profile, whereas SIGKILL is uncatchable
        // and discards it. Fall back to SIGKILL if it doesn't stop in time so
        // teardown can never hang.
        self.terminate();
        if self.wait_for_exit(SHUTDOWN_TIMEOUT).is_none() {
            self.child.kill().ok();
        }
        self.child.wait().ok();
    }
}
