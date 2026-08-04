//! Output management for wlroots based compositors.
//!
//! It talks to the compositor over the `zwlr-output-management-v1` protocol 
//! and models the desired layout.

pub mod config_file;
pub mod daemon;
pub mod wl_config;

mod commons;
mod functions;
mod wlr_client;

pub use functions::position::exec as position;
pub use functions::set_property::exec as set_property;
pub use functions::show::exec as show;

pub use wlr_client::Client;
pub use wlr_client::errors::ClientError;
pub use wlr_client::output::heads::{Head, Heads, Mode};
pub use wlr_client::output::rotation::Rotation;
pub use wlr_client::point::Point;
