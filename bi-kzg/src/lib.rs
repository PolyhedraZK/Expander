mod coeff_form_bi_kzg;
mod structs;
mod util;

// mod lagrange_form_bi_kzg;

#[cfg(test)]
mod tests;

pub use coeff_form_bi_kzg::CoeffFormBiKZG;
pub use structs::{BiKZGCommitment, BiKZGProof, BiKZGSRS, BiKZGVerifierParam};

// pub use lagrange_form_bi_kzg::LagrangeFormBiKZG;
