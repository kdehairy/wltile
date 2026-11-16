//! The desired output configuration: which outputs to change, and how.
//!
//! A [`Config`] holds one [`Target`] per output. Apply it with
//! [`position`](crate::position) for the [`Position`] of each target, and
//! with [`set_property`](crate::set_property) for everything else.

use std::fmt::Display;

/// The axis a [`Relation`] places along, or an [`Alignment`] lines edges up on.
///
/// A relation and an alignment are only usable together when they act on
/// different axes: the relation fixes one coordinate of the target, the
/// alignment the other.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    /// The axis name, as used in error messages.
    fn name(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

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

impl Relation {
    /// The axis this relation places the target along.
    fn axis(self) -> Axis {
        match self {
            Self::LeftOf | Self::RightOf => Axis::Horizontal,
            Self::TopOf | Self::BottomOf => Axis::Vertical,
        }
    }

    /// The alignments usable with this relation: the ones acting on the axis
    /// the relation leaves free.
    ///
    /// [`Position::new`] rejects every other pairing.
    #[must_use]
    pub fn compatible_alignments(self) -> (Alignment, Alignment) {
        match self {
            Self::LeftOf | Self::RightOf => (Alignment::AlignTop, Alignment::AlignBottom),
            Self::TopOf | Self::BottomOf => (Alignment::AlignLeft, Alignment::AlignRight),
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
/// [`Relation::TopOf`] and [`Relation::BottomOf`] — see
/// [`Relation::compatible_alignments`]. [`Position::new`] rejects every other
/// pairing.
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

impl Alignment {
    /// The axis this alignment lines the two outputs' edges up on.
    fn axis(self) -> Axis {
        match self {
            Self::AlignLeft | Self::AlignRight => Axis::Horizontal,
            Self::AlignTop | Self::AlignBottom => Axis::Vertical,
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
/// The relation fixes one axis and the alignment lines the two outputs up
/// along the other, so they have to act on different axes: `left-of`/`right-of`
/// take `align-top`/`align-bottom`, `top-of`/`bottom-of` take
/// `align-left`/`align-right` ([`Relation::compatible_alignments`]).
///
/// [`new`](Self::new) is the only constructor and rejects every other pairing,
/// so a `Position` that exists is a position that can be applied: nothing
/// downstream needs to check it again.
///
/// Parses from a string of the same shape,
/// e.g. `"left-of eDP-1 align-bottom"`, and displays the same way.
#[derive(Debug, Clone)]
pub struct Position {
    relation: Relation,
    reference: String,
    alignment: Alignment,
}

impl Position {
    /// Creates a position from a relation, the output to position against and
    /// an alignment.
    ///
    /// # Errors
    ///
    /// Returns an error when `relation` and `alignment` act on the same axis:
    /// the alignment would overwrite the coordinate the relation had set and
    /// the two outputs would overlap. The message names the axis and the
    /// alignments that do work, e.g. `` `top-of` and `align-top` both act on
    /// the vertical axis; use `align-left` or `align-right` ``.
    pub fn new(
        relation: Relation,
        reference: String,
        alignment: Alignment,
    ) -> Result<Self, String> {
        if alignment.axis() == relation.axis() {
            let (first, second) = relation.compatible_alignments();
            let axis = relation.axis().name();
            return Err(format!(
                "`{relation}` and `{alignment}` both act on the {axis} axis; use `{first}` or `{second}`"
            ));
        }

        Ok(Self {
            relation,
            reference,
            alignment,
        })
    }

    /// Which side of the reference output the target is placed on.
    #[must_use]
    pub fn relation(&self) -> Relation {
        self.relation
    }

    /// The output this one is positioned against, matched like
    /// [`Heads::find`](crate::Heads::find).
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Which edges of the two outputs line up.
    #[must_use]
    pub fn alignment(&self) -> Alignment {
        self.alignment
    }
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

        Position::new(relation, reference, alignment)
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

#[cfg(test)]
mod tests {
    use super::{Alignment, Axis, Position, Relation};

    const RELATIONS: [Relation; 4] = [
        Relation::LeftOf,
        Relation::RightOf,
        Relation::TopOf,
        Relation::BottomOf,
    ];

    const ALIGNMENTS: [Alignment; 4] = [
        Alignment::AlignBottom,
        Alignment::AlignTop,
        Alignment::AlignRight,
        Alignment::AlignLeft,
    ];

    /// Both alignments a relation accepts, as something a loop or `contains`
    /// can work with.
    fn compatible(relation: Relation) -> [Alignment; 2] {
        let (first, second) = relation.compatible_alignments();
        [first, second]
    }

    /// The axis each variant acts on: relations place along one, alignments
    /// line edges up along the other, and `Position::new` requires the two to
    /// differ.
    #[test]
    fn relations_and_alignments_split_across_the_axes() {
        for relation in RELATIONS {
            let expected = match relation {
                Relation::LeftOf | Relation::RightOf => Axis::Horizontal,
                Relation::TopOf | Relation::BottomOf => Axis::Vertical,
            };
            assert_eq!(expected, relation.axis(), "{relation}");
        }
        for alignment in ALIGNMENTS {
            let expected = match alignment {
                Alignment::AlignLeft | Alignment::AlignRight => Axis::Horizontal,
                Alignment::AlignTop | Alignment::AlignBottom => Axis::Vertical,
            };
            assert_eq!(expected, alignment.axis(), "{alignment}");
        }
    }

    /// The matrix, in one place: each relation's compatible alignments are the
    /// ones on the other axis, two per relation, all four reachable.
    #[test]
    fn compatible_alignments_are_the_perpendicular_ones() {
        for relation in RELATIONS {
            let (first, second) = relation.compatible_alignments();
            assert_ne!(first, second, "{relation} has two distinct alignments");
            for alignment in [first, second] {
                assert_ne!(
                    relation.axis(),
                    alignment.axis(),
                    "{relation} + {alignment} are on different axes"
                );
            }
        }

        assert_eq!(
            (Alignment::AlignTop, Alignment::AlignBottom),
            Relation::LeftOf.compatible_alignments()
        );
        assert_eq!(
            (Alignment::AlignTop, Alignment::AlignBottom),
            Relation::RightOf.compatible_alignments()
        );
        assert_eq!(
            (Alignment::AlignLeft, Alignment::AlignRight),
            Relation::TopOf.compatible_alignments()
        );
        assert_eq!(
            (Alignment::AlignLeft, Alignment::AlignRight),
            Relation::BottomOf.compatible_alignments()
        );
    }

    #[test]
    fn new_accepts_the_compatible_pairs() {
        for relation in RELATIONS {
            for alignment in compatible(relation) {
                let position = Position::new(relation, String::from("eDP-1"), alignment)
                    .unwrap_or_else(|err| panic!("{relation} + {alignment} should build: {err}"));
                assert_eq!(relation, position.relation());
                assert_eq!("eDP-1", position.reference());
                assert_eq!(alignment, position.alignment());
            }
        }
    }

    #[test]
    fn new_rejects_pairs_on_the_same_axis() {
        for relation in RELATIONS {
            for alignment in ALIGNMENTS {
                let outcome = Position::new(relation, String::from("eDP-1"), alignment);
                if compatible(relation).contains(&alignment) {
                    assert!(
                        outcome.is_ok(),
                        "{relation} + {alignment} should be accepted"
                    );
                    continue;
                }

                let err =
                    outcome.expect_err(&format!("{relation} + {alignment} should be rejected"));
                assert!(
                    err.contains(&relation.to_string()) && err.contains(&alignment.to_string()),
                    "{relation} + {alignment}: {err} should name the pair"
                );
                assert!(
                    err.contains(relation.axis().name()) && err.contains("axis"),
                    "{relation} + {alignment}: {err} should name the axis"
                );
                for suggestion in compatible(relation) {
                    assert!(
                        err.contains(&suggestion.to_string()),
                        "{relation} + {alignment}: {err} should suggest {suggestion}"
                    );
                }
            }
        }
    }

    /// The two ways of building a `Position` have to agree: the config file
    /// parses strings, the CLI and library callers go through `new`.
    #[test]
    fn string_and_constructor_agree() {
        for relation in RELATIONS {
            for alignment in ALIGNMENTS {
                let from_string = Position::try_from(format!("{relation} eDP-1 {alignment}"));
                let from_new = Position::new(relation, String::from("eDP-1"), alignment);
                match (from_string, from_new) {
                    (Ok(parsed), Ok(built)) => {
                        assert_eq!(built.relation(), parsed.relation());
                        assert_eq!(built.reference(), parsed.reference());
                        assert_eq!(built.alignment(), parsed.alignment());
                    }
                    (Err(parsed_err), Err(built_err)) => {
                        assert_eq!(built_err, parsed_err, "{relation} + {alignment}");
                    }
                    (parsed, built) => panic!(
                        "{relation} + {alignment} disagreed: parse {:?}, new {:?}",
                        parsed.map(|p| p.to_string()),
                        built.map(|p| p.to_string())
                    ),
                }
            }
        }
    }

    #[test]
    fn parses_and_displays_the_three_tokens() {
        let text = "right-of eDP-1 align-top";
        let position = Position::try_from(String::from(text)).expect("valid position");

        assert_eq!(Relation::RightOf, position.relation());
        assert_eq!("eDP-1", position.reference());
        assert_eq!(Alignment::AlignTop, position.alignment());
        assert_eq!(text, position.to_string());
    }

    #[test]
    fn reports_which_token_is_missing_or_unknown() {
        for (value, expected) in [
            ("", "Missing relation"),
            ("left-of", "Missing reference"),
            ("left-of eDP-1", "Missing alignment"),
            ("bogus eDP-1 align-top", "Unknown relation 'bogus'"),
            ("left-of eDP-1 bogus", "Unknown alignment 'bogus'"),
        ] {
            let err =
                Position::try_from(String::from(value)).expect_err(&format!("{value:?} invalid"));
            assert_eq!(expected, err, "{value:?}");
        }
    }
}
