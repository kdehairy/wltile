use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    thread,
};

use libc::SIGHUP;
use signal_hook::{consts::TERM_SIGNALS, flag, iterator::Signals};
use tracing::{debug, error, trace};

use crate::{
    functions,
    wl_config::Config,
    wlr_client::{self, Client},
};

#[allow(clippy::needless_pass_by_value)]
pub fn daemon_main(config_file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    trace!("daemon started");
    let term_now = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        // term_now is initially false. No effect
        // first sigterm will set term_now. Now the app has a chance to shutdown gracefully.
        // second sigterm, term_now is already true. The handler itself will kill the app.
        flag::register_conditional_shutdown(*sig, -1, term_now.clone())?;

        // This is where the term_now is set when first sigterm is sent.
        flag::register(*sig, term_now.clone())?;
    }

    let mut sigs = vec![SIGHUP];
    sigs.extend(TERM_SIGNALS);
    let mut signals = Signals::new(&sigs)?;

    // signal a reload configurations is needed
    let (reload_tx, reload_rx) = crossbeam_channel::unbounded();

    let client = Arc::new(wlr_client::Client::new().unwrap());

    // wait for a reload signal to reload and apply configs
    thread::spawn({
        let config_file = config_file.clone();
        let client = client.clone();
        move || {
            while let Some(()) = reload_rx.recv().into_iter().next() {
                //drain the channel. No need to reload configs multiple times in a tight loop!
                reload_rx.try_iter().for_each(drop);
                reload_configs(&config_file, &client);
            }
        }
    });

    // wait for a wayland configuration changes to signal a reload
    thread::spawn({
        let update_rx = client.clone().subscribe();
        let reload_tx = reload_tx.clone();
        move || {
            while let Some(()) = update_rx.recv().into_iter().next() {
                let _ = reload_tx.send(());
            }
        }
    });

    // Infinitly iterate over signals queued to be handled.
    // This blocks on iter.next()
    for signal in &mut signals {
        match signal {
            SIGHUP => {
                trace!("Recieved a SIGHUP");
                let _ = reload_tx.send(());
            }
            other => {
                if TERM_SIGNALS.contains(&other) {
                    break;
                }
            }
        }
    }

    // My chance to shutdown gracefully

    Ok(())
}

fn reload_configs(config_file: &Path, client: &Client) {
    trace!("Config file reloading...");
    let mut filtered_config = Config::default();
    {
        let config = Config::try_from(config_file).unwrap();
        let cur_configs = client.configurations_read_lock();
        let heads = cur_configs.read().heads().unwrap();
        drop(cur_configs);
        for target in config.targets() {
            if heads.find(&target.name).is_some() {
                if let Some(pos) = target.position.as_ref()
                    && heads.find(&pos.reference).is_some() {
                        filtered_config.add_target(target.clone());
                }
                if target.position.is_none() {
                    filtered_config.add_target(target.clone());
                }
            }
        }
    }
    debug!("Parsed configs: {filtered_config}");
    //apply any mode change, scale or transformations first.
    if let Err(err) = functions::set_property::exec(&filtered_config, client) {
        error!("failed to apply configurations in file: {err}");
    }

    if let Err(err) = functions::position::exec(&filtered_config, client) {
        error!("failed to apply configurations in file: {err}");
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::daemon;

    #[test]
    fn test_daemon() {
        let config_path = PathBuf::from("/home/kdehairy/test_config.toml");
        let _ = daemon::daemon_main(config_path);
    }
}
