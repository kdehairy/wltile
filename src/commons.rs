pub trait ToString {
    fn to_string(&self) -> String;
}

pub trait TryFrom<T> : Sized {
    fn try_from(other: T) -> Result<Self, String>;
}
