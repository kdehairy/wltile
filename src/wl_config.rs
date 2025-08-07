#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq)]
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

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Alignment {
    AlignBottom,
    AlignTop,
    AlignRight,
    AlignLeft,
}

impl From<crate::cli::Alignment> for Alignment {
    fn from(value: crate::cli::Alignment) -> Self {
        match value{
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

#[derive(Debug)]
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
            None => return Err(String::from("Missing relation")),
        };

        let alignment: Alignment = match iter.next() {
            Some(align) => Alignment::try_from(align)?,
            None => return Err(String::from("Missing relation")),
        };

        Ok(Position {
            relation,
            reference,
            alignment,
        })
    }
}

#[derive(Debug, Default)]
pub struct Target {
    pub name: String,
    pub position: Option<Position>,
    pub scale: Option<f64>,
    pub mode: Option<usize>,
    pub rotation: Option<i32>,
}
impl Target {
    pub(crate) fn new(name: String) -> Self {
        Self { name, ..Default::default() }
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
