use std::fs::{self, File};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::swaymsg;

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const APPEAR_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const SWAYMSG_TIMEOUT: Duration = Duration::from_secs(15);
/// A healthy `wltile` one-shot completes well under a second; wltile bounds its
/// own compositor waits internally (a few seconds each). This ceiling only
/// catches a genuine hang (e.g. a Wayland read-coordination stall under CI CPU
/// contention) and turns it into a fast, labelled failure.
const WLTILE_TIMEOUT: Duration = Duration::from_secs(30);

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
    ///
    /// Sway's headless backend always starts with one output (`HEADLESS-1`)
    /// already present, so the first call only waits for it instead of
    /// issuing a redundant `create_output` (which would otherwise spawn an
    /// untracked extra output).
    pub fn add_output(&mut self) -> String {
        self.output_count += 1;
        let expected = format!("HEADLESS-{}", self.output_count);

        if !self.outputs().iter().any(|o| o.name == expected) {
            let result = self.swaymsg(&["create_output"]);
            assert!(
                result.status.success(),
                "swaymsg create_output failed: {}",
                String::from_utf8_lossy(&result.stderr),
            );
        }

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
        self.run_wltile_with_env(args, &[])
    }

    /// Like [`Self::run_wltile`], but with additional environment variables set
    /// (e.g. to isolate `XDG_STATE_HOME` so the daemon doesn't touch the real
    /// user's state directory).
    ///
    /// Bounded by `WLTILE_TIMEOUT`
    pub fn run_wltile_with_env(&self, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
        let bin = env!("CARGO_BIN_EXE_wltile");
        let wayland_display = self
            .wayland_sock
            .file_name()
            .expect("failed to get wayland display")
            .to_owned();
        let runtime_dir = self.runtime_dir.clone();
        let owned_env: Vec<(String, String)> = extra_env
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        let owned_args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();

        run_bounded(WLTILE_TIMEOUT, format!("wltile {owned_args:?}"), move || {
            Command::new(bin)
                .env("WAYLAND_DISPLAY", &wayland_display)
                .env("XDG_RUNTIME_DIR", &runtime_dir)
                .envs(owned_env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .args(&owned_args)
                .output()
        })
    }

    /// Returns the current output state reported by the compositor.
    pub fn outputs(&self) -> Vec<swaymsg::Output> {
        let result = self.swaymsg(&["-t", "get_outputs"]);
        swaymsg::parse_outputs(&result.stdout)
    }

    /// The isolated `XDG_RUNTIME_DIR` this compositor instance runs under.
    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    /// Writes a TOML daemon config into this instance's runtime dir and returns its path.
    pub fn write_config(&self, contents: &str) -> PathBuf {
        let path = self.runtime_dir.join("config.toml");
        fs::write(&path, contents).expect("failed to write daemon config");
        path
    }

    /// Spawns `wltile daemon --systemd` against this compositor, returning a handle
    /// used to signal and observe it. `--systemd` keeps the process in the
    /// foreground instead of forking, so it stays a controllable child here.
    pub fn spawn_daemon(&self, config_path: &Path) -> Daemon {
        let log =
            File::create(self.runtime_dir.join("daemon.log")).expect("failed to create daemon log");
        let child = Command::new(env!("CARGO_BIN_EXE_wltile"))
            .env(
                "WAYLAND_DISPLAY",
                self.wayland_sock
                    .file_name()
                    .expect("failed to get wayland display"),
            )
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .args([
                "-vvv",
                "daemon",
                "--systemd",
                "--config",
                config_path.to_str().expect("config path must be utf-8"),
            ])
            .stdout(log.try_clone().expect("failed to clone daemon log"))
            .stderr(log)
            .spawn()
            .expect("failed to spawn wltile daemon");

        Daemon {
            pid: child.id(),
            child,
        }
    }

    /// Polls `outputs()` until `predicate` is satisfied or `timeout` elapses.
    /// Returns whether the predicate was satisfied.
    pub fn wait_for_outputs(
        &self,
        timeout: Duration,
        predicate: impl Fn(&[swaymsg::Output]) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(&self.outputs()) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    /// Repeatedly sends SIGHUP to `daemon` and polls `outputs()` until `predicate`
    /// is satisfied or `timeout` elapses. Returns whether the predicate was satisfied.
    ///
    /// The daemon's Wayland client only dispatches/polls every 500ms, so its
    /// internal view of the output-manager `serial` can lag behind what
    /// `swaymsg` already reports on the compositor side. A single SIGHUP sent
    /// right after observing a prior change via `swaymsg` can race that lag:
    /// the daemon builds its next commit from a stale serial and the
    /// compositor silently rejects it. Resending on each poll absorbs this
    /// without depending on exact timing.
    pub fn reload_until(
        &self,
        daemon: &Daemon,
        timeout: Duration,
        predicate: impl Fn(&[swaymsg::Output]) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(&self.outputs()) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            daemon.reload();
            thread::sleep(POLL_INTERVAL);
        }
    }

    /// Disables a connected output via sway's IPC.
    pub fn disable_output(&self, name: &str) {
        let result = self.swaymsg(&["output", name, "disable"]);
        assert!(
            result.status.success(),
            "swaymsg output disable failed: {}",
            String::from_utf8_lossy(&result.stderr),
        );
    }

    /// Runs `swaymsg`, bounded by `SWAYMSG_TIMEOUT`.
    fn swaymsg(&self, args: &[&str]) -> Output {
        let sway_sock = self.sway_sock.clone();
        let runtime_dir = self.runtime_dir.clone();
        let owned_args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();

        run_bounded(
            SWAYMSG_TIMEOUT,
            format!("swaymsg {owned_args:?} (sway IPC likely starved)"),
            move || {
                Command::new("swaymsg")
                    .arg("-s")
                    .arg(&sway_sock)
                    .env("XDG_RUNTIME_DIR", &runtime_dir)
                    .args(&owned_args)
                    .output()
            },
        )
    }
}

/// Runs a `Command` bounded by `timeout`.
///
/// Since `Command::output()` has no built-in timeout, this helper runs it Bounded
/// instead of waiting forever.
fn run_bounded<F>(timeout: Duration, label: String, run: F) -> Output
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

impl Drop for Compositor {
    fn drop(&mut self) {
        self.process.kill().ok();
        self.process.wait().ok();
        fs::remove_dir_all(&self.runtime_dir).ok();
    }
}

/// Handle to a `wltile daemon` process spawned via [`Compositor::spawn_daemon`].
pub struct Daemon {
    child: Child,
    pid: u32,
}

impl Daemon {
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
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(POLL_INTERVAL);
        }
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
        self.child.kill().ok();
        self.child.wait().ok();
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
