use thiserror::Error;

/// Everything that can go wrong with quantities.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum QuantityError {
    /// Two readings in point units were added: only a reading and a difference add up.
    #[error(
        "cannot add two readings (`{left}` and `{right}`); subtract them for a difference, or add a difference to a reading"
    )]
    ReadingPlusReading { left: String, right: String },

    /// A reading was subtracted from a difference.
    #[error("cannot subtract a reading (`{right}`) from a difference (`{left}`)")]
    DifferenceMinusReading { left: String, right: String },

    /// A unit operation failed: incompatible dimensions, a conversion out of range, ...
    /// See [`UnitError`](inchworm_units::UnitError) for detail.
    #[error(transparent)]
    Unit(#[from] inchworm_units::UnitError),
}
