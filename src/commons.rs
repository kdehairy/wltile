use wayland_client::protocol::wl_output::Transform;

/// Own `ToString` trait
///
/// Used to add a `to_string()` method on external types that does not implement
/// the `string::ToString` trait. Since either the trait or the type must be
/// owned by the crate, we can't really implement the `string::ToString` for the
/// type at hand.
pub trait ToString {
    fn to_string(&self) -> String;
}

pub trait TryFrom<T>: Sized {
    fn try_from(other: T) -> Result<Self, String>;
}

impl TryFrom<i32> for Transform {
    /// Maps a requested **image** rotation 
    /// to the equivalent `wl_output` `Transform`. The protocol's rotation 
    /// variants (`_90`/`_180`/`_270`) are **counter-clockwise**.
    fn try_from(other: i32) -> Result<Self, String> {
        let norm = other % 360;
        let norm = if norm <= 0 {
            norm
        } else {
            norm.saturating_sub(360)
        };

        match norm {
            0 => Ok(Transform::Normal),
            -90 => Ok(Transform::_90),
            -180 => Ok(Transform::_180),
            -270 => Ok(Transform::_270),
            _ => Err(String::from("Invalid angle")),
        }
    }
}

#[cfg(test)]
mod tests {
    use wayland_client::protocol::wl_output::Transform;

    /// Disambiguates our `commons::TryFrom` from the prelude's
    /// `core::TryFrom`; the return type drives `Self = Transform`, `T = i32`.
    fn transform(angle: i32) -> Result<Transform, String> {
        super::TryFrom::try_from(angle)
    }

    #[test]
    fn zero_rotation_maps_to_normal() {
        assert_eq!(transform(0), Ok(Transform::Normal));
    }

    #[test]
    fn image_rotation_maps_to_the_counter_clockwise_transform() {
        let cases = [
            (90, Transform::_270),
            (180, Transform::_180),
            (270, Transform::_90),
            (-90, Transform::_90),
            (-180, Transform::_180),
            (-270, Transform::_270),
        ];
        for (angle, expected) in cases {
            assert_eq!(transform(angle), Ok(expected), "angle {angle}");
        }
    }

    #[test]
    fn angles_are_canonicalized_modulo_360() {
        assert_eq!(transform(360), Ok(Transform::Normal));
        assert_eq!(transform(-360), Ok(Transform::Normal));
        assert_eq!(transform(720), Ok(Transform::Normal));
        assert_eq!(transform(450), Ok(Transform::_270)); // 450 ≡ 90
        assert_eq!(transform(-450), Ok(Transform::_90)); // -450 ≡ -90
    }

    #[test]
    fn non_right_angles_are_rejected() {
        for angle in [1, 45, 89, 100, 179, 359, -1, -45, -100] {
            assert_eq!(
                transform(angle),
                Err(String::from("Invalid angle")),
                "angle {angle} should be rejected"
            );
        }
    }
}
