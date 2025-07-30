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
mod daemon;
mod functions;
mod heads;
mod wlr_client;

use std::env;
use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use cli::Property;
use daemonize::Daemonize;
use functions::position::TargetSetup;
use tracing::debug;
use tracing::level_filters::LevelFilter;
use tracing::trace;

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
            let configs = client.configurations();
            let heads = configs.heads()?;
            functions::list::exec(&heads);
            Ok(())
        }
        Commands::Show {
            output: Some(output),
        } => {
            trace!("cli command: show {output}");
            let client = wlr_client::Client::new()?;
            let configs = client.configurations();
            let heads = configs.heads()?;
            let head = heads.find(&output).ok_or("output does not exist")?;
            functions::show_output::exec(head, &configs);
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
            let configs = client.configurations();
            let heads = configs.heads()?;
            let target_head = heads
                .find(&target)
                .cloned()
                .ok_or("target output does not exist")?;
            let reference_head = heads
                .find(&reference)
                .cloned()
                .ok_or("reference output does not exist")?;
            functions::position::exec(
                &TargetSetup {
                    target: target_head,
                    reference: reference_head,
                    relation,
                    alignment,
                },
                &client,
            )?;
            Ok(())
        }
        Commands::Set {
            target,
            property,
            value,
        } => {
            let client = wlr_client::Client::new()?;
            let configs = client.configurations();
            let heads = configs.heads()?;
            let target_head = heads
                .find(&target)
                .cloned()
                .ok_or("target output does not exist")?;
            match property {
                Property::Mode => {
                    if let Ok(desired) = value.trim().parse::<usize>() {
                        functions::set_mode::exec(&target_head, desired, &client)?;
                    } else {
                        Err("Expected integer identifier for the mode")?;
                    }
                }
                Property::Scale => {
                    if let Ok(desired) = value.trim().parse::<f64>() {
                        functions::set_scale::exec(&target_head, desired, &client)?;
                    } else {
                        Err("Expected number identifier for the scale")?;
                    }
                }
                Property::Rotation => {
                    if let Ok(desired) = value.trim().parse::<i32>() {
                        functions::set_rotation::exec(&target_head, desired, &client)?;
                    } else {
                        Err("Expected number identifier for the rotation")?;
                    }
                }
            }

            Ok(())
        }
        Commands::Daemon {
            config,
            log,
            err_log,
            systemd,
        } => {
            // read and parse the file
            // execute the commands in it in sequence
            // set a watcher on the file
            //
            // set a SIGHUB & SIGTERM handlers
            //
            // Get notified if one of the commands' target is removed or added.

            let xdg_dirs = xdg::BaseDirectories::with_prefix("wltile");
            let tmp_dir = env::temp_dir();

            let config_path = match config {
                Some(path) => PathBuf::from(path),
                None => xdg_dirs.place_config_file("config.yaml")?,
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

            debug!("config file: {}", config_path.to_str().unwrap());
            debug!("log file: {}", log_path.to_str().unwrap());
            debug!("err file: {}", err_path.to_str().unwrap());
            debug!("pid file: {}", pid_path.to_str().unwrap());

            if systemd {
                debug!("systemd managed daemon: {}", pid_path.to_str().unwrap());
            } else {
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
