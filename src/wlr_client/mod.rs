mod connection_manager;
pub mod display;
pub mod errors;
pub mod point;
pub mod shmem;

pub(crate) mod input;
pub(crate) mod output;

use std::sync::{Arc, Condvar, Mutex, RwLock, RwLockReadGuard};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use display::DisplayServer;
use errors::ClientError;
use input::InputServer;
use output::config_writer::{ConfigWriter, UpdateRequest};
use output::configs::Configurations;
use tracing::{debug, error, trace};

use wayland_client::protocol::wl_callback::{self, WlCallback};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

use crate::wlr_client::connection_manager::ConnectionManager;
use crate::wlr_client::output::config_writer;

type AsyncState = Arc<RwLock<Configurations>>;

struct StateWrapper {
    state: AsyncState,
    update_tx: Sender<()>,
}

/// One-shot cross-thread barrier used by [`Client::refresh_configurations`] to
/// block until the main queue thread has dispatched a `wl_display.sync`
/// callback; every compositor event emitted before it is now reflected in
/// the shared `Configurations` cache.
#[derive(Clone)]
struct Barrier {
    inner: Arc<(Mutex<bool>, Condvar)>,
}

impl Default for Barrier {
    fn default() -> Self {
        Self {
            inner: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }
}

impl Barrier {
    fn signal(&self) {
        let (lock, cvar) = &*self.inner;
        *lock.lock().expect("barrier mutex poisoned") = true;
        cvar.notify_all();
    }

    /// Waits up to `timeout` for [`signal`](Self::signal). Returns `true` if
    /// signalled, `false` on timeout.
    fn wait(&self, timeout: Duration) -> bool {
        let (lock, cvar) = &*self.inner;
        let guard = lock.lock().expect("barrier mutex poisoned");
        let (_guard, res) = cvar
            .wait_timeout_while(guard, timeout, |signalled| !*signalled)
            .expect("barrier mutex poisoned");
        !res.timed_out()
    }
}

impl Dispatch<WlCallback, Barrier> for StateWrapper {
    fn event(
        _state: &mut Self,
        _proxy: &WlCallback,
        event: <WlCallback as wayland_client::Proxy>::Event,
        data: &Barrier,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            data.signal();
        }
    }
}

#[derive(Clone)]
pub struct ConfigurationsReadLock {
    configurations: AsyncState,
}

impl ConfigurationsReadLock {
    pub fn read(&self) -> RwLockReadGuard<'_, Configurations> {
        self.configurations.read().unwrap()
    }
}

/// wlroots client that handles communication with the compositor.
///
/// The client is unusable until the first invokation of `connect()` method.
pub struct Client {
    configurations: AsyncState,
    output_manager: ZwlrOutputManagerV1,
    connection_manager: ConnectionManager,
    input_server: Option<InputServer>,
    update_rx: Receiver<()>,
    queue_handle: QueueHandle<StateWrapper>,
}

impl Client {
    pub fn new() -> Result<Client, ClientError> {
        let mut conn_man = ConnectionManager::connect()?;
        let mut queue: EventQueue<StateWrapper> = conn_man.new_queue();
        let queue_handle = queue.handle();

        let state = Arc::new(RwLock::new(Configurations::default()));
        let output_manager: ZwlrOutputManagerV1 = conn_man.bind_global(&queue_handle, 4..=4, ())?;
        trace!("output_manager is binded");
        conn_man.sync()?;

        let (tx, rx) = crossbeam_channel::unbounded();
        let mut wrapper = StateWrapper {
            state: state.clone(),
            update_tx: tx,
        };

        // Drain what the initial sync pulled in so the cache is populated before
        // we hand the queue to its dispatch thread and return to the caller.
        queue.dispatch_pending(&mut wrapper)?;
        debug!("configurations received");

        // Event-driven main dispatch: block until the compositor sends events,
        // then dispatch them into the shared `Configurations` cache.
        thread::spawn(move || {
            loop {
                if let Err(err) = queue.blocking_dispatch(&mut wrapper) {
                    error!("fatal: main queue dispatch failed: {err}");
                    std::process::exit(1);
                }
            }
        });

        Ok(Client {
            configurations: state,
            output_manager,
            connection_manager: conn_man,
            input_server: None,
            update_rx: rx,
            queue_handle,
        })
    }

    pub fn configurations_read_lock(&self) -> ConfigurationsReadLock {
        ConfigurationsReadLock {
            configurations: self.configurations.clone(),
        }
    }

    pub fn subscribe(&self) -> Receiver<()> {
        self.update_rx.clone()
    }

    /// Blocks until the shared `Configurations` cache reflects everything the
    /// compositor has emitted so far, then returns.
    ///
    /// A caller that applies a property change and then immediately needs to
    /// compute geometry from it (e.g. positioning one head relative to another
    /// whose scale it just changed) would otherwise read stale configurations.
    ///
    /// Implemented as a barrier: enqueue a `wl_display.sync` callback on the
    /// main queue. Because its `Done` is dispatched by the main queue thread
    /// only after every earlier compositor event, the cache is guaranteed
    /// current once the barrier fires.
    pub(crate) fn refresh_configurations(&self) -> Result<(), ClientError> {
        let barrier = Barrier::default();
        let conn = self.connection_manager.connection();
        conn.display().sync(&self.queue_handle, barrier.clone());
        conn.flush()?;
        if !barrier.wait(Duration::from_secs(3)) {
            return Err(ClientError::Dispatch {
                msg: String::from("Timed out refreshing configurations"),
            });
        }
        Ok(())
    }

    /// Updates the outputs configurations to match the provided request.
    pub(crate) fn update_configurations(
        &self,
        update_request: &UpdateRequest,
    ) -> Result<(), ClientError> {
        trace!("received update request: {update_request}");
        let mut config_writer: ConfigWriter =
            config_writer::ConfigWriter::new(&self.connection_manager);
        config_writer.write(
            update_request,
            &self.output_manager,
            &self.configurations_read_lock(),
        )?;
        self.connection_manager.sync()
    }

    pub(crate) fn new_display_server(&mut self) -> Result<DisplayServer, ClientError> {
        let configs_lock = self.configurations_read_lock();
        DisplayServer::start(&mut self.connection_manager, configs_lock)
    }

    pub(crate) fn get_input_server(&mut self) -> Result<&mut InputServer, ClientError> {
        if self.input_server.is_none() {
            let input_server = InputServer::start(&mut self.connection_manager)?;
            self.input_server = Some(input_server);
        }

        Ok(self.input_server.as_mut().expect("Should not happen"))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.output_manager.stop();
    }
}
