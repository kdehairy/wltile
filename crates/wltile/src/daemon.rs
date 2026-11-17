//! Process bootstrap for `wltile daemon`.
//!
//! Resolves the config/log/pid paths, detaches from the terminal unless systemd
//! is managing the process, then hands over to [`wltile_core::daemon`].
//!
//! Nothing here panics: a daemon is usually started before anyone is looking at
//! it, so an environment it is not ready for is reported as an error naming the
//! path rather than a crash. The guardrail is the fork — a problem the user has
//! to read about is detected before it, while there is still a terminal to read
//! it on.

use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use daemonize::Daemonize;
use tracing::debug;

/// Where the config file comes from when `--config` is left out, spelled the way
/// the user would have to write it out.
const DEFAULT_CONFIG: &str = "$XDG_CONFIG_HOME/wltile/config.toml";

/// The files a daemon run is laid out around, with the config file settled: it
/// either existed or was created empty.
#[derive(Debug)]
struct Paths {
    config: PathBuf,
    log: PathBuf,
    err: PathBuf,
    pid: PathBuf,
}

impl Paths {
    /// Resolves the four paths `run` works with. The config file is the only one
    /// that has to exist afterwards, and only when it was asked for by name.
    ///
    /// # Errors
    ///
    /// Errors when a named config file is not there, when a default file cannot
    /// be placed in (or created in) its XDG directory, or when the default config
    /// path is taken by something that is not a regular file. Every message names
    /// the path it tripped over.
    fn resolve(
        dirs: &xdg::BaseDirectories,
        config: Option<&str>,
        log: Option<&str>,
        err_log: Option<&str>,
    ) -> Result<Paths, Box<dyn std::error::Error>> {
        let config = match config {
            Some(path) => {
                let path = PathBuf::from(path);
                if !is_file(&path) {
                    Err(format!(
                        "config file '{}' does not exist; create it, or leave out --config to \
                         use {DEFAULT_CONFIG}",
                        path.display()
                    ))?;
                }
                path
            }
            None => default_config(dirs)?,
        };

        let log = match log {
            Some(path) => PathBuf::from(path),
            None => dirs
                .place_state_file("logs.log")
                .map_err(|err| format!("failed to place the default log file: {err}"))?,
        };
        let err = match err_log {
            Some(path) => PathBuf::from(path),
            None => dirs
                .place_state_file("errors.log")
                .map_err(|err| format!("failed to place the default error log file: {err}"))?,
        };
        // The pid file belongs in $XDG_RUNTIME_DIR, which need not exist outside a
        // login session. Falling back to the temp dir keeps the refusing-a-second-
        // daemon guard below working instead of failing a start that would
        // otherwise be fine.
        let pid = dirs
            .place_runtime_file("daemon.pid")
            .unwrap_or_else(|_| env::temp_dir().join("daemon.pid"));

        Ok(Paths {
            config,
            log,
            err,
            pid,
        })
    }
}

pub(crate) fn run(
    config: Option<&str>,
    log: Option<&str>,
    err_log: Option<&str>,
    systemd: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let dirs = xdg::BaseDirectories::with_prefix("wltile");
    let Paths {
        config,
        log,
        err,
        pid,
    } = Paths::resolve(&dirs, config, log, err_log)?;

    debug!("config file: {}", config.display());
    debug!("log file: {}", log.display());
    debug!("err file: {}", err.display());
    debug!("pid file: {}", pid.display());

    if systemd {
        debug!("systemd managed daemon: {}", pid.display());
    } else {
        refuse_a_second_daemon(&pid)?;
        // Both logs are opened here rather than left to `daemonize`: a log path
        // that cannot be written to is a message on the terminal that launched the
        // daemon, not a panic in a detached process that has nowhere left to put
        // one.
        let daemonize = Daemonize::new()
            .pid_file(pid)
            .chown_pid_file(true)
            .working_directory(env::temp_dir())
            .stdout(open_log(&log)?)
            .stderr(open_log(&err)?);

        daemonize.start()?;
    }

    wltile_core::daemon::daemon_main(config)
}

