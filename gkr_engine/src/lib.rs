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
mod transcript;

pub use field_engine::*;
use mersenne31::M31Ext3;
pub use mpi_engine::*;
pub use poly_commit::*;
pub use transcript::*;

/// Core trait defining the configuration types for a GKR protocol implementation.
///
/// This trait serves as the main configuration interface for the GKR protocol, specifying the required
/// types for field operations, MPI communication, transcript generation, and polynomial commitment schemes.
///
/// # Associated Types
///
/// * `FieldConfig` - Configuration for field arithmetic operations, implementing `FieldEngine`
/// * `MPIConfig` - Configuration for distributed computing operations, implementing `MPIEngine`
/// * `TranscriptConfig` - Configuration for transcript generation, implementing `Transcript` over the challenge field
/// * `PCSConfig` - Configuration for polynomial commitment scheme, implementing `PCSForExpanderGKR`
///
/// # Usage
///
/// This trait is typically implemented by configuration structs that define the complete
/// setup for running the GKR protocol in a distributed environment.
///
/// # Example
/// ```ignore
/// struct MyGKRConfig;
///
/// impl GKREngine for MyGKRConfig {
///     type FieldConfig = MyFieldConfig;
///     type MPIConfig = MyMPIConfig;
///     type TranscriptConfig = MyTranscriptConfig;
///     type PCSConfig = MyPCSConfig;
/// }
/// ```
pub trait GKREngine {
    /// Configuration for field arithmetic operations
    type FieldConfig: FieldEngine;

    /// Configuration for distributed computing operations
    type MPIConfig: MPIEngine;

    /// Configuration for transcript generation over the challenge field
    type TranscriptConfig: Transcript<<Self::FieldConfig as FieldEngine>::ChallengeField>;

    /// Configuration for polynomial commitment scheme
    type PCSConfig: PCSForExpanderGKR<Self::FieldConfig>;
}


pub struct M31ExtConfigPoseidonRaw;

impl GKREngine for M31ExtConfigPoseidonRaw {
    type FieldConfig = M31ExtConfig;
    type MPIConfig = MPIConfig;
    type TranscriptConfig = BytesHashTranscript<M31Ext3, PoseidonHasher>;
    type PCSConfig = PCSConfig;
}