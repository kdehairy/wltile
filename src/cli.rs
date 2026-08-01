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
        #[arg(
            value_name = "OUTPUT",
            help = "Could be a serial number, name or partial match with the screen make"
        )]
        output: Option<String>,
    },

    #[command(about = "Position outputs")]
    #[command(arg_required_else_help = true)]
    Position {
        /// Output to be positioned
        #[arg(
            value_name = "TARGET_OUTPUT",
            help = "Could be a serial number, name or partial match with the screen make"
        )]
        target: String,

        /// How is it positioned to the reference output
        #[arg(value_name = "RELATION")]
        relation: Relation,

        /// Reference Output
        #[arg(
            value_name = "REFERENCE_OUTPUT",
            help = "Could be a serial number, name or partial match with the screen make"
        )]
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
        #[arg(
            value_name = "TARGET_OUTPUT",
            help = "Could be a serial number, name or partial match with the screen make"
        )]
        target: String,

        #[arg(value_name = "PROPERTY")]
        property: Property,

        #[arg(value_name = "VALUE")]
        value: String,
    },
    #[command(
        about = "Launches wltile as a daemon watching over ouptut configuration changes and applying required configurations"
    )]
    Daemon {
        #[arg(
            value_name = "CONFIG",
            long = "config",
            short = 'c',
            help = "config file path. default: ${XDG_CONFIG_HOME}/config.toml",
            required = false
        )]
        config: Option<String>,
        #[arg(
            value_name = "LOG",
            long = "log",
            short = 'l',
            help = "log file path. default:  ${XDG_STATE_HOME}/logs.log",
            required = false
        )]
        log: Option<String>,
        #[arg(
            value_name = "ERR",
            long = "err",
            short = 'e',
            help = "error log file path. default:  ${XDG_STATE_HOME}/errors.log",
            required = false
        )]
        err_log: Option<String>,
        #[arg(
            value_name = "SYSTEMD",
            long = "systemd",
            short = 'd',
            help = "If the daemon is managed by systemd"
        )]
        systemd: bool,
    },
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Property {
    Mode,
    Scale,
    Rotation,
    Active,
}

