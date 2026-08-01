mod compositor;
mod daemon;
mod process;
mod startup;

pub use compositor::Compositor;
// Returned by `Compositor::spawn_daemon` / `spawn_wltile`. Tests get them by
// inference and so don't name them today, but the names must stay reachable
// for any test helper that wants them in a signature.
#[allow(unused_imports)]
pub use daemon::Daemon;
#[allow(unused_imports)]
pub use process::WltileChild;

use std::thread;
use std::time::{Duration, Instant};

/// How often every wait in the harness re-checks its condition.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Polls `f` every `POLL_INTERVAL` until it yields a value, or `timeout` elapses.
///
/// `f` is always evaluated at least once, and once more after the final sleep,
/// so a condition that becomes true exactly at the deadline is still observed.
fn poll_until<T>(timeout: Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = f() {
            return Some(value);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(POLL_INTERVAL);
    }
}
