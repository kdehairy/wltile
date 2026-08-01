//! A headless sway instance under its own `XDG_RUNTIME_DIR`, plus the operations
//! tests drive it with: creating outputs, running `wltile` against it, and
//! observing the resulting compositor state via sway's IPC.

use std::fs::{self, File};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::swaymsg;

use super::daemon::Daemon;
use super::poll_until;
use super::process::{WltileChild, run_bounded};
use super::startup::{find_sway_sock, find_wayland_sock, wait_for_sway_ready};

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

const APPEAR_TIMEOUT: Duration = Duration::from_secs(5);
const SWAYMSG_TIMEOUT: Duration = Duration::from_secs(15);
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

        let wayland_sock = find_wayland_sock(&runtime_dir, &sway_log);
        let sway_sock = find_sway_sock(&runtime_dir, sway_pid, &sway_log);
        wait_for_sway_ready(&sway_sock, &runtime_dir, &sway_log);

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

        poll_until(APPEAR_TIMEOUT, || {
            self.outputs()
                .iter()
                .any(|o| o.name == expected)
                .then_some(())
        })
        .unwrap_or_else(|| panic!("output {expected} did not appear within {APPEAR_TIMEOUT:?}"));

        expected
    }

    /// Runs the `wltile` binary against this compositor with the given args.
    ///
    /// Bounded by `WLTILE_TIMEOUT`
    pub fn run_wltile(&self, args: &[&str]) -> Output {
        self.run_wltile_with_env(args, &[])
    }

    /// Like [`Self::run_wltile`], but with additional environment variables set
    /// (e.g. to isolate `XDG_STATE_HOME` so the daemon doesn't touch the real
    /// user's state directory).
    ///
    /// Bounded by `WLTILE_TIMEOUT`
    pub fn run_wltile_with_env(&self, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
        let mut cmd = self.wltile_command();
        cmd.envs(extra_env.iter().copied()).args(args);

        run_bounded(WLTILE_TIMEOUT, format!("wltile {args:?}"), move || {
            cmd.output()
        })
    }

    /// Spawns `wltile` without waiting for it. Needed for the no-arg `show`,
    /// which blocks for user input and so can't go through the bounded
    /// [`run_wltile`](Self::run_wltile).
    pub fn spawn_wltile(&self, args: &[&str]) -> WltileChild {
        let child = self
            .wltile_command()
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn wltile");
        WltileChild::new(child)
    }

    /// Spawns `wltile daemon --systemd` against this compositor, returning a handle
    /// used to signal and observe it.
    pub fn spawn_daemon(&self, config_path: &Path) -> Daemon {
        let log =
            File::create(self.runtime_dir.join("daemon.log")).expect("failed to create daemon log");
        let child = self
            .wltile_command()
            .args([
                "-vvv",
                "daemon",
                // keeps the process in the foreground instead of forking, so it stays a controllable
                // child here.
                "--systemd",
                "--config",
                config_path.to_str().expect("config path must be utf-8"),
            ])
            .stdout(log.try_clone().expect("failed to clone daemon log"))
            .stderr(log)
            .spawn()
            .expect("failed to spawn wltile daemon");

        Daemon::new(child)
    }

    /// A `Command` for the `wltile` binary, pointed at this compositor instance.
    fn wltile_command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_wltile"));
        cmd.env("WAYLAND_DISPLAY", self.wayland_display_name())
            .env("XDG_RUNTIME_DIR", &self.runtime_dir);
        cmd
    }

    /// Injects a single key press (types `x`) into whichever surface currently
    /// holds keyboard focus, via `wtype` (the Wayland virtual-keyboard protocol).
    pub fn press_key(&self) {
        Command::new("wtype")
            .env("WAYLAND_DISPLAY", self.wayland_display_name())
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .arg("x")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("failed to run `wtype` (is it installed in the test image?)");
    }

    fn wayland_display_name(&self) -> std::ffi::OsString {
        self.wayland_sock
            .file_name()
            .expect("failed to get wayland display")
            .to_owned()
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

    /// Polls `outputs()` until `predicate` is satisfied or `timeout` elapses.
    /// Returns whether the predicate was satisfied.
    pub fn wait_for_outputs(
        &self,
        timeout: Duration,
        predicate: impl Fn(&[swaymsg::Output]) -> bool,
    ) -> bool {
        poll_until(timeout, || predicate(&self.outputs()).then_some(())).is_some()
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
        poll_until(timeout, || {
            if predicate(&self.outputs()) {
                return Some(());
            }
            daemon.reload();
            None
        })
        .is_some()
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
        let mut cmd = Command::new("swaymsg");
        cmd.arg("-s")
            .arg(&self.sway_sock)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .args(args);

        run_bounded(
            SWAYMSG_TIMEOUT,
            format!("swaymsg {args:?} (sway IPC likely starved)"),
            move || cmd.output(),
        )
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        self.process.kill().ok();
        self.process.wait().ok();
        fs::remove_dir_all(&self.runtime_dir).ok();
    }
}
