/// Own `ToString` trait
///
/// Used to add a `to_string()` method on external types that does not implement
/// the `string::ToString` trait. Since either the trait or the type must be 
/// owned by the crate, we can't really implement the `string::ToString` for the 
/// type at hand.
pub trait ToString {
    fn to_string(&self) -> String;
}

pub trait TryFrom<T> : Sized {
    fn try_from(other: T) -> Result<Self, String>;
}
