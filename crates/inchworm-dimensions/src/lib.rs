//! A library for defining and working with physical dimensions in the Inchworm framework.
//!
//! This crate provides structures and utilities for defining base and derived physical dimensions,
//! as well as a registry for managing them. It is designed to be used in conjunction with the
//! Inchworm units system to enable dimensional analysis and unit conversions.

mod base_dimension_def;
mod derived_dimension_def;
mod dimension_component;
mod dimension_def;
mod errors;
mod registry;

pub use base_dimension_def::BaseDimensionDef;
pub use derived_dimension_def::DerivedDimensionDef;
pub use dimension_component::DimensionComponent;
pub use dimension_def::DimensionDef;
pub use errors::DimensionError;
pub use registry::DimensionRegistry;
