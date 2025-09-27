use std::fmt::Display;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    LeftOf,
    RightOf,
    TopOf,
    BottomOf,
}

impl TryFrom<&str> for Relation {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "left-of" => Ok(Self::LeftOf),
            "right-of" => Ok(Self::RightOf),
            "top-of" => Ok(Self::TopOf),
            "bottom-of" => Ok(Self::BottomOf),
            other => Err(format!("Unknown relation '{other}'")),
        }
    }
}

impl From<crate::cli::Relation> for Relation {
    fn from(value: crate::cli::Relation) -> Self {
        match value {
            crate::cli::Relation::LeftOf => Self::LeftOf,
            crate::cli::Relation::RightOf => Self::RightOf,
            crate::cli::Relation::TopOf => Self::TopOf,
            crate::cli::Relation::BottomOf => Self::BottomOf,
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

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Alignment {
    AlignBottom,
    AlignTop,
    AlignRight,
    AlignLeft,
}

impl From<crate::cli::Alignment> for Alignment {
    fn from(value: crate::cli::Alignment) -> Self {
        match value {
            crate::cli::Alignment::AlignBottom => Alignment::AlignBottom,
            crate::cli::Alignment::AlignTop => Alignment::AlignTop,
            crate::cli::Alignment::AlignRight => Alignment::AlignRight,
            crate::cli::Alignment::AlignLeft => Alignment::AlignLeft,
        }
    }
}

impl TryFrom<&str> for Alignment {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "align-bottom" => Ok(Self::AlignBottom),
            "align-top" => Ok(Self::AlignTop),
            "align-right" => Ok(Self::AlignRight),
            "align-left" => Ok(Self::AlignLeft),
            other => Err(format!("Unknown alignment '{other}'")),
        }
    }
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

#[derive(Debug, Clone)]
pub struct Position {
    pub relation: Relation,
    pub reference: String,
    pub alignment: Alignment,
}

impl TryFrom<String> for Position {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut iter = value.split_whitespace();
        let relation: Relation = match iter.next() {
            Some(rel) => Relation::try_from(rel)?,
            None => return Err(String::from("Missing relation")),
        };

        let reference: String = match iter.next() {
            Some(refer) => String::from(refer),
            None => return Err(String::from("Missing reference")),
        };

        let alignment: Alignment = match iter.next() {
            Some(align) => Alignment::try_from(align)?,
            None => return Err(String::from("Missing alignment")),
        };

        Ok(Position {
            relation,
            reference,
            alignment,
        })
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.relation, self.reference, self.alignment)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Target {
    pub name: String,
    pub position: Option<Position>,
    pub scale: Option<f64>,
    pub mode: Option<usize>,
    pub rotation: Option<i32>,
}
impl Target {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Target:{}", self.name)?;
        if let Some(position) = self.position.as_ref() {
            writeln!(f, "\tposition: {position}")?;
        }
        if let Some(scale) = self.scale {
            writeln!(f, "\tscale: {scale}")?;
        }
        if let Some(mode) = self.mode {
            writeln!(f, "\tmode: {mode}")?;
        }
        if let Some(rotation) = self.rotation {
            writeln!(f, "\trotation: {rotation}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Config {
    targets: Vec<Target>,
}

impl Config {
    pub fn add_target(&mut self, target: Target) {
        self.targets.push(target);
    }

    pub fn targets(&self) -> &Vec<Target> {
        &self.targets
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Config:")?;
        for t in &self.targets {
            writeln!(f, "\t{t}")?;
        }

        Ok(())
    }
}
