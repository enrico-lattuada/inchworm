//! # inchworm-units
//!
//! Named units built on inchworm-dimensions: unit atoms (meter, degree,
//! celsius) and Unit values (reduced multisets of atom/rational-power
//! factors with cached dimension). No magnitudes (see inchworm-quantities).
#![forbid(unsafe_code)]

mod atom;
mod atom_ids;
mod delta;
mod error;
mod form;
mod parse;
mod point;
mod prefix;
mod registry;
mod scale;
#[cfg(test)]
mod test_utils;
mod unit;

pub use atom_ids::{UnitId, UnitRegistryId};
pub use delta::DeltaUnit;
pub use error::UnitError;
pub use form::UnitForm;
pub use point::PointUnit;
pub use registry::UnitRegistry;
pub use scale::Scale;
pub use unit::Unit;
