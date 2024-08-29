#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GatesID {
    // Constant gates
    Constants,

    // Arithmetic gates
    Add, // addition, also used for subtraction
    Mul, // multiplication, also used for division

    // Binary gates
    Binary,
    NonZero,

    // Comparison gates
    Equal,

    // any other gates that are not covered by the above
    MISC,
}
