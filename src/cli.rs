use std::fmt::Display;

use clap::{Parser, Subcommand, ValueEnum};

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
    List,

    #[command(about = "Position outputs")]
    #[command(arg_required_else_help = true)]
    Position {
        /// Output to be positioned
        #[arg(value_name="TARGET_OUTPUT")]
        target: String,

        /// How is it positioned to the reference output
        #[arg(value_name="RELATION")]
        relation: Relation, 

        /// Reference Output
        #[arg(value_name="REFERENCE_OUTPUT")]
        reference: String,

        /// Alignment
        #[arg(value_name="ALIGNMENT", default_value_t= Alignment::AlignBottom)]
        alignment: Alignment,
    },
}

#[derive(ValueEnum, Copy, Clone, PartialEq, Eq)]
pub enum Relation {
    LeftOf,
    RightOf,
}

impl Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Relation::LeftOf => write!(f, "left of"),
            Relation::RightOf => write!(f, "right of"),
        }
    }
}

#[derive(ValueEnum, Copy, Clone, PartialEq, Eq)]
pub enum Alignment {
    AlignBottom,
    AlignTop,
}

impl Display for Alignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Alignment::AlignBottom => write!(f, "align-bottom"),
            Alignment::AlignTop => write!(f, "align-top"),
        }
    }
}
