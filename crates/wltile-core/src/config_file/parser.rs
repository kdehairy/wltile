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
    /// Returns an error if `s` is not valid TOML, if a table has a property of
    /// the wrong type, or if a `position` value is not a valid [`Position`]
    /// (`"<relation> <reference> <alignment>"`, with the relation and the
    /// alignment on different axes). Every message but the first names the
    /// section it came from, e.g. `[dell]: …`, and the TOML and type errors
    /// carry the line and column the parser reported.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Debug, Deserialize)]
        struct T {
            scale: Option<f64>,
            mode: Option<usize>,
            rotation: Option<i32>,
            position: Option<String>,
        }
        let config_table = s
            .parse::<Table>()
            .map_err(|err| format!("invalid TOML: {err}"))?;
        let mut config = Config::default();
        for target in config_table {
            let t = target
                .1
                .try_into::<T>()
                .map_err(|err| format!("[{}]: {err}", target.0))?;
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
    /// Returns an error if the file cannot be opened or read, or the same
    /// errors as parsing the contents with [`str::parse`]. The I/O messages
    /// name `path` and the OS error behind them.
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        debug!("parsing file {:?}", path);
        let mut file = fs::File::open(path)
            .map_err(|err| format!("failed to open config file '{}': {err}", path.display()))?;
        let mut buff = String::new();
        file.read_to_string(&mut buff)
            .map_err(|err| format!("failed to read config file '{}': {err}", path.display()))?;
        buff.parse::<Config>()
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
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

    // Everything below is on the daemon's per-reload path: an error there is
    // logged and that reload skipped, while a panic takes the reload thread
    // down for good and the daemon never applies a config file again.

    #[test]
    fn accepts_an_empty_config_file_as_no_targets() {
        // `wltile daemon` creates an empty default config on first run, so the
        // file the daemon reads before the user has written anything has to
        // parse — not report a missing section or a panic.
        let config: Config = String::new()
            .parse()
            .expect("an empty config file is a config with no targets");
        assert!(config.targets().is_empty());
    }

    #[test]
    fn rejects_invalid_toml_with_the_parsers_own_report() {
        let err = "[dell]\nscale = "
            .parse::<Config>()
            .expect_err("a `scale` with no value is not TOML");
        assert!(
            err.starts_with("invalid TOML"),
            "{err} should say the TOML is invalid"
        );
        assert!(
            err.contains("line"),
            "{err} should carry the parser's location: {err}"
        );
    }

    #[test]
    fn rejects_a_property_of_the_wrong_type_naming_the_section() {
        let cases = [
            ("[dell]\nscale = \"two\"\n", "scale"),
            ("[dell]\nmode = \"big\"\n", "mode"),
            ("[dell]\nrotation = \"a quarter turn\"\n", "rotation"),
            ("[dell]\nposition = 12\n", "position"),
        ];
        for (contents, property) in cases {
            let err = contents.parse::<Config>().expect_err(&format!(
                "`{property}` should reject a string that is not one"
            ));
            assert!(err.starts_with("[dell]"), "{err} should name the section");
            assert!(err.contains(property), "{err} should name the property");
        }
    }

    #[test]
    fn reports_a_config_file_it_cannot_open_naming_the_path() {
        let path = env::temp_dir().join("wltile-parser-absent-config.toml");
        let _ = fs::remove_file(&path);

        let err = Config::try_from(path.as_path()).expect_err("an absent file cannot be parsed");
        assert!(
            err.contains("config file"),
            "{err} should say what could not be done"
        );
        assert!(
            err.contains(&path.display().to_string()),
            "{err} should name {}",
            path.display()
        );
    }

    #[test]
    fn reports_a_config_file_it_cannot_read_naming_the_path() {
        // A directory opens fine on Linux and fails on the read, which is the
        // second I/O failure mode the daemon can hit — a config path a user
        // pointed at a folder, say.
        let dir = env::temp_dir().join("wltile-parser-directory-as-config");
        fs::create_dir_all(&dir).expect("failed to create temp dir");

        let err = Config::try_from(dir.as_path()).expect_err("a directory is not a config file");
        assert!(
            err.contains("config file"),
            "{err} should say what could not be done"
        );
        assert!(
            err.contains(&dir.display().to_string()),
            "{err} should name {}",
            dir.display()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reports_a_config_file_that_is_not_utf8() {
        let dir = env::temp_dir().join("wltile-parser-non-utf8-config");
        let path = dir.join("config.toml");
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        fs::write(&path, [0xff_u8, 0xfe, 0x00]).expect("failed to write temp config");

        let err = Config::try_from(path.as_path()).expect_err("bytes that are not UTF-8 fail here");
        assert!(
            err.contains("config file"),
            "{err} should say what could not be done"
        );
        assert!(
            err.contains(&path.display().to_string()),
            "{err} should name {}",
            path.display()
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
