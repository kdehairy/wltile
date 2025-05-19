pub mod config_writer;
pub mod configs;
mod output_manager;
pub mod wlr_head;
pub mod wlr_mode;
pub mod errors;

use config_writer::{ConfigWriter, UpdateRequest};
use configs::Configurations;
use errors::ClientError;
use tracing::{debug, trace};

use std::fmt::{Debug, Display};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::{self};
use wayland_client::{Connection, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq)]
pub struct Point(pub i32, pub i32);
impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let me = self.0.pow(2).saturating_add(self.1.pow(2));
        let other = other.0.pow(2).saturating_add(other.1.pow(2));
        me.cmp(&other)
    }
}

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
/// The client is unusable until the first invokation of connect() method.
pub struct Client {
    configurations: Option<Configurations>,
    output_manager: Option<ZwlrOutputManagerV1>,
    wlr_connection: Option<Connection>,
}

impl Client {
    pub fn new() -> Self {
        Client {
            configurations: None,
            output_manager: None,
            wlr_connection: None,
        }
    }

    /// Connects to the wlroots compositor and receive the outputs configurations.
    pub fn connect(&mut self) -> Result<(), ClientError> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<Configurations>(&conn)?;
        self.wlr_connection = Some(conn);
        trace!("queue handle acquired");

        let output_manager: ZwlrOutputManagerV1 = globals.bind(&queue.handle(), 4..=4, ())?;
        self.output_manager = Some(output_manager);
        trace!("output_manager acquired");

        let mut configs = Configurations::default();
        queue.roundtrip(&mut configs)?;
        self.configurations = Some(configs);
        debug!("configurations received");
        Ok(())
    }

    pub fn configurations(&self) -> Result<&Configurations, String> {
        self.configurations.as_ref().ok_or(String::from("failed to acquire current configurations"))
    }

    /// Updates the outputs configurations to match the provided request.
    pub fn update_configurations(&self, update_request: &UpdateRequest) -> Result<(), String> {
        trace!("received update request: {update_request}");
        let mut config_writer: ConfigWriter = config_writer::ConfigWriter::new(
            self.wlr_connection
                .as_ref()
                .ok_or("failed to initialize wlr_client")?,
        );
        config_writer.write(
            update_request,
            self.output_manager
                .as_ref()
                .ok_or("failed to initialize wlr_client")?,
        )
    }
}
