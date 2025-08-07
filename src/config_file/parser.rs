use std::{fs, io::Read, path::Path, str::FromStr};

use serde::Deserialize;
use toml::Table;

use crate::wl_config::{Config, Position, Target};

impl FromStr for Config {
    type Err = String;

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
                Some(position) => Some(Position::try_from(position)?),
                None => None,
            };
            let t = Target {
                name: target.0,
                scale: t.scale,
                mode: t.mode,
                rotation: t.rotation,
                position,
            };
            config.add_target(t);
        }

        Ok(config)
    }
}

impl TryFrom<&Path> for Config {
    type Error = String;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
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
        assert_eq!(Relation::LeftOf, t.position.as_ref().unwrap().relation);
        assert_eq!("eDP-1", t.position.as_ref().unwrap().reference);
        assert_eq!(Alignment::AlignBottom, t.position.as_ref().unwrap().alignment);
    }
}
