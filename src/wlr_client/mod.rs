mod connection_manager;
pub mod display;
pub mod errors;
pub mod point;
pub mod shmem;

pub(crate) mod input;
pub(crate) mod output;

use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use display::DisplayServer;
use errors::ClientError;
use input::InputServer;
use output::config_writer::{ConfigWriter, UpdateRequest};
use output::configs::Configurations;
use tracing::{debug, error, trace};

use wayland_client::EventQueue;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

use crate::wlr_client::connection_manager::ConnectionManager;
use crate::wlr_client::output::config_writer;

type AsyncState = Arc<RwLock<Configurations>>;

struct StateWrapper {
    state: AsyncState,
    update_tx: Sender<()>,
}

/// Owns the main event queue used to keep the shared `Configurations` cache
/// up to date. Shared (via `Arc<Mutex<_>>`) between the periodic background
/// dispatcher and any caller that needs an on-demand refresh — see
/// [`Client::refresh_configurations`].
struct MainDispatcher {
    queue: EventQueue<StateWrapper>,
    wrapper: StateWrapper,
}

impl MainDispatcher {
    fn dispatch(&mut self) -> Result<usize, ClientError> {
        self.queue.flush()?;
        Ok(self.queue.dispatch_pending(&mut self.wrapper)?)
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
    dispatcher: Arc<Mutex<MainDispatcher>>,
}

impl Client {
    pub fn new() -> Result<Client, ClientError> {
        let mut conn_man = ConnectionManager::connect()?;
        let queue: EventQueue<StateWrapper> = conn_man.new_queue();
        let queue_handle = queue.handle();

        let state = Arc::new(RwLock::new(Configurations::default()));
        let output_manager: ZwlrOutputManagerV1 = conn_man.bind_global(&queue_handle, 4..=4, ())?;
        trace!("output_manager is binded");
        conn_man.sync()?;

        let (tx, rx) = crossbeam_channel::unbounded();
        let dispatcher = Arc::new(Mutex::new(MainDispatcher {
            queue,
            wrapper: StateWrapper {
                state: state.clone(),
                update_tx: tx.clone(),
            },
        }));
        dispatcher.lock().unwrap().dispatch()?;
        debug!("configurations received");

        thread::spawn({
            let dispatcher = dispatcher.clone();
            move || {
                loop {
                    thread::sleep(Duration::from_millis(500));

                    match dispatcher.lock().unwrap().dispatch() {
                        Ok(size) => {
                            if size > 0 {
                                trace!("Dispatched {size} pending events");
                            }
                        }
                        Err(err) => error!("Error dispatching events: {}", err),
                    }
                }
            }
        });

        trace!("started display server");

        Ok(Client {
            configurations: state,
            output_manager,
            connection_manager: conn_man,
            input_server: None,
            update_rx: rx,
            dispatcher,
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

    /// Forces an immediate dispatch of any already-buffered output-manager
    /// events, refreshing the shared `Configurations` cache.
    ///
    /// The periodic background dispatch only runs every ~500ms, decoupled
    /// from this client's own `update_configurations` calls (which commit
    /// through a separate, dedicated event queue). A caller that applies a
    /// property change and then immediately needs to compute geometry from
    /// it (e.g. positioning one head relative to another whose scale it just
    /// changed) would otherwise read stale, pre-change state from the cache.
    pub(crate) fn refresh_configurations(&self) -> Result<(), ClientError> {
        self.dispatcher.lock().unwrap().dispatch()?;
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
