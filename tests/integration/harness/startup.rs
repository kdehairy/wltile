//! Waiting for a freshly spawned sway to become usable: locating the sockets it
//! creates in its runtime dir, and confirming its IPC loop is accepting clients.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::poll_until;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Polls `swaymsg -t get_version` until sway's IPC loop is ready to accept connections.
pub(super) fn wait_for_sway_ready(sway_sock: &Path, runtime_dir: &Path, log: &Path) {
    poll_until(STARTUP_TIMEOUT, || {
        Command::new("swaymsg")
            .arg("-s")
            .arg(sway_sock)
            .env("XDG_RUNTIME_DIR", runtime_dir)
            .args(["-t", "get_version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
            .then_some(())
    })
    .unwrap_or_else(|| {
        panic!(
            "sway IPC never became ready\n--- sway log ---\n{}",
            read_log(log),
        )
    });
}

/// Scans `runtime_dir` for `sway-ipc.*.<sway_pid>.sock`, blocking until found.
pub(super) fn find_sway_sock(runtime_dir: &Path, sway_pid: u32, log: &Path) -> PathBuf {
    let suffix = format!(".{sway_pid}.sock");
    poll_until(STARTUP_TIMEOUT, || {
        find_entry(runtime_dir, |name| {
            name.starts_with("sway-ipc.") && name.ends_with(&suffix)
        })
    })
    .unwrap_or_else(|| {
        panic!(
            "timed out waiting for sway IPC socket (pid {sway_pid}) in {runtime_dir:?}\n--- sway log ---\n{}",
            read_log(log),
        )
    })
}

/// Dynamically finds the Wayland socket created by sway, scanning for any socket in the runtime directory.
/// Sway creates a wayland socket with a numbered suffix (wayland-0, wayland-1, etc.).
pub(super) fn find_wayland_sock(runtime_dir: &Path, log: &Path) -> PathBuf {
    poll_until(STARTUP_TIMEOUT, || {
        find_entry(runtime_dir, |name| {
            name.starts_with("wayland-") && !name.ends_with(".lock")
        })
    })
    .unwrap_or_else(|| {
        panic!(
            "timed out waiting for Wayland socket in {runtime_dir:?}\n--- sway log ---\n{}",
            read_log(log),
        )
    })
}

/// Returns the path of the first entry in `dir` whose file name satisfies `matches`.
fn find_entry(dir: &Path, matches: impl Fn(&str) -> bool) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .find_map(|entry| matches(&entry.file_name().to_string_lossy()).then(|| entry.path()))
}

fn read_log(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| String::from("<log unavailable>"))
}
