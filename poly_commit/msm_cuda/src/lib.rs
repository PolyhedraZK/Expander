// Copyright Supranational LLC
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

pub mod util;

#[cfg(feature = "cuda")]
mod arkworks_impl;
#[cfg(feature = "cuda")]
pub use arkworks_impl::*;

#[cfg(feature = "cuda")]
mod halo2_wrapper;
#[cfg(feature = "cuda")]
pub use halo2_wrapper::*;

#[cfg(not(feature = "cuda"))]
mod dummy_impl;
#[cfg(not(feature = "cuda"))]
pub use dummy_impl::*;
