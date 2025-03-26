//! This crate defines the generic APIs that are used by GKR sub-kernals
//!
//! - Config trait: a trait that defines the following components of a GKR protocol
//! - MPI Engine: a simple MPI engine that is used to communicate between processes
//! - Field Engine: a simple field engine that is used to perform field operations and its
//!   extensions
//! - Transcript trait: a trait that defines the API for transcript
//! - PCS trait: a trait that defines the API for polynomial commitment schemes
//!
//! MISC: some naming patterns:
//! - A Engine is a trait that defines the API for a component of GKR
//! - A Config is a struct that implements the Engine trait and contains the parameters for the GKR
//!   protocol

mod field_engine;
mod mpi_engine;
mod poly_commit;

pub use field_engine::*;
pub use mpi_engine::*;
pub use poly_commit::*;

pub trait GKREngine {
    type FieldEngine: FieldEngine;
    // type MPIEngine: MPIEngine;
    // type Transcript: Transcript<Self::FieldEngine::ChallengeField>;
    type PCS: PCSForExpanderGKR<Self::FieldEngine>;
}
