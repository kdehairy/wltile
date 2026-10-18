//! Output management for wlroots based compositors.
//!
//! It talks to the compositor over the `zwlr-output-management-v1` protocol
//! and models the desired layout.
//!
//! Everything starts with a [`Client`], which connects to the compositor and
//! keeps a live cache of its outputs ([`Heads`]). Changes are described as a
//! [`wl_config::Config`] and applied with [`position`] or [`set_property`].
//!
//! ```no_run
//! use wltile_core::Client;
//! use wltile_core::wl_config::{Config, Target};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new()?;
//! for head in client.heads()?.heads() {
//!     println!("{} at {}", head.name(), head.position());
//! }
//!
//! let mut config = Config::default();
//! config.add_target(Target {
//!     scale: Some(2.0),
//!     ..Target::new("eDP-1".into())
//! });
//! wltile_core::set_property(&config, &client)?;
//! # Ok(())
//! # }
//! ```
#![warn(missing_docs)]

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
