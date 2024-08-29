#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GatesID {
    Constants,

    Add,
    Mul,

    Binary,
    NonZero,

    Equal,

    MISC,
}
