mod connection_manager;
pub mod display;
pub mod errors;
pub mod point;
pub mod shmem;

pub(crate) mod input;
pub(crate) mod output;

use std::sync::{Arc, RwLock, RwLockReadGuard};
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
        queue.dispatch_pending(&mut StateWrapper {
            state: state.clone(),
            update_tx: tx.clone(),
        })?;
        debug!("configurations received");

        thread::spawn({
            let mut state = StateWrapper {
                state: state.clone(),
                update_tx: tx.clone(),
            };
            move || loop {
                thread::sleep(Duration::from_millis(500));

                // flushing all out going requests, if any
                if let Err(err) = queue.flush() {
                    error!("Failed to flush out going events: {}", err);
                }

                match queue.dispatch_pending(&mut state) {
                    Ok(size) => {
                        if size > 0 {
                            trace!("Dispatched {size} pending events");
                        }
                    }
                    Err(err) => error!("Error dispatching events: {}", err),
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
