//! # inchworm-units
//!
//! Named units built on inchworm-dimensions: unit atoms (meter, degree,
//! celsius) and Unit values (reduced multisets of atom/rational-power
//! factors with cached dimension and scale-to-coherent-base). No
//! magnitudes (see inchworm-quantities).
#![forbid(unsafe_code)]

mod atom;

pub use atom::{UnitId, UnitRegistryId};
