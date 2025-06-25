pub mod config_writer;
pub mod configs;
pub mod errors;
mod output_manager;
pub mod point;
pub mod wlr_head;
pub mod wlr_mode;
pub mod shmem;

use config_writer::{ConfigWriter, UpdateRequest};
use configs::Configurations;
use errors::ClientError;
use tracing::{debug, trace};

use wayland_client::globals::{registry_queue_init, GlobalList, GlobalListContents};
use wayland_client::protocol::wl_registry::{self};
use wayland_client::{Connection, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Configurations {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

/// wlroots client that handles communication with the compositor.
///
/// The client is unusable until the first invokation of `connect()` method.
pub struct Client {
    globals: GlobalList,
    configurations: Configurations,
    output_manager: ZwlrOutputManagerV1,
    wlr_connection: Connection,
}

impl Client {
    /// Connects to the wlroots compositor and receive the outputs configurations.
    pub fn new() -> Result<Client, ClientError> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<Configurations>(&conn)?;
        trace!("queue handle acquired");

        let output_manager: ZwlrOutputManagerV1 = globals.bind(&queue.handle(), 4..=4, ())?;
        trace!("output_manager acquired");

        // globals.contents().with_list(|list| {
        //     for i in list {
        //         println!("{}", i.interface);
        //     }
        // });

        let mut configurations = Configurations::default();
        queue.roundtrip(&mut configurations)?;
        debug!("configurations received");
        Ok(
            Client {
                globals,
                configurations,
                output_manager,
                wlr_connection: conn,
            }
)
    }

    pub fn configurations(&self) -> &Configurations {
        &self.configurations
    }

    /// Updates the outputs configurations to match the provided request.
    pub fn update_configurations(&self, update_request: &UpdateRequest) -> Result<(), String> {
        trace!("received update request: {update_request}");
        let mut config_writer: ConfigWriter = config_writer::ConfigWriter::new(
            &self.wlr_connection
        );
        config_writer.write(
            update_request,
            &self.output_manager
        )
    }
}
