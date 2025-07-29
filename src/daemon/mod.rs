use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

use libc::SIGHUP;
use signal_hook::{consts::TERM_SIGNALS, flag, iterator::Signals};
use tracing::trace;

pub fn daemon_main(config_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

    // This is where the wayland client stuff will happen.

    // Infinitly iterate over signals queued to be handled.
    // This blocks on iter.next()
    for signal in &mut signals {
        match signal {
            SIGHUP => reload_configs(config_file)?,
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

fn reload_configs(_config_file: &Path) -> Result<(), std::io::Error> {
    todo!()
}
