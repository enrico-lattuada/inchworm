//! # inchworm-quantities
//!
//! Quantities: a magnitude in a unit (`5 km`, `20 °C`), converted between units of the
//! same kind. The magnitude can be any type implementing [`Magnitude`].
//!
//! Re-exports `inchworm-dimensions` as [`dimensions`] and `inchworm-units` as [`units`].
#![forbid(unsafe_code)]

mod error;
mod magnitude;
mod ops;
mod quantity;
#[cfg(test)]
mod test_utils;

pub use error::QuantityError;
pub use magnitude::Magnitude;
pub use quantity::Quantity;

pub use inchworm_dimensions as dimensions;
pub use inchworm_units as units;
