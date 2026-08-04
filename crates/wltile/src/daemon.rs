//! Process bootstrap for `wltile daemon`.
//!
//! Resolves the config/log/pid paths, detaches from the terminal unless systemd
//! is managing the process, then hands over to [`wltile_core::daemon`].

use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use daemonize::Daemonize;
use tracing::debug;

pub(crate) fn run(
    config: Option<String>,
    log: Option<String>,
    err_log: Option<String>,
    systemd: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("wltile");
    let tmp_dir = env::temp_dir();

    let config_path = if let Some(path) = config {
        PathBuf::from(path)
    } else {
        let default_path = xdg_dirs.place_config_file("config.toml")?;
        match fs::exists(&default_path) {
            Ok(false) => {
                fs::File::create_new(&default_path)?;
            }
            Err(err) => panic!("{err}"),
            _ => (),
        }
        default_path
    };

    let log_path = match log {
        Some(path) => PathBuf::from(path),
        None => xdg_dirs.place_state_file("logs.log")?,
    };
    let err_path = match err_log {
        Some(path) => PathBuf::from(path),
        None => xdg_dirs.place_state_file("errors.log")?,
    };
    let pid_path = match xdg_dirs.place_runtime_file("daemon.pid") {
        Ok(path) => path,
        Err(_) => tmp_dir.join("daemon.pid"),
    };

    if !is_file(&config_path) {
        Err(format!(
            "config file '{}' does not exist",
            config_path.to_str().unwrap()
        ))?;
    }

    debug!("config file: {}", config_path.to_str().unwrap());
    debug!("log file: {}", log_path.to_str().unwrap());
    debug!("err file: {}", err_path.to_str().unwrap());
    debug!("pid file: {}", pid_path.to_str().unwrap());

    if systemd {
        debug!("systemd managed daemon: {}", pid_path.to_str().unwrap());
    } else {
        if is_file(&pid_path) {
            Err(format!(
                "a pid file exists which might belong to an already running daemon. Please make sure the daemon is stopped and the file is cleaned first: {}",
                pid_path.to_str().unwrap()
            ))?;
        }

        let daemonize = Daemonize::new()
            .pid_file(pid_path)
            .chown_pid_file(true)
            .working_directory(tmp_dir)
            .stdout(File::create(log_path).unwrap())
            .stderr(File::create(err_path).unwrap());

        daemonize.start()?;
    }

    wltile_core::daemon::daemon_main(config_path)
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
    use super::is_file;
    use std::env;
    use std::fs;

    #[test]
    fn reports_true_only_for_an_existing_regular_file() {
        let dir = env::temp_dir().join("wltile-is-file-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create temp dir");

        let file = dir.join("config.toml");
        fs::write(&file, "").expect("failed to write temp file");

        assert!(is_file(&file), "an existing regular file");
        assert!(!is_file(&dir), "a directory is not a file");
        assert!(!is_file(&dir.join("absent.toml")), "a missing path");

        let _ = fs::remove_dir_all(&dir);
    }
}
