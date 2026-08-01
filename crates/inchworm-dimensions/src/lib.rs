//! # inchworm-dimensions
//!
//! The algebra of physical dimensions and dimensional analysis — no units, no
//! magnitudes. The foundation of the inchworm suite
//! (`inchworm-dimensions` → `inchworm-units` → `inchworm-quantities`).
//!
//! ## Core model
//!
//! - A [`Dimension`] is an immutable value: a reduced multiset of
//!   *(named atom, rational power)* factors plus two cached reductions — the
//!   **base signature** (everything expanded to base dimensions; named
//!   dimensionless kinds vanish) and the **canonical form** (expanded, but
//!   named-dimensionless atoms such as `plane_angle` kept as irreducible
//!   factors).
//! - [`Compatibility`] is three-valued: `Full` (equal canonical forms, e.g.
//!   `length/time` vs `velocity`), `Partial` (equal base signatures, different
//!   canonical forms, e.g., `plane_angle×length` vs `length`), `Incompatible`.
//! - Named dimensions whose signature reduces to dimensionless are **never**
//!   silently collapsed to bare dimensionless: this keeps plane angle, solid
//!   angle, strain, and a pure number mutually distinguishable.
//! - [`DimRegistry`] is an instance-based, mutable namespace. `Dimension`
//!   values hold `Arc`s to their atoms and never need the registry after
//!   creation. Cross-registry mixing is detected and rejected.
#![forbid(unsafe_code)]

mod atom;
mod dimension;
mod error;
mod exp;
mod form;
mod parser;
mod registry;
#[cfg(test)]
mod test_utils;

pub use atom::{AtomId, RegistryId};
pub use dimension::{Compatibility, Dimension};
pub use error::DimensionError;
pub use exp::Exp;
pub use form::{Form, Signature};
pub use registry::DimRegistry;
