use std::fmt::Display;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "wltile")]
#[command(about = "A tool for wlroot based compositors to configure outputs layout", long_about = None)]
pub struct Cli {
    #[arg(short='v', action = clap::ArgAction::Count)]
    pub verbose: u8,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Lists all connected outputs")]
    List,

    #[command(
        about = "Shows the current layout. If an output is provided as argument, it shows detailed info for the specified output"
    )]
    Show {
        #[arg(value_name = "OUTPUT")]
        output: Option<String>,
    },

    #[command(about = "Position outputs")]
    #[command(arg_required_else_help = true)]
    Position {
        /// Output to be positioned
        #[arg(value_name = "TARGET_OUTPUT")]
        target: String,

        /// How is it positioned to the reference output
        #[arg(value_name = "RELATION")]
        relation: Relation,

        /// Reference Output
        #[arg(value_name = "REFERENCE_OUTPUT")]
        reference: String,

        /// Alignment
        #[arg(value_name="ALIGNMENT", default_value_t= Alignment::AlignBottom)]
        alignment: Alignment,
    },
    #[command(
        about = "Sets properties of the output to a desired value",
        arg_required_else_help = true
    )]
    Set {
        #[arg(value_name = "TARGET_OUTPUT")]
        target: String,

        #[arg(value_name = "PROPERTY")]
        property: Property,

        #[arg(value_name = "VALUE")]
        value: String,
    },
}

#[derive(ValueEnum, Copy, Clone, PartialEq, Eq)]
pub enum Property {
    Mode,
    Scale,
    Rotation,
}

// Postfix is functionally needed here
#[allow(clippy::enum_variant_names)]
#[derive(ValueEnum, Copy, Clone, PartialEq, Eq)]
pub enum Relation {
    LeftOf,
    RightOf,
    TopOf,
    BottomOf,
}

impl Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Relation::LeftOf => write!(f, "left-of"),
            Relation::RightOf => write!(f, "right-of"),
            Relation::TopOf => write!(f, "top-of"),
            Relation::BottomOf => write!(f, "bottom-of"),
        }
    }
}

// Prefix is functionally needed here
#[allow(clippy::enum_variant_names)]
#[derive(ValueEnum, Copy, Clone, PartialEq, Eq)]
pub enum Alignment {
    AlignBottom,
    AlignTop,
    AlignRight,
    AlignLeft,
}

impl Display for Alignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Alignment::AlignBottom => write!(f, "align-bottom"),
            Alignment::AlignTop => write!(f, "align-top"),
            Alignment::AlignRight => write!(f, "align-right"),
            Alignment::AlignLeft => write!(f, "align-left"),
        }
    }
}

pub(crate) fn validate(args: &Cli) -> Result<(), String> {
    match &args.command {
        Commands::Position {
            relation,
            alignment,
            ..
        } => match (relation, alignment) {
            (Relation::LeftOf | Relation::RightOf, Alignment::AlignRight) => Err(String::from(
                "Impossible to align right on a horizontal setup",
            )),
            (Relation::RightOf | Relation::LeftOf, Alignment::AlignLeft) => Err(String::from(
                "Impossible to align left on a horizontal setup",
            )),
            (Relation::TopOf | Relation::BottomOf, Alignment::AlignBottom) => Err(String::from(
                "Impossible to align bottom on a vertical setup",
            )),
            (Relation::TopOf | Relation::BottomOf, Alignment::AlignTop) => Err(String::from(
                "Impossible to align top on a horizontal setup",
            )),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}
