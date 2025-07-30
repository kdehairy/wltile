use std::{fs, io::Read, path::Path, str::FromStr};

use serde::Deserialize;
use toml::Table;

use crate::wl_config::{Config, Position, Target};

impl FromStr for Config {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Debug, Deserialize)]
        struct T {
            scale: f32,
            position: String,
        }
        let config_table = s.parse::<Table>().unwrap();
        let mut config = Config::default();
        for target in config_table {
            let t = target.1.try_into::<T>().unwrap();
            let t = Target {
                name: target.0,
                scale: t.scale,
                position: Position::try_from(t.position)?,
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
        assert!(t.scale.eq(&1f32));
        assert_eq!(Relation::LeftOf, t.position.relation);
        assert_eq!("eDP-1", t.position.reference);
        assert_eq!(Alignment::AlignBottom, t.position.alignment);
    }
}
