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
#![allow(clippy::manual_div_ceil)]

#![feature(associated_type_defaults)]

mod errors;
mod field_engine;
mod mpi_engine;
mod poly_commit;
mod scheme;
mod transcript;

use arith::Field;
pub use errors::*;
pub use field_engine::*;
pub use mpi_engine::*;
pub use poly_commit::*;
pub use scheme::*;
pub use transcript::*;

/// Core trait defining the configuration types for a GKR protocol implementation.
///
/// This trait serves as the main configuration interface for the GKR protocol, specifying the
/// required types for field operations, MPI communication, transcript generation, and polynomial
/// commitment schemes.
///
/// # Associated Types
///
/// * `FieldConfig` - Configuration for field arithmetic operations, implementing `FieldEngine`
/// * `MPIConfig` - Configuration for distributed computing operations, implementing `MPIEngine`
/// * `TranscriptConfig` - Configuration for transcript generation, implementing `Transcript` over
///   the challenge field
/// * `PCSConfig` - Configuration for polynomial commitment scheme, implementing `PCSForExpanderGKR`
/// * `Scheme` - Identifier for the GKR scheme, candidates are `GKRScheme::Vanilla` and
///   `GKRScheme::GkrSquare`
///
/// # Usage
///
/// This trait is typically implemented by configuration structs that define the complete
/// setup for running the GKR protocol in a distributed environment.
///
/// # Example
/// ```ignore
/// struct M31Ext3Sha2Raw;
///
/// impl GKREngine for M31Ext3Sha2Raw {
///     type FieldConfig = M31Ext3Config;
///     type MPIConfig = MPIConfig;
///     type TranscriptConfig = BytesHashTranscript<M31Ext3, Sha2hasher>;
///     type PCSConfig = RawPCS<M31Ext3>;
///     const SCHEME: GKRScheme = GKRScheme::Vanilla;
/// }
/// ```
pub trait GKREngine: Send + Sync {
    /// Configuration for field arithmetic operations
    type FieldConfig: FieldEngine;

    /// Configuration for distributed computing operations
    type MPIConfig: MPIEngine;

    /// Configuration for transcript generation over the challenge field
    type TranscriptConfig: Transcript;

    /// Configuration for polynomial commitment scheme
    type PCSField: Field = <<Self as GKREngine>::FieldConfig as FieldEngine>::SimdCircuitField;
    type PCSConfig: ExpanderPCS<Self::FieldConfig, Self::PCSField>;

    /// GKR scheme
    const SCHEME: GKRScheme;
}
