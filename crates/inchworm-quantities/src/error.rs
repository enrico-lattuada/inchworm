use thiserror::Error;

/// Everything that can go wrong with quantities.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum QuantityError {
    /// A unit operation failed: incompatible dimensions, a conversion out of range, ...
    /// See [`UnitError`](inchworm_units::UnitError) for detail.
    #[error(transparent)]
    Unit(#[from] inchworm_units::UnitError),
}
