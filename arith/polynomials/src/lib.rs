mod bivariate;
pub use bivariate::*;

mod univariate;
pub use univariate::*;

mod mle;
pub use mle::*;

mod eq;
pub use eq::*;

mod utils;
pub use utils::*;

#[cfg(test)]
mod tests;
