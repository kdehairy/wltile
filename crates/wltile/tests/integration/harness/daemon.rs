//! Handle for a `wltile daemon` child process: signalling it and observing its exit.

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
}

impl Daemon {
    pub(super) fn new(child: Child) -> Self {
        Self {
            pid: child.id(),
            child,
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