// Postfix is functionally needed here
#[allow(clippy::enum_variant_names)]
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
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
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
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
                "Impossible to align top on a vertical setup",
            )),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Alignment, Cli, Commands, Property, Relation, validate};
    use clap::{CommandFactory, Parser};

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("args should parse")
    }

    #[test]
    fn clap_config_is_valid() {
        Cli::command().debug_assert();
    }

    // --- validate(): the relation x alignment compatibility matrix ---

    #[test]
    fn validate_rejects_horizontal_relation_with_horizontal_alignment() {
        for relation in ["left-of", "right-of"] {
            for alignment in ["align-left", "align-right"] {
                let cli = parse(&["wltile", "position", "A", relation, "B", alignment]);
                assert!(
                    validate(&cli).is_err(),
                    "{relation} + {alignment} should be rejected"
                );
            }
        }
    }

    #[test]
    fn validate_rejects_vertical_relation_with_vertical_alignment() {
        for relation in ["top-of", "bottom-of"] {
            for alignment in ["align-top", "align-bottom"] {
                let cli = parse(&["wltile", "position", "A", relation, "B", alignment]);
                assert!(
                    validate(&cli).is_err(),
                    "{relation} + {alignment} should be rejected"
                );
            }
        }
    }

    #[test]
    fn validate_accepts_horizontal_relation_with_vertical_alignment() {
        for relation in ["left-of", "right-of"] {
            for alignment in ["align-top", "align-bottom"] {
                let cli = parse(&["wltile", "position", "A", relation, "B", alignment]);
                assert!(
                    validate(&cli).is_ok(),
                    "{relation} + {alignment} should be accepted"
                );
            }
        }
    }

    #[test]
    fn validate_accepts_vertical_relation_with_horizontal_alignment() {
        for relation in ["top-of", "bottom-of"] {
            for alignment in ["align-left", "align-right"] {
                let cli = parse(&["wltile", "position", "A", relation, "B", alignment]);
                assert!(
                    validate(&cli).is_ok(),
                    "{relation} + {alignment} should be accepted"
                );
            }
        }
    }

    #[test]
    fn validate_ignores_non_position_commands() {
        assert!(validate(&parse(&["wltile", "list"])).is_ok());
        assert!(validate(&parse(&["wltile", "show"])).is_ok());
    }

    // --- parsing round-trips ---

    #[test]
    fn parses_list() {
        assert!(matches!(parse(&["wltile", "list"]).command, Commands::List));
    }

    #[test]
    fn parses_show_without_output() {
        assert!(matches!(
            parse(&["wltile", "show"]).command,
            Commands::Show { output: None }
        ));
    }

    #[test]
    fn parses_show_with_output() {
        match parse(&["wltile", "show", "HDMI-1"]).command {
            Commands::Show { output } => assert_eq!(output.as_deref(), Some("HDMI-1")),
            _ => panic!("expected the Show command"),
        }
    }

    #[test]
    fn position_defaults_alignment_to_align_bottom() {
        match parse(&["wltile", "position", "A", "right-of", "B"]).command {
            Commands::Position { alignment, .. } => {
                assert_eq!(alignment, Alignment::AlignBottom);
            }
            _ => panic!("expected the Position command"),
        }
    }

    #[test]
    fn parses_position_with_all_fields() {
        match parse(&["wltile", "position", "A", "top-of", "B", "align-left"]).command {
            Commands::Position {
                target,
                relation,
                reference,
                alignment,
            } => {
                assert_eq!(target, "A");
                assert_eq!(relation, Relation::TopOf);
                assert_eq!(reference, "B");
                assert_eq!(alignment, Alignment::AlignLeft);
            }
            _ => panic!("expected the Position command"),
        }
    }

    #[test]
    fn parses_set_with_property_value_enum() {
        match parse(&["wltile", "set", "HDMI-1", "scale", "2"]).command {
            Commands::Set {
                target,
                property,
                value,
            } => {
                assert_eq!(target, "HDMI-1");
                assert_eq!(property, Property::Scale);
                assert_eq!(value, "2");
            }
            _ => panic!("expected the Set command"),
        }
    }

    #[test]
    fn parses_daemon_with_all_flags() {
        match parse(&[
            "wltile", "daemon", "--config", "/c.toml", "--log", "/l.log", "--err", "/e.log",
            "--systemd",
        ])
        .command
        {
            Commands::Daemon {
                config,
                log,
                err_log,
                systemd,
            } => {
                assert_eq!(config.as_deref(), Some("/c.toml"));
                assert_eq!(log.as_deref(), Some("/l.log"));
                assert_eq!(err_log.as_deref(), Some("/e.log"));
                assert!(systemd);
            }
            _ => panic!("expected the Daemon command"),
        }
    }

    #[test]
    fn parses_daemon_with_defaults() {
        match parse(&["wltile", "daemon"]).command {
            Commands::Daemon {
                config,
                log,
                err_log,
                systemd,
            } => {
                assert!(config.is_none());
                assert!(log.is_none());
                assert!(err_log.is_none());
                assert!(!systemd);
            }
            _ => panic!("expected the Daemon command"),
        }
    }

    #[test]
    fn counts_verbosity_flags() {
        assert_eq!(parse(&["wltile", "list"]).verbose, 0);
        assert_eq!(parse(&["wltile", "-vvv", "list"]).verbose, 3);
    }

    #[test]
    fn rejects_unknown_property() {
        assert!(Cli::try_parse_from(["wltile", "set", "A", "bogus", "1"]).is_err());
    }

    #[test]
    fn position_requires_arguments() {
        assert!(Cli::try_parse_from(["wltile", "position"]).is_err());
    }

    // --- Display impls ---

    #[test]
    fn relation_display_renders_kebab_case() {
        assert_eq!(Relation::LeftOf.to_string(), "left-of");
        assert_eq!(Relation::RightOf.to_string(), "right-of");
        assert_eq!(Relation::TopOf.to_string(), "top-of");
        assert_eq!(Relation::BottomOf.to_string(), "bottom-of");
    }

    #[test]
    fn alignment_display_renders_kebab_case() {
        assert_eq!(Alignment::AlignBottom.to_string(), "align-bottom");
        assert_eq!(Alignment::AlignTop.to_string(), "align-top");
        assert_eq!(Alignment::AlignRight.to_string(), "align-right");
        assert_eq!(Alignment::AlignLeft.to_string(), "align-left");
    }
}
