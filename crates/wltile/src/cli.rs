use std::fmt::Display;

use clap::{Parser, Subcommand, ValueEnum};
use wltile_core::wl_config;

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

impl From<Relation> for wl_config::Relation {
    fn from(value: Relation) -> Self {
        match value {
            Relation::LeftOf => Self::LeftOf,
            Relation::RightOf => Self::RightOf,
            Relation::TopOf => Self::TopOf,
            Relation::BottomOf => Self::BottomOf,
        }
    }
}

impl From<Alignment> for wl_config::Alignment {
    fn from(value: Alignment) -> Self {
        match value {
            Alignment::AlignBottom => Self::AlignBottom,
            Alignment::AlignTop => Self::AlignTop,
            Alignment::AlignRight => Self::AlignRight,
            Alignment::AlignLeft => Self::AlignLeft,
        }
    }
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

/// Interprets a `set` command's `PROPERTY VALUE` pair into a target the library
/// can apply.
///
/// The value arrives as an opaque string because the property decides how to
/// read it.
pub(crate) fn target_from_set(
    name: String,
    property: Property,
    value: &str,
) -> Result<wl_config::Target, String> {
    let mut target = wl_config::Target::new(name);
    let value = value.trim();
    match property {
        Property::Mode => {
            target.mode = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| String::from("Expected integer identifier for the mode"))?,
            );
        }
        Property::Scale => {
            target.scale = Some(
                value
                    .parse::<f64>()
                    .map_err(|_| String::from("Expected number identifier for the scale"))?,
            );
        }
        Property::Rotation => {
            target.rotation = Some(
                value
                    .parse::<i32>()
                    .map_err(|_| String::from("Expected number identifier for the rotation"))?,
            );
        }
        Property::Active => {
            target.active = Some(
                value
                    .parse::<bool>()
                    .map_err(|_| String::from("Expected 'true' or 'false' for active"))?,
            );
        }
    }
    Ok(target)
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
            (Relation::TopOf | Relation::BottomOf, Alignment::AlignTop) => {
                Err(String::from("Impossible to align top on a vertical setup"))
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Alignment, Cli, Commands, Property, Relation, target_from_set, validate};
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
            "wltile",
            "daemon",
            "--config",
            "/c.toml",
            "--log",
            "/l.log",
            "--err",
            "/e.log",
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

    #[test]
    fn set_parses_each_property_onto_its_own_field() {
        let target = target_from_set(String::from("A"), Property::Mode, "2").expect("valid mode");
        assert_eq!(target.mode, Some(2));

        let target = target_from_set(String::from("A"), Property::Scale, "1.5").expect("valid");
        assert_eq!(target.scale, Some(1.5));

        let target = target_from_set(String::from("A"), Property::Rotation, "-90").expect("valid");
        assert_eq!(target.rotation, Some(-90));

        let target = target_from_set(String::from("A"), Property::Active, "false").expect("valid");
        assert_eq!(target.active, Some(false));
    }

    #[test]
    fn set_trims_surrounding_whitespace() {
        let target = target_from_set(String::from("A"), Property::Scale, "  2  ").expect("valid");
        assert_eq!(target.scale, Some(2.0));
    }

    #[test]
    fn set_rejects_values_that_do_not_match_the_property() {
        let cases = [
            (
                Property::Mode,
                "1.5",
                "Expected integer identifier for the mode",
            ),
            (
                Property::Scale,
                "big",
                "Expected number identifier for the scale",
            ),
            (
                Property::Rotation,
                "1.5",
                "Expected number identifier for the rotation",
            ),
            (
                Property::Active,
                "yes",
                "Expected 'true' or 'false' for active",
            ),
        ];
        for (property, value, expected) in cases {
            let err = target_from_set(String::from("A"), property, value)
                .expect_err("{property:?} should reject {value}");
            assert_eq!(err, expected, "{property:?} rejecting {value}");
        }
    }

    #[test]
    fn set_keeps_the_target_name() {
        let target = target_from_set(String::from("HDMI-1"), Property::Scale, "1").expect("valid");
        assert_eq!(target.name, "HDMI-1");
    }

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