/// The config file under `$XDG_CONFIG_HOME`, created empty on a first run.
///
/// An empty file parses as a config with no targets, which is both something the
/// daemon can apply and something the user has to edit.
///
/// # Errors
///
/// Errors when the file's directory cannot be created or the file itself cannot,
/// or when the path is taken by something that is not a regular file.
fn default_config(dirs: &xdg::BaseDirectories) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = dirs
        .place_config_file("config.toml")
        .map_err(|err| format!("failed to place the default config file: {err}"))?;
    if !is_file(&path) {
        match File::create_new(&path) {
            Ok(_) => (),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(format!(
                "'{}' exists and is not a regular file",
                path.display()
            ))?,
            Err(err) => Err(format!(
                "failed to create the default config file '{}': {err}",
                path.display()
            ))?,
        }
    }
    Ok(path)
}

/// Refuses to start while a pid file is around, which may belong to a daemon that
/// is still running.
///
/// # Errors
///
/// Errors naming `pid` when it exists.
fn refuse_a_second_daemon(pid: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if is_file(pid) {
        Err(format!(
            "a pid file exists which might belong to an already running daemon. Please make sure \
             the daemon is stopped and the file is cleaned first: {}",
            pid.display()
        ))?;
    }
    Ok(())
}

/// Opens a file for the daemon to log into, truncating what is already there.
///
/// # Errors
///
/// Errors naming `path` and the OS reason when it cannot be created or opened.
fn open_log(path: &Path) -> Result<File, Box<dyn std::error::Error>> {
    File::create(path)
        .map_err(|err| format!("failed to open log file '{}': {err}", path.display()).into())
}

