use std::fmt::Display;

#[derive(Debug, Default, PartialEq, Clone, Copy, Eq, Hash)]
pub struct Point(pub i32, pub i32);

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let me = self.0.pow(2).saturating_add(self.1.pow(2));
        let other = other.0.pow(2).saturating_add(other.1.pow(2));
        me.cmp(&other)
    }
}
