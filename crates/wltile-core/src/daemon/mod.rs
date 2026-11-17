//! The daemon loop behind `wltile daemon`.
//!
//! It keeps a config file applied: every time outputs are connected or
//! removed, or the process receives `SIGHUP`, it re-reads the file and
//! applies it with [`position`](crate::position) and
//! [`set_property`](crate::set_property).

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crossbeam_channel::Receiver;
use libc::SIGHUP;
use signal_hook::{consts::TERM_SIGNALS, flag, iterator::Signals};
use tracing::{debug, error, info, trace, warn};

use crate::{
    functions,
    wl_config::Config,
    wlr_client::{self, Client},
};

/// Runs the daemon event loop: applies `config_file` whenever the compositor
/// reports an output change or the process receives `SIGHUP`, and returns on the
/// first termination signal.
///
/// The compositor is waited for — see [`wait_for_compositor`] — and a config file
/// the parser rejects (an unreadable or unparsable file, a property of the wrong
/// type, an unknown or same-axis `position` value) is logged and that reload
/// skipped. Anything that panics partway through a reload is caught and logged too
/// (see [`apply_guarded`]), so the thread that applies config files outlives it.
/// Either way the daemon stays up for the next `SIGHUP`.
///
/// # Errors
///
/// Returns an error if the signal handlers cannot be installed.
#[allow(clippy::needless_pass_by_value)]
pub fn daemon_main(config_file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    info!("daemon started");
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

    // Reading signals blocks until one arrives and cannot be given a deadline of
    // its own, so the thread below owns that blocking read and everyone else
    // waits on this channel — which can be waited on with a timeout, which is how
    // [`wait_for_compositor`] both retries on `SIGHUP` and stops on a termination
    // signal instead of sleeping through either.
    let (signal_tx, signal_rx) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        for signal in &mut signals {
            let _ = signal_tx.send(signal);
        }
    });

    // signal a reload configurations is needed
    let (reload_tx, reload_rx) = crossbeam_channel::unbounded();

    let Some(client) = wait_for_compositor(&signal_rx, &term_now) else {
        return Ok(());
    };
    let client = Arc::new(client);

    // wait for a reload signal to reload and apply configs
    thread::spawn({
        let config_file = config_file.clone();
        let client = client.clone();
        move || {
            while let Some(()) = reload_rx.recv().into_iter().next() {
                //drain the channel. No need to reload configs multiple times in a tight loop!
                reload_rx.try_iter().for_each(drop);
                if let Err(panic) = apply_guarded(|| reload_configs(&config_file, &client)) {
                    error!(
                        "panicked applying {}; skipping this reload and going on to the next: \
                         {panic}",
                        config_file.display()
                    );
                }
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

    // Block on the signal channel until a termination signal ends the run.
    while let Ok(signal) = signal_rx.recv() {
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

/// Connects to the compositor, waiting for there to be one.
///
/// A daemon started as part of a session is usually up before its compositor
/// gets there, and a compositor that is restarting takes a moment to announce
/// `zwlr_output_manager_v1` again. Neither is a reason to give up on the machine,
/// so a failed attempt is logged and tried again with [`Backoff`] waits: quickly
/// enough to notice a compositor that is coming up, slowly enough that waiting
/// for one that never arrives costs nothing.
///
/// The wait is interruptible in both directions. `SIGHUP` means try again now,
/// and a termination signal ends the wait — `None` tells the caller to shut down,
/// so a daemon nothing can reach is still a daemon that can be stopped.
///
/// One cost worth knowing: an attempt that reaches the Wayland registry leaves
/// its registry dispatch thread blocked when the attempt then fails. At the
/// [`Backoff`] ceiling that is two threads a minute, and only while there is no
/// compositor to talk to.
fn wait_for_compositor(signal_rx: &Receiver<i32>, term_now: &AtomicBool) -> Option<Client> {
    let mut backoff = Backoff::default();
    loop {
        match wlr_client::Client::new() {
            Ok(client) => {
                if backoff.retries() > 0 {
                    info!(
                        "connected to the compositor after {} retries",
                        backoff.retries()
                    );
                }
                return Some(client);
            }
            Err(err) => {
                let delay = backoff.next_wait();
                warn!("no compositor to talk to: {err}; trying again in {delay:?}");
                // Anything wakes us early: a `SIGHUP` asks for the next attempt
                // now, a termination signal asks for no further attempts.
                let _ = signal_rx.recv_timeout(delay);
                if term_now.load(Ordering::SeqCst) {
                    info!("shutting down rather than waiting for a compositor");
                    return None;
                }
            }
        }
    }
}

/// The waits between attempts at something that may sort itself out: short at
/// first, doubling to a ceiling.
#[derive(Debug)]
struct Backoff {
    /// The wait the next call to [`next_wait`](Self::next_wait) hands out.
    wait: Duration,
    /// What waits stop growing at: long enough to be free, short enough that a
    /// compositor that finally shows up is not waited for much longer than this.
    ceiling: Duration,
    /// How many waits have been handed out, i.e. how many attempts failed.
    retries: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            wait: Duration::from_secs(1),
            ceiling: Duration::from_secs(30),
            retries: 0,
        }
    }
}

impl Backoff {
    /// The wait before the next attempt: one second, then double every time up
    /// to the ceiling.
    fn next_wait(&mut self) -> Duration {
        self.retries = self.retries.saturating_add(1);
        let waited = self.wait;
        self.wait = waited.saturating_mul(2).min(self.ceiling);
        waited
    }

    /// How many waits — failed attempts — so far.
    fn retries(&self) -> u32 {
        self.retries
    }
}

/// Runs one reload in a way that survives a panic inside it.
///
/// A panic escaping the reload thread is the one failure here nothing else can
/// cover for: the process stays up, still answers `SIGHUP`, and never applies a
/// config file again — indistinguishable from a config file that simply does not
/// match the outputs, and only worth noticing by comparing the screens to the
/// file by hand. So the panic is caught, reported like any other failure to
/// apply, and the loop carries on to the next `SIGHUP` or hotplug.
///
/// Continuing with a [`Client`] whose state a panic may have left mid-update is
/// the deliberate trade: that state is a cache of what the compositor last said,
/// the compositor is the authority, and the next event overwrites it.
///
/// Returns `Ok(())` when the reload ran to the end — having logged whatever it
/// could not apply itself — and `Err(text)` with the panic's message when it
/// didn't.
///
/// The payload is read here, while it is owned. Handing it out as a reference is
/// the trap: `Box<dyn Any + Send>` is `Any` itself, so a `&payload` coerces to a
/// trait object over the `Box` and then answers "not a `&str`" to every payload
/// there is — silently throwing away the one piece of information worth logging.
fn apply_guarded(reload: impl FnOnce()) -> Result<(), String> {
    catch_unwind(AssertUnwindSafe(reload)).map_err(|payload| {
        // `unwrap`/`expect`/`panic!` all arrive as a `&str`, `panic_any` with
        // whatever it was given; anything unreadable still has to be reported as
        // the panic it was.
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_string())
            })
            .unwrap_or_else(|| String::from("panicked without a message"))
    })
}