fn is_file(path: &Path) -> bool {
    if let Ok(meta) = fs::metadata(path) {
        meta.is_file()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::DirBuilderExt;
    use std::path::{Path, PathBuf};

    use super::{Paths, is_file, open_log, refuse_a_second_daemon};

    /// A `BaseDirectories` with all three homes inside `root`, so a test resolves
    /// the paths the daemon does without touching the real ones — or the process
    /// environment, which tests in one binary share.
    ///
    /// `$XDG_RUNTIME_DIR` has to exist and be private to the user before the
    /// `xdg` crate will place anything in it, so an unusable one is left unset and
    /// the daemon's temp-dir fallback applies.
    fn dirs_in(root: &Path) -> xdg::BaseDirectories {
        let mut dirs = xdg::BaseDirectories::with_prefix("wltile");
        dirs.config_home = Some(root.join("config"));
        dirs.state_home = Some(root.join("state"));
        dirs.runtime_dir = Some(root.join("run"));
        dirs
    }

    /// An isolated tree to resolve against, cleaned up first so a rerun starts
    /// from nothing.
    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("failed to create the scratch dir");
        fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join("run"))
            .expect("failed to create the runtime dir");
        root
    }

    #[test]
    fn reports_true_only_for_an_existing_regular_file() {
        let dir = scratch("wltile-is-file-test");

        let file = dir.join("config.toml");
        fs::write(&file, "").expect("failed to write temp file");

        assert!(is_file(&file), "an existing regular file");
        assert!(!is_file(&dir), "a directory is not a file");
        assert!(!is_file(&dir.join("absent.toml")), "a missing path");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_the_default_config_file_that_a_first_run_has_no_reason_to_have() {
        let root = scratch("wltile-daemon-paths-first-run");

        let paths = Paths::resolve(&dirs_in(&root), None, None, None)
            .expect("a fresh setup with no config file is a startable daemon");

        assert_eq!(
            root.join("config/wltile/config.toml"),
            paths.config,
            "the default config file is the one under $XDG_CONFIG_HOME"
        );
        assert!(
            is_file(&paths.config),
            "the daemon creates the config file it is about to read"
        );
        assert!(
            fs::metadata(&paths.config)
                .expect("the config file it just created is readable")
                .len()
                == 0,
            "and creates it empty, rather than inventing a layout to apply"
        );

        // Resolving twice — the second daemon run on the same machine — must keep
        // the file the user has meanwhile written into.
        fs::write(&paths.config, "[eDP-1]\nscale = 2\n").expect("failed to write the config");
        let again = Paths::resolve(&dirs_in(&root), None, None, None)
            .expect("an existing config file is no reason to fail");
        assert_eq!(
            "[eDP-1]\nscale = 2\n",
            fs::read_to_string(&again.config).expect("failed to read the config back")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_a_named_config_file_that_is_not_there_without_creating_it() {
        let root = scratch("wltile-daemon-paths-absent-config");
        let config = root.join("nope.toml");

        let err = Paths::resolve(&dirs_in(&root), Some(config.to_str().unwrap()), None, None)
            .expect_err("a config file named on the command line is not silently invented")
            .to_string();
        assert!(
            err.contains(&config.display().to_string()),
            "{err} should name the file it looked for"
        );
        assert!(
            err.contains("$XDG_CONFIG_HOME/wltile/config.toml"),
            "{err} should point at the default it would have used"
        );
        assert!(
            !config.exists(),
            "refusing to start must not leave an empty config file behind"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_a_config_path_taken_by_something_that_is_not_a_file() {
        let root = scratch("wltile-daemon-paths-config-is-a-directory");
        fs::create_dir_all(root.join("config/wltile/config.toml"))
            .expect("failed to put a directory at the default config path");

        let err = Paths::resolve(&dirs_in(&root), None, None, None)
            .expect_err("a directory is not a config file to read later")
            .to_string();
        assert!(
            err.contains("not a regular file"),
            "{err} should say what is in the way"
        );
        assert!(
            err.contains("config.toml"),
            "{err} should name the path in the way: {err}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_a_state_directory_it_cannot_write_into() {
        let root = scratch("wltile-daemon-paths-state-is-a-file");
        // Whatever `place_state_file` tries next, creating a directory under a
        // regular file fails — which is the shape of a read-only or hijacked
        // $XDG_STATE_HOME, and fails the same way whatever the user's privileges.
        fs::write(root.join("state"), "not a directory").expect("failed to write the state file");

        let err = Paths::resolve(&dirs_in(&root), None, None, None)
            .expect_err("a daemon that cannot place its log has nowhere to report a problem")
            .to_string();
        assert!(
            err.contains("log file"),
            "{err} should say which file failed"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn keeps_the_log_and_pid_paths_it_was_given() {
        let root = scratch("wltile-daemon-paths-explicit-logs");
        let config = root.join("config.toml");
        fs::write(&config, "").expect("failed to write the config file");
        let log = root.join("out.log");
        let err_log = root.join("err.log");

        let paths = Paths::resolve(
            &dirs_in(&root),
            config.to_str(),
            log.to_str(),
            err_log.to_str(),
        )
        .expect("a config file that exists and writable log paths start a daemon");

        assert_eq!(config, paths.config);
        assert_eq!(log, paths.log, "a --log path is used as given");
        assert_eq!(err_log, paths.err, "an --err path is used as given");
        assert!(
            paths.pid.ends_with("daemon.pid"),
            "the pid file is the daemon's own, at {}",
            paths.pid.display()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn names_the_log_file_it_could_not_open() {
        let root = scratch("wltile-daemon-open-log");
        // A directory: `File::create` fails on it for every user.
        let err = open_log(&root)
            .expect_err("a directory cannot be logged into")
            .to_string();
        assert!(
            err.contains(&root.display().to_string()),
            "{err} should name the path"
        );
        assert!(err.contains("log file"), "{err} should say what failed");

        let log = root.join("logs.log");
        let file = open_log(&log).expect("a plain path opens for logging");
        assert!(is_file(&log));
        drop(file);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_a_second_daemon_while_its_pid_file_lives() {
        let root = scratch("wltile-daemon-pid-file");
        let pid = root.join("daemon.pid");

        refuse_a_second_daemon(&pid).expect("no pid file, nothing to refuse");

        fs::write(&pid, "999999999").expect("failed to seed a pid file");
        let err = refuse_a_second_daemon(&pid)
            .expect_err("a pid file may belong to a running daemon")
            .to_string();
        assert!(err.contains("pid file"), "{err} should say what it found");
        assert!(
            err.contains(&pid.display().to_string()),
            "{err} should name the file to clean up"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
