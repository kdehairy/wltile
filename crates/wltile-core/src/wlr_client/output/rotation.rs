use std::fmt::Display;

use wayland_client::protocol::wl_output::Transform;

/// How an output's image is oriented.
///
/// The rotation variants are **counter-clockwise**, matching the protocol.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Rotation {
    Normal,
    R90,
    R180,
    R270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
    /// A transform the compositor reported that this version does not know
    /// about. `wl_output`'s `Transform` is non-exhaustive.
    Unknown,
}

impl From<Transform> for Rotation {
    fn from(value: Transform) -> Self {
        match value {
            Transform::Normal => Rotation::Normal,
            Transform::_90 => Rotation::R90,
            Transform::_180 => Rotation::R180,
            Transform::_270 => Rotation::R270,
            Transform::Flipped => Rotation::Flipped,
            Transform::Flipped90 => Rotation::Flipped90,
            Transform::Flipped180 => Rotation::Flipped180,
            Transform::Flipped270 => Rotation::Flipped270,
            _ => Rotation::Unknown,
        }
    }
}

impl Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rotation::Normal => write!(f, "0°"),
            Rotation::R90 => write!(f, "-90°"),
            Rotation::R180 => write!(f, "-180°"),
            Rotation::R270 => write!(f, "-270°"),
            Rotation::Flipped => write!(f, "Flipped"),
            Rotation::Flipped90 => write!(f, "flipped then rotated 90°"),
            Rotation::Flipped180 => write!(f, "flipped then rotated 180°"),
            Rotation::Flipped270 => write!(f, "flipped then rotated 270°"),
            Rotation::Unknown => write!(f, "?"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Rotation;
    use wayland_client::protocol::wl_output::Transform;

    #[test]
    fn renders_the_same_text_as_the_wayland_transform() {
        let cases = [
            (Transform::Normal, "0°"),
            (Transform::_90, "-90°"),
            (Transform::_180, "-180°"),
            (Transform::_270, "-270°"),
            (Transform::Flipped, "Flipped"),
            (Transform::Flipped90, "flipped then rotated 90°"),
            (Transform::Flipped180, "flipped then rotated 180°"),
            (Transform::Flipped270, "flipped then rotated 270°"),
        ];
        for (transform, expected) in cases {
            assert_eq!(Rotation::from(transform).to_string(), expected);
        }
    }

    #[test]
    fn unknown_transforms_render_as_a_question_mark() {
        assert_eq!(Rotation::Unknown.to_string(), "?");
    }
}
