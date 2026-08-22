//! # inchworm-units
//!
//! Named units built on inchworm-dimensions: unit atoms (meter, degree,
//! celsius) and Unit values (reduced multisets of atom/rational-power
//! factors with cached dimension and scale-to-coherent-base). No
//! magnitudes (see inchworm-quantities).
#![forbid(unsafe_code)]

mod atom;
mod error;
mod registry;
#[cfg(test)]
mod test_utils;
mod unit;

pub use atom::{UnitId, UnitRegistryId};
pub use error::UnitError;
pub use registry::UnitRegistry;
pub use unit::Unit;
