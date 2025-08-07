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
