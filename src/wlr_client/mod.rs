mod connection_manager;
pub mod display;
pub mod errors;
pub mod point;
pub mod shmem;

pub(crate) mod output;
pub(crate) mod input;

use output::config_writer::{ConfigWriter, UpdateRequest};
use output::configs::Configurations;
use display::DisplayServer;
use input::InputServer;
use errors::ClientError;
use tracing::{debug, trace};

use wayland_client::EventQueue;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

use crate::wlr_client::connection_manager::ConnectionManager;
use crate::wlr_client::output::config_writer;

/// wlroots client that handles communication with the compositor.
///
/// The client is unusable until the first invokation of `connect()` method.
pub struct Client {
    configurations: Configurations,
    output_manager: ZwlrOutputManagerV1,
    connection_manager: ConnectionManager,
    display_server: Option<DisplayServer>,
    input_server: Option<InputServer>,
}

impl Client {
    /// Connects to the wlroots compositor and receive the outputs configurations.
    pub fn new() -> Result<Client, ClientError> {
        let mut conn_man = ConnectionManager::connect()?;
        let mut queue: EventQueue<Configurations> = conn_man.new_queue();
        let queue_handle = queue.handle();

        let mut configurations = Configurations::default();
        let output_manager: ZwlrOutputManagerV1 = conn_man.bind_global(&queue_handle, 4..=4, ())?;
        trace!("output_manager is binded");
        conn_man.sync()?;
        queue.dispatch_pending(&mut configurations)?;
        debug!("configurations received");


        trace!("started display server");

        Ok(Client {
            configurations,
            output_manager,
            connection_manager: conn_man,
            display_server: None,
            input_server: None,
        })
    }

    pub fn configurations(&self) -> &Configurations {
        &self.configurations
    }

    /// Updates the outputs configurations to match the provided request.
    pub(crate) fn update_configurations(&self, update_request: &UpdateRequest) -> Result<(), String> {
        trace!("received update request: {update_request}");
        let mut config_writer: ConfigWriter =
            config_writer::ConfigWriter::new(&self.connection_manager);
        config_writer.write(update_request, &self.output_manager)
    }

    pub(crate) fn get_display_server(&mut self) -> Result<&mut DisplayServer, ClientError> {
        if self.display_server.is_none() {
            let display_server = DisplayServer::start(&mut self.connection_manager)?;
            self.display_server = Some(display_server);
        }

        Ok(self.display_server.as_mut().expect("Should not happen"))

        // if let Err(err) = display_server.write(text, head) {
        //     return Err(ClientError::Display {
        //         msg: format!("failed to render text: {err}"),
        //     });
        // }
        // Ok(())
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
