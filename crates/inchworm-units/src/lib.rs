//! # inchworm-units
//!
//! Named units built on inchworm-dimensions: unit atoms (meter, degree,
//! celsius) and Unit values (reduced multisets of atom/rational-power
//! factors with cached dimension). No magnitudes (see inchworm-quantities).
#![forbid(unsafe_code)]

mod atom_ids;
mod error;
mod prefix;
#[cfg(test)]
mod test_utils;

pub use atom_ids::{UnitId, UnitRegistryId};
pub use error::UnitError;
