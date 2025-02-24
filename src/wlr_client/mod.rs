pub mod config_writer;
pub mod configs;
mod output_manager;
pub mod wlr_head;
pub mod wlr_mode;

use config_writer::{ConfigWriter, UpdateRequest};
use configs::Configurations;

use std::fmt::{Debug, Display};
use wayland_client::globals::{registry_queue_init, BindError, GlobalListContents};
use wayland_client::protocol::wl_registry::{self};
use wayland_client::{ConnectError, Connection, Dispatch, DispatchError};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Point(pub i32, pub i32);
impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

#[derive(Debug)]
pub enum ClientError {
    Connection { msg: String },
    Binding { msg: String },
    Dispatch { msg: String },
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ClientError::Connection { msg } => {
                write!(f, "failed to connect to wayland server: {msg}")
            }
            ClientError::Binding { msg } => {
                write!(f, "failed to bind to wayland object: {msg}")
            }
            ClientError::Dispatch { msg } => {
                write!(f, "failed to dispatch message: {msg}")
            }
        }
    }
}

impl From<ConnectError> for ClientError {
    fn from(value: ConnectError) -> Self {
        match value {
            ConnectError::NoWaylandLib | ConnectError::InvalidFd | ConnectError::NoCompositor => {
                ClientError::Connection {
                    msg: format!("{value}"),
                }
            }
        }
    }
}

impl From<BindError> for ClientError {
    fn from(value: BindError) -> Self {
        match value {
            BindError::UnsupportedVersion | BindError::NotPresent => ClientError::Binding {
                msg: format!("{value}"),
            },
        }
    }
}

impl From<DispatchError> for ClientError {
    fn from(value: DispatchError) -> Self {
        match value {
            DispatchError::BadMessage {
                sender_id: ref _i,
                interface: _,
                opcode: _,
            } => ClientError::Dispatch {
                msg: format!("{value}"),
            },
            DispatchError::Backend(ref _i) => ClientError::Dispatch {
                msg: format!("{value}"),
            },
        }
    }
}

impl std::error::Error for ClientError {}

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

    pub fn connect(&mut self) -> Result<(), ClientError> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<Configurations>(&conn).unwrap();
        self.wlr_connection = Some(conn);
        log::debug!("queue handle acquired");

        let output_manager: ZwlrOutputManagerV1 = globals.bind(&queue.handle(), 4..=4, ())?;
        self.output_manager = Some(output_manager);
        log::debug!("output_manager acquired");

        let mut configs = Configurations::default();
        queue.roundtrip(&mut configs)?;
        self.configurations = Some(configs);
        log::debug!("configurations received");
        Ok(())
    }

    pub fn configurations(&self) -> Option<&Configurations> {
        self.configurations.as_ref()
    }

    pub fn update_configurations(&self, update_request: &UpdateRequest) -> bool {
        let mut config_writer: ConfigWriter =
            config_writer::ConfigWriter::new(self.wlr_connection.as_ref().unwrap());
        config_writer.write(update_request, self.output_manager.as_ref().unwrap())
    }
}
