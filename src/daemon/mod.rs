use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use libc::SIGHUP;
use signal_hook::{consts::TERM_SIGNALS, flag, iterator::Signals};
use tracing::trace;

use crate::wlr_client;

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

    let client = wlr_client::Client::new()?;
    let update_rx = client.subscribe();
    thread::spawn({
        let config_file = config_file.clone();
        move || {
            while let Some(()) = update_rx.recv().into_iter().next() {
                reload_configs(&config_file);
            }
        }
    });

    // Infinitly iterate over signals queued to be handled.
    // This blocks on iter.next()
    for signal in &mut signals {
        match signal {
            SIGHUP => reload_configs(&config_file),
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

fn reload_configs(_config_file: &Path) {
    trace!("Config file reloaded");
}
