use std::fs::{self, File};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::swaymsg;

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const APPEAR_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct Compositor {
    process: Child,
    /// Unique per-instance directory used as XDG_RUNTIME_DIR.
    /// Sway creates both its Wayland socket and IPC socket here.
    runtime_dir: PathBuf,
    sway_sock: PathBuf,
    wayland_sock: PathBuf,
    output_count: u32,
}

impl Compositor {
    pub fn new() -> Self {
        let n = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();

        // Isolated dir per instance — guarantees no socket name collisions
        // when tests run in parallel.
        let runtime_dir = PathBuf::from(format!("/tmp/wltile-test-{pid}-{n}"));

        // XDG spec requires mode 0700; wlroots refuses to create sockets otherwise.
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&runtime_dir)
            .expect("failed to create runtime dir");

        let sway_log = runtime_dir.join("sway.log");

        let config = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("sway-test.conf");

        let process = Command::new("sway")
            .env("WLR_BACKENDS", "headless")
            .env("WLR_RENDERER", "pixman")
            .env("LIBSEAT_BACKEND", "noop")
            .env("XDG_RUNTIME_DIR", &runtime_dir)
            .arg("--unsupported-gpu")
            .arg("-c")
            .arg(&config)
            .stdout(Stdio::null())
            .stderr(File::create(&sway_log).expect("failed to create sway log"))
            .spawn()
            .expect("failed to spawn sway — is it installed?");

        let sway_pid = process.id();

        // Dynamically find the Wayland socket instead of expecting wayland-0
        let wayland_sock = find_wayland_sock(&runtime_dir, &sway_log, STARTUP_TIMEOUT);
        let sway_sock = find_sway_sock(&runtime_dir, sway_pid, &sway_log);
        wait_for_sway_ready(&sway_sock, &runtime_dir, &sway_log, STARTUP_TIMEOUT);

        Self {
            process,
            runtime_dir,
            sway_sock,
            wayland_sock,
            output_count: 0,
        }
    }

    /// Creates one virtual output and returns its name (e.g. `"HEADLESS-1"`).
    pub fn add_output(&mut self) -> String {
        self.output_count += 1;
        let expected = format!("HEADLESS-{}", self.output_count);

        let result = self.swaymsg(&["create_output"]);
        assert!(
            result.status.success(),
            "swaymsg create_output failed: {}",
            String::from_utf8_lossy(&result.stderr),
        );

        let deadline = Instant::now() + APPEAR_TIMEOUT;
        loop {
            if self.outputs().iter().any(|o| o.name == expected) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "output {expected} did not appear within {APPEAR_TIMEOUT:?}",
            );
            thread::sleep(POLL_INTERVAL);
        }

        expected
    }

    /// Runs the `wltile` binary against this compositor with the given args.
    pub fn run_wltile(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wltile"))
            .env(
                "WAYLAND_DISPLAY",
                self.wayland_sock
                    .file_name()
                    .expect("failed to get wayland display"),
            )
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .args(args)
            .output()
            .expect("failed to run wltile")
    }

    /// Returns the current output state reported by the compositor.
    pub fn outputs(&self) -> Vec<swaymsg::Output> {
        let result = self.swaymsg(&["-t", "get_outputs"]);
        swaymsg::parse_outputs(&result.stdout)
    }

    fn swaymsg(&self, args: &[&str]) -> Output {
        Command::new("swaymsg")
            .arg("-s")
            .arg(&self.sway_sock)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .args(args)
            .output()
            .expect("swaymsg failed")
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        self.process.kill().ok();
        self.process.wait().ok();
        fs::remove_dir_all(&self.runtime_dir).ok();
    }
}

fn read_log(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| String::from("<log unavailable>"))
}

/// Polls `swaymsg -t get_version` until sway's IPC loop is ready to accept connections.
fn wait_for_sway_ready(sway_sock: &Path, runtime_dir: &Path, log: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let ok = Command::new("swaymsg")
            .arg("-s")
            .arg(sway_sock)
            .env("XDG_RUNTIME_DIR", runtime_dir)
            .args(["-t", "get_version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if ok {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "sway IPC never became ready\n--- sway log ---\n{}",
            read_log(log),
        );
        thread::sleep(POLL_INTERVAL);
    }
}

/// Scans `runtime_dir` for `sway-ipc.*.<sway_pid>.sock`, blocking until found.
fn find_sway_sock(runtime_dir: &Path, sway_pid: u32, log: &Path) -> PathBuf {
    let suffix = format!(".{sway_pid}.sock");
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("sway-ipc.") && name.ends_with(&suffix) {
                    return entry.path();
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for sway IPC socket (pid {sway_pid}) in {runtime_dir:?}\n--- sway log ---\n{}",
            read_log(log),
        );
        thread::sleep(POLL_INTERVAL);
    }
}

/// Dynamically finds the Wayland socket created by sway, scanning for any socket in the runtime directory.
/// Sway creates a wayland socket with a numbered suffix (wayland-0, wayland-1, etc.).
fn find_wayland_sock(runtime_dir: &Path, log: &Path, timeout: Duration) -> PathBuf {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("wayland-") && !name.ends_with(".lock") {
                    return entry.path();
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for Wayland socket in {runtime_dir:?}\n--- sway log ---\n{}",
            read_log(log),
        );
        thread::sleep(POLL_INTERVAL);
    }
}
