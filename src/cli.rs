use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wayout")]
#[command(about = "A wayland tool for wlroot based compositors to configure outputs layout", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Lists all connected outputs")]
    List
}
