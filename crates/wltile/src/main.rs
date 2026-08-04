mod cli;
mod daemon;
mod print;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use tracing::level_filters::LevelFilter;
use tracing::trace;

use wltile_core::Client;
use wltile_core::wl_config::Config;
use wltile_core::wl_config::Position;
use wltile_core::wl_config::Target;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::DEBUG;

#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::ERROR;

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
            let client = Client::new()?;
            print::list::exec(&client.heads()?);
            Ok(())
        }
        Commands::Show {
            output: Some(output),
        } => {
            trace!("cli command: show {output}");
            let client = Client::new()?;
            let heads = client.heads()?;
            let head = heads.find(&output).ok_or("output does not exist")?;
            print::show_output::exec(head);
            Ok(())
        }
        Commands::Show { output: None } => {
            trace!("cli command: show");
            let mut client = Client::new()?;
            wltile_core::show(&mut client)?;
            Ok(())
        }
        Commands::Position {
            target,
            relation,
            reference,
            alignment,
        } => {
            trace!("cli command: position {target} {relation} {reference} {alignment}");
            let client = Client::new()?;
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
            wltile_core::position(&config, &client)?;
            Ok(())
        }
        Commands::Set {
            target,
            property,
            value,
        } => {
            trace!("cli command: set {target} {property:?} {value}");
            let client = Client::new()?;
            let mut config = Config::default();
            config.add_target(cli::target_from_set(target, property, &value)?);
            wltile_core::set_property(&config, &client)?;

            Ok(())
        }
        Commands::Daemon {
            config,
            log,
            err_log,
            systemd,
        } => daemon::run(config, log, err_log, systemd),
    }
}
