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

        /// Alignment. Defaults to `align-bottom` with `left-of`/`right-of` and
        /// to `align-left` with `top-of`/`bottom-of`.
        #[arg(
            value_name = "ALIGNMENT",
            help = "Defaults to `align-bottom` with `left-of`/`right-of` and to `align-left` with `top-of`/`bottom-of` [possible values: align-bottom, align-top, align-right, align-left]"
        )]
        alignment: Option<Alignment>,
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

/// Builds the [`wl_config::Position`] a `position` command asks for.
///
/// A missing `ALIGNMENT` defaults to the one that keeps the layout reading
/// top-to-bottom, left-to-right: [`Alignment::AlignBottom`] with
/// `left-of`/`right-of`, [`Alignment::AlignLeft`] with `top-of`/`bottom-of`.
/// Both are perpendicular to their relation, as an alignment has to be — the
/// compatibility rule itself lives in `wltile-core`, so the daemon's config
/// file reports the same error from the same place.
///
/// # Errors
///
/// Returns an error if the given relation and alignment act on the same axis;
/// the message names the alignments that would work.
pub(crate) fn position_from_args(
    relation: Relation,
    reference: String,
    alignment: Option<Alignment>,
) -> Result<wl_config::Position, String> {
    let relation = wl_config::Relation::from(relation);
    let alignment = match alignment {
        Some(alignment) => wl_config::Alignment::from(alignment),
        None => match relation {
            wl_config::Relation::LeftOf | wl_config::Relation::RightOf => {
                wl_config::Alignment::AlignBottom
            }
            wl_config::Relation::TopOf | wl_config::Relation::BottomOf => {
                wl_config::Alignment::AlignLeft
            }
        },
    };

    wl_config::Position::new(relation, reference, alignment)
}

#[cfg(test)]
mod tests {
    use super::{
        Alignment, Cli, Commands, Property, Relation, position_from_args, target_from_set,
    };
    use clap::{CommandFactory, Parser};
    use wltile_core::wl_config;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("args should parse")
    }

    #[test]
    fn clap_config_is_valid() {
        Cli::command().debug_assert();
    }

    // --- position_from_args(): the default alignment, plus core's matrix ---

    #[test]
    fn position_defaults_the_alignment_per_relation() {
        // Every default is one the relation accepts: an alignment may not land
        // on the axis the relation already fixed.
        for (relation, expected) in [
            (Relation::LeftOf, wl_config::Alignment::AlignBottom),
            (Relation::RightOf, wl_config::Alignment::AlignBottom),
            (Relation::TopOf, wl_config::Alignment::AlignLeft),
            (Relation::BottomOf, wl_config::Alignment::AlignLeft),
        ] {
            let position =
                position_from_args(relation, String::from("B"), None).unwrap_or_else(|err| {
                    panic!("{relation} without an alignment should get the default: {err}")
                });
            assert_eq!(wl_config::Relation::from(relation), position.relation());
            assert_eq!("B", position.reference());
            assert_eq!(expected, position.alignment(), "{relation} default");

            let (first, second) = wl_config::Relation::from(relation).compatible_alignments();
            assert!(
                expected == first || expected == second,
                "{relation} defaults to {expected}, which it does not accept"
            );
        }
    }

    #[test]
    fn position_rejects_relation_and_alignment_on_the_same_axis() {
        for (relation, alignment) in [
            (Relation::LeftOf, Alignment::AlignLeft),
            (Relation::LeftOf, Alignment::AlignRight),
            (Relation::RightOf, Alignment::AlignLeft),
            (Relation::RightOf, Alignment::AlignRight),
            (Relation::TopOf, Alignment::AlignTop),
            (Relation::TopOf, Alignment::AlignBottom),
            (Relation::BottomOf, Alignment::AlignTop),
            (Relation::BottomOf, Alignment::AlignBottom),
        ] {
            let (relation_name, alignment_name) = (relation.to_string(), alignment.to_string());
            let err = position_from_args(relation, String::from("B"), Some(alignment)).expect_err(
                &format!(
                    "{relation_name} + {alignment_name} act on the same axis and should be rejected"
                ),
            );
            assert!(
                err.contains(&relation_name)
                    && err.contains(&alignment_name)
                    && err.contains("axis"),
                "{relation_name} + {alignment_name}: {err} should name the pair and its axis"
            );
        }
    }

    #[test]
    fn position_accepts_perpendicular_pairs_and_maps_the_fields() {
        for (relation, alignment) in [
            (Relation::LeftOf, Alignment::AlignTop),
            (Relation::LeftOf, Alignment::AlignBottom),
            (Relation::RightOf, Alignment::AlignTop),
            (Relation::RightOf, Alignment::AlignBottom),
            (Relation::TopOf, Alignment::AlignLeft),
            (Relation::TopOf, Alignment::AlignRight),
            (Relation::BottomOf, Alignment::AlignLeft),
            (Relation::BottomOf, Alignment::AlignRight),
        ] {
            let position = position_from_args(relation, String::from("B"), Some(alignment))
                .unwrap_or_else(|err| panic!("{relation} + {alignment} should be accepted: {err}"));
            assert_eq!(wl_config::Relation::from(relation), position.relation());
            assert_eq!(wl_config::Alignment::from(alignment), position.alignment());
            assert_eq!("B", position.reference());
        }
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
    fn position_leaves_the_alignment_unset_when_omitted() {
        // The default is decided by `position_from_args`, per relation: clap
        // must not hand it a value that only fits the horizontal ones.
        match parse(&["wltile", "position", "A", "right-of", "B"]).command {
            Commands::Position { alignment, .. } => assert!(alignment.is_none()),
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
                assert_eq!(Some(Alignment::AlignLeft), alignment);
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
