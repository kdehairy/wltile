pub mod config_writer;
pub mod configs;
mod connection_manager;
pub mod display;
pub mod errors;
mod output_manager;
pub mod point;
pub mod shmem;
mod wl_compositor;
mod wl_shm;
pub mod wlr_head;
pub mod wlr_mode;

use config_writer::{ConfigWriter, UpdateRequest};
use configs::Configurations;
use display::DisplayServer;
use errors::ClientError;
use tracing::{debug, trace};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

use crate::wlr_client::connection_manager::ConnectionManager;
use crate::wlr_client::point::Point;

/// wlroots client that handles communication with the compositor.
///
/// The client is unusable until the first invokation of `connect()` method.
pub struct Client {
    configurations: Configurations,
    output_manager: ZwlrOutputManagerV1,
    connection_manager: ConnectionManager,
    display_server: Option<DisplayServer>,
}

impl Client {
    /// Connects to the wlroots compositor and receive the outputs configurations.
    pub fn new() -> Result<Client, ClientError> {
        let mut conn_man = ConnectionManager::connect()?;
        let mut queue = conn_man.new_queue();
        let queue_handle = queue.handle();

        let mut configurations = Configurations::default();
        let output_manager: ZwlrOutputManagerV1 = conn_man.bind_global(&queue_handle, 4..=4, ())?;
        trace!("output_manager is binded");
        conn_man.sync()?;
        queue.dispatch_pending(&mut configurations)?;
        debug!("configurations received");

        // let display_server = DisplayServer::start(
        //     &mut conn_man,
        // )?;

        trace!("started display server");

        Ok(Client {
            configurations,
            output_manager,
            connection_manager: conn_man,
            display_server: None,
        })
    }

    pub fn configurations(&self) -> &Configurations {
        &self.configurations
    }

    /// Updates the outputs configurations to match the provided request.
    pub fn update_configurations(&self, update_request: &UpdateRequest) -> Result<(), String> {
        trace!("received update request: {update_request}");
        let mut config_writer: ConfigWriter =
            config_writer::ConfigWriter::new(&self.connection_manager);
        config_writer.write(update_request, &self.output_manager)
    }

    pub fn render_text(&mut self, text: &str, position: Point) -> Result<(), ClientError> {
        trace!(
            "received text '{}' to render at position {}",
            text,
            position
        );

        if self.display_server.is_none() {
            let display_server = DisplayServer::start(&mut self.connection_manager)?;
            self.display_server = Some(display_server);
        }

        let display_server = self.display_server.as_ref().expect("Should not happen");

        if let Err(err) = display_server.write(text) {
            return Err(ClientError::Display {
                msg: format!("failed to render text. {err}"),
            });
        };
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.output_manager.stop();
    }
}
