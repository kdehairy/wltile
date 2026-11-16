//! TOML parsing for [`Config`].
//!
//! This module adds two ways to build a [`Config`]: [`str::parse`] (through
//! `FromStr`) and `Config::try_from(&Path)`. Each top-level table is one
//! [`Target`], keyed by the output it applies to:
//!
//! ```toml
//! [eDP-1]
//! scale = 2
//!
//! [dell]
//! position = "left-of eDP-1 align-bottom"
//! mode = 0
//! rotation = 90
//! ```
//!
//! The `active` property can't be set from a config file.

use std::{fs, io::Read, path::Path, str::FromStr};

use serde::Deserialize;
use toml::Table;
use tracing::debug;

use crate::wl_config::{Config, Position, Target};

impl FromStr for Config {
    type Err = String;

    /// Parses a config file's contents.
    ///
    /// # Errors
    ///
    /// Returns an error if a `position` value is not a valid [`Position`]
    /// (`"<relation> <reference> <alignment>"`, with the relation and the
    /// alignment on different axes). The message names the section it came
    /// from, e.g. `[dell]: …`.
    ///
    /// # Panics
    ///
    /// Panics if `s` is not valid TOML, or if a table has a property of the
    /// wrong type.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Debug, Deserialize)]
        struct T {
            scale: Option<f64>,
            mode: Option<usize>,
            rotation: Option<i32>,
            position: Option<String>,
        }
        let config_table = s.parse::<Table>().unwrap();
        let mut config = Config::default();
        for target in config_table {
            let t = target.1.try_into::<T>().unwrap();
            let position = match t.position {
                Some(position) => Some(
                    Position::try_from(position).map_err(|err| format!("[{}]: {err}", target.0))?,
                ),
                None => None,
            };
            let t = Target {
                name: target.0,
                scale: t.scale,
                mode: t.mode,
                rotation: t.rotation,
                position,
                active: None,
            };
            config.add_target(t);
        }

        Ok(config)
    }
}

impl TryFrom<&Path> for Config {
    type Error = String;

    /// Reads and parses the config file at `path`.
    ///
    /// # Errors
    ///
    /// Returns the same errors as parsing the contents with [`str::parse`].
    ///
    /// # Panics
    ///
    /// Panics if the file can't be opened or read, or in the same cases
    /// as parsing the contents with [`str::parse`].
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        debug!("parsing file {:?}", path);
        let mut file = fs::File::open(path).unwrap();
        let mut buff = String::new();
        file.read_to_string(&mut buff).unwrap();
        buff.parse::<Config>()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::wl_config::{Alignment, Config, Relation};

    #[test]
    fn test_parsing() {
        let config = Config::try_from(Path::new("src/config_file/test_config.toml")).unwrap();
        assert_eq!(1, config.targets().len());
        let t = config.targets().first().unwrap();
        assert_eq!("DP-2", t.name);
        assert!(t.scale.unwrap().eq(&1f64));
        let position = t.position.as_ref().unwrap();
        assert_eq!(Relation::LeftOf, position.relation());
        assert_eq!("eDP-1", position.reference());
        assert_eq!(Alignment::AlignBottom, position.alignment());
    }

    #[test]
    fn accepts_every_valid_position_pairing() {
        let config: Config = r#"
[a]
position = "left-of eDP-1 align-top"

[b]
position = "right-of eDP-1 align-bottom"

[c]
position = "top-of eDP-1 align-left"

[d]
position = "bottom-of eDP-1 align-right"
"#
        .parse()
        .expect("every relation with a perpendicular alignment is valid");

        assert_eq!(4, config.targets().len());
        for (name, relation, alignment) in [
            ("a", Relation::LeftOf, Alignment::AlignTop),
            ("b", Relation::RightOf, Alignment::AlignBottom),
            ("c", Relation::TopOf, Alignment::AlignLeft),
            ("d", Relation::BottomOf, Alignment::AlignRight),
        ] {
            let target = config
                .targets()
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("target [{name}] missing"));
            let position = target.position.as_ref().expect("target has a position");
            assert_eq!(relation, position.relation(), "[{name}] relation");
            assert_eq!(alignment, position.alignment(), "[{name}] alignment");
        }
    }

    #[test]
    fn rejects_a_position_whose_relation_and_alignment_share_an_axis() {
        // The CLI rejects these four pairs; the config file has to as well,
        // otherwise the daemon applies a layout where the outputs overlap.
        for position in [
            "top-of eDP-1 align-top",
            "top-of eDP-1 align-bottom",
            "bottom-of eDP-1 align-top",
            "bottom-of eDP-1 align-bottom",
            "left-of eDP-1 align-left",
            "left-of eDP-1 align-right",
            "right-of eDP-1 align-left",
            "right-of eDP-1 align-right",
        ] {
            let contents = format!("[dell]\nposition = \"{position}\"\n");
            let err = contents
                .parse::<Config>()
                .expect_err(&format!("`{position}` should be rejected"));
            assert!(err.starts_with("[dell]"), "{err} should name the section");
            assert!(err.contains("axis"), "{err} should name the axis");
        }
    }
}