fn reload_configs(config_file: &Path, client: &Client) {
    trace!("Config file reloading...");
    let config = match Config::try_from(config_file) {
        Ok(config) => config,
        Err(err) => {
            error!(
                "failed to load config file {}; skipping this reload: {err}",
                config_file.display()
            );
            return;
        }
    };
    let mut filtered_config = Config::default();
    {
        let cur_configs = client.configurations_read_lock();
        // The cache can be internally inconsistent for a moment mid-hotplug — a
        // head still pointing at a mode the compositor has withdrawn — which is
        // worth skipping this reload over: the next output event reloads it with a
        // cache that has caught up.
        let heads = match cur_configs.read().heads() {
            Ok(heads) => heads,
            Err(err) => {
                error!("failed to read the connected outputs; skipping this reload: {err}",);
                return;
            }
        };
        drop(cur_configs);
        for target in config.targets() {
            if heads.find(&target.name).is_some() {
                if let Some(pos) = target.position.as_ref()
                    && heads.find(pos.reference()).is_some()
                {
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

    // Force the shared head cache to catch up with the property changes just
    // applied above before computing positions from it: set_property::exec's
    // commit goes through its own, separate event queue, so without this the
    // cache used below could still reflect each head's pre-change geometry.
    if let Err(err) = client.refresh_configurations() {
        error!("failed to refresh configurations before positioning: {err}");
    }

    if let Err(err) = functions::position::exec(&filtered_config, client) {
        error!("failed to apply configurations in file: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::Backoff;
    use std::time::Duration;

    fn waits(count: usize) -> Vec<Duration> {
        let mut backoff = Backoff::default();
        (0..count).map(|_| backoff.next_wait()).collect()
    }

    #[test]
    fn starts_short_enough_to_notice_a_compositor_coming_up() {
        // The common case is a session leader racing its own compositor: the
        // first retry has to be soon enough that outputs are configured almost
        // as soon as the compositor is there.
        assert_eq!(Duration::from_secs(1), Backoff::default().next_wait());
    }

    #[test]
    fn doubles_until_it_levels_off_at_the_ceiling() {
        let seconds: Vec<u64> = waits(8).iter().map(Duration::as_secs).collect();
        assert_eq!(
            vec![1, 2, 4, 8, 16, 30, 30, 30],
            seconds,
            "waiting for a compositor that never arrives should cost nothing"
        );
    }

    #[test]
    fn counts_the_attempts_it_has_waited_on() {
        let mut backoff = Backoff::default();
        assert_eq!(0, backoff.retries(), "a daemon that has not retried");
        backoff.next_wait();
        backoff.next_wait();
        assert_eq!(2, backoff.retries(), "two waits handed out, two failures");
    }

    // The panic-catching around a reload. Its own failures are invisible from
    // outside the process — the daemon looks healthy and simply stops applying
    // config files — so they are pinned here rather than trusted.

    #[test]
    fn a_panic_inside_a_reload_is_reported_instead_of_ending_the_reload_thread() {
        // The default panic hook prints its line to stderr while this runs: that
        // line is the test doing its job, not this test panicking.
        let outcome =
            super::apply_guarded(|| panic!("the compositor reported a mode that is not there"));
        let text = outcome.expect_err("a reload that panicked did not complete");
        assert!(
            text.contains("the compositor reported a mode that is not there"),
            "{text} should carry what was panicked with, so the log says why"
        );
    }

    #[test]
    fn a_reload_that_panics_nothing_is_left_alone() {
        let mut applied = 0;
        super::apply_guarded(|| applied += 1).expect("no panic, no report");
        assert_eq!(1, applied, "the reload ran exactly once");
    }

    #[test]
    fn a_panic_that_carries_no_message_still_says_a_panicked() {
        let outcome = super::apply_guarded(|| std::panic::panic_any(7_u32));
        assert_eq!(
            "panicked without a message",
            outcome.expect_err("a payload nobody can read is still a panic to report")
        );
    }

    #[test]
    fn a_payload_thrown_as_a_string_is_reported_as_its_text() {
        // `panic_any(String)` is the other payload type a panic can arrive with;
        // a `panic!("…")` of any kind arrives as a `&str`, which the test above
        // covers end to end.
        let outcome =
            super::apply_guarded(|| std::panic::panic_any(String::from("thrown as a String")));
        assert_eq!(
            "thrown as a String",
            outcome.expect_err("a panic is not a completed reload")
        );
    }
}
