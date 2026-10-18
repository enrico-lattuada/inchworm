//! # inchworm-units
//!
//! Named units built on inchworm-dimensions, with no magnitudes (see
//! inchworm-quantities). A [`UnitRegistry`] holds three kinds of unit:
//! - **base** units, one per irreducible dimension, possibly prefixed (`kg`, `cm`);
//! - **derived** delta units, `scale × definition` (`km`, `J`, `delta_degF`);
//! - **point** (affine) units, a delta unit plus an offset (`degC`, `degF`).
//!
//! Delta units compose into [`DeltaUnit`] values; [`Unit`] wraps either kind.
//! Values never need their registry after creation: [`DeltaUnit::to_base`]
//! expands any unit to base units on its own.
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
