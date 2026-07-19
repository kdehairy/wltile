#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division,
    clippy::needless_lifetimes
)]

mod cli;
mod commons;
mod config_file;
mod daemon;
mod functions;
mod wl_config;
mod wlr_client;

use std::env;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use cli::Property;
use daemonize::Daemonize;
use tracing::debug;
use tracing::level_filters::LevelFilter;
use tracing::trace;

use crate::wl_config::Config;
use crate::wl_config::Position;
use crate::wl_config::Target;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::DEBUG;

#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::ERROR;

#[allow(clippy::too_many_lines)]
#[tracing::instrument()]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    cli::validate(&args)?;

    let log_level = match args.verbose {
        0 => DEFAULT_LOG_LEVEL,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(log_level)
        .init();

    match args.command {
        Commands::List => {
            trace!("cli command: list");
            let client = wlr_client::Client::new()?;
            let configs = client.configurations_read_lock();
            let heads = configs.read().heads()?;
            functions::list::exec(&heads);
            Ok(())
        }
        Commands::Show {
            output: Some(output),
        } => {
            trace!("cli command: show {output}");
            let client = wlr_client::Client::new()?;
            let configs = client.configurations_read_lock();
            let heads = configs.read().heads()?;
            let head = heads.find(&output).ok_or("output does not exist")?;
            functions::show_output::exec(head);
            Ok(())
        }
        Commands::Show { output: None } => {
            trace!("cli command: show");
            let mut client = wlr_client::Client::new()?;
            functions::show::exec(&mut client)?;
            Ok(())
        }
        Commands::Position {
            target,
            relation,
            reference,
            alignment,
        } => {
            trace!("cli command: position {target} {relation} {reference} {alignment}");
            let client = wlr_client::Client::new()?;
            let mut config = Config::default();
            config.add_target(Target {
                name: target,
                position: Some(Position {
                    relation: relation.into(),
                    reference,
                    alignment: alignment.into(),
                }),
                ..Default::default()
            });
            functions::position::exec(&config, &client)?;
            Ok(())
        }
        Commands::Set {
            target,
            property,
            value,
        } => {
            let client = wlr_client::Client::new()?;
            let mut config = Config::default();
            let mut target = Target::new(target);
            match property {
                Property::Mode => {
                    if let Ok(desired) = value.trim().parse::<usize>() {
                        target.mode = Some(desired);
                    } else {
                        Err("Expected integer identifier for the mode")?;
                    }
                }
                Property::Scale => {
                    if let Ok(desired) = value.trim().parse::<f64>() {
                        target.scale = Some(desired);
                    } else {
                        Err("Expected number identifier for the scale")?;
                    }
                }
                Property::Rotation => {
                    if let Ok(desired) = value.trim().parse::<i32>() {
                        target.rotation = Some(desired);
                    } else {
                        Err("Expected number identifier for the rotation")?;
                    }
                }
                Property::Active => {
                    if let Ok(desired) = value.trim().parse::<bool>() {
                        target.active = Some(desired);
                    } else {
                        Err("Expected 'true' or 'false' for active")?;
                    }
                }
            }

            config.add_target(target);
            functions::set_property::exec(&config, &client)?;

            Ok(())
        }
        Commands::Daemon {
            config,
            log,
            err_log,
            systemd,
        } => {
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

            daemon::daemon_main(config_path)
        }
    }
}

fn is_file(config_path: &PathBuf) -> bool {
    if let Ok(meta) = fs::metadata(config_path) {
        meta.is_file()
    } else {
        false
    }
}
