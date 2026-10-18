//! The desired output configuration: which outputs to change, and how.
//!
//! A [`Config`] holds one [`Target`] per output. Apply it with
//! [`position`](crate::position) for the [`Position`] of each target, and
//! with [`set_property`](crate::set_property) for everything else.

use std::fmt::Display;

#[allow(clippy::enum_variant_names)]
/// Where a target output sits relative to its reference output.
///
/// Parses from and displays as `left-of`, `right-of`, `top-of` and
/// `bottom-of`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    /// To the left of the reference output.
    LeftOf,
    /// To the right of the reference output.
    RightOf,
    /// Above the reference output.
    TopOf,
    /// Below the reference output.
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
/// Which edges of the target and reference outputs line up.
///
/// Use [`AlignTop`](Self::AlignTop) or [`AlignBottom`](Self::AlignBottom)
/// with [`Relation::LeftOf`] and [`Relation::RightOf`], and
/// [`AlignLeft`](Self::AlignLeft) or [`AlignRight`](Self::AlignRight) with
/// [`Relation::TopOf`] and [`Relation::BottomOf`].
///
/// Parses from and displays as `align-bottom`, `align-top`, `align-right`
/// and `align-left`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// Line up the bottom edges.
    AlignBottom,
    /// Line up the top edges.
    AlignTop,
    /// Line up the right edges.
    AlignRight,
    /// Line up the left edges.
    AlignLeft,
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

/// Where to place an output, relative to another one.
///
/// Parses from a string of the form `"<relation> <reference> <alignment>"`,
/// e.g. `"left-of eDP-1 align-bottom"`, and displays the same way.
#[derive(Debug, Clone)]
pub struct Position {
    /// Which side of the reference output to place the target on.
    pub relation: Relation,
    /// The output to position against, matched like [`Heads::find`](crate::Heads::find).
    pub reference: String,
    /// Which edges of the two outputs line up.
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

/// The changes wanted for one output.
///
/// Every property is optional; `None` leaves it as it is.
#[derive(Debug, Default, Clone)]
pub struct Target {
    /// The output to change, matched like [`Heads::find`](crate::Heads::find).
    pub name: String,
    /// Where to place the output. Applied by [`position`](crate::position).
    pub position: Option<Position>,
    /// The scale factor, e.g. `2.0` for a high-DPI display.
    pub scale: Option<f64>,
    /// An index into the output's [`modes`](crate::Head::modes), sorted
    /// from largest to smallest (by resolution, then refresh rate).
    pub mode: Option<usize>,
    /// The rotation in degrees. Must be a multiple of 90; positive values
    /// rotate the image clockwise.
    pub rotation: Option<i32>,
    /// Whether the output is enabled (`true`) or disabled (`false`).
    pub active: Option<bool>,
}
impl Target {
    /// Creates a target for the output `name`, with every property unset.
    #[must_use]
    pub fn new(name: String) -> Self {
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
        if let Some(active) = self.active {
            writeln!(f, "\tactive: {active}")?;
        }

        Ok(())
    }
}

/// A set of [`Target`]s to apply together.
///
/// Build one by hand with [`add_target`](Self::add_target), or load it from
/// a TOML file (see [`config_file::parser`](crate::config_file::parser)).
#[derive(Debug, Default)]
pub struct Config {
    targets: Vec<Target>,
}

impl Config {
    /// Adds a target to the config.
    pub fn add_target(&mut self, target: Target) {
        self.targets.push(target);
    }

    /// The targets, in the order they were added.
    #[must_use]
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
