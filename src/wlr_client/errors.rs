use std::fmt::Display;

use wayland_client::{
    backend::WaylandError,
    globals::{BindError, GlobalError},
    ConnectError, DispatchError,
};

#[derive(Debug)]
pub enum ClientError {
    Connection { msg: String },
    Binding { msg: String },
    Dispatch { msg: String },
    Display { msg: String },
}

impl From<GlobalError> for ClientError {
    fn from(value: GlobalError) -> Self {
        match value {
            GlobalError::Backend(err) => ClientError::Connection {
                msg: err.to_string(),
            },
            GlobalError::InvalidId(err) => ClientError::Connection {
                msg: err.to_string(),
            },
        }
    }
}

impl From<WaylandError> for ClientError {
    fn from(value: WaylandError) -> Self {
        match value {
            WaylandError::Io(err) => ClientError::Connection {
                msg: err.to_string(),
            },
            WaylandError::Protocol(err) => ClientError::Connection {
                msg: err.to_string(),
            },
        }
    }
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
            ClientError::Display { msg } => {
                write!(f, "failed to render on display: {msg}")
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
