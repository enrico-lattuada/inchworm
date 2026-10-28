//! [`Converter`]: a reusable conversion between two compatible units.

use inchworm_dimensions::Compatibility;
use std::fmt;

use crate::{
    DeltaUnit, PointUnit, UnitError, UnitRegistryId, atom_ids::check_same_registry, scale::Scale,
};

/// A reusable conversion between two compatible units: a reading `x` in the
/// source unit is `apply(x) = x · factor + offset` in the target unit.
///
/// Built by [`Unit::converter_to`](crate::Unit::converter_to) or
/// [`DeltaUnit::converter_to`](crate::DeltaUnit::converter_to), and validated once,
/// so applying it to many values is a multiply-add each. The offset is non-zero
/// only when a point unit (`degC`, `degF`) is involved; see [`is_linear`](Self::is_linear).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Converter {
    factor: f64,
    offset: f64,
}

impl Converter {
    /// Builds a converter from a validated `factor` and an `offset` in target units.
    ///
    /// `None` if `offset` is not finite.
    pub(crate) fn new(factor: Scale, offset: f64) -> Option<Self> {
        offset.is_finite().then(|| Self {
            factor: factor.value(),
            offset,
        })
    }

    /// Converts `x` from the source unit to the target unit: `x · factor + offset`.
    pub fn apply(self, x: f64) -> f64 {
        x * self.factor + self.offset
    }

    /// `true` iff the offset is exactly zero, so `apply(x) == x · factor`.
    ///
    /// A linear conversion can be applied in place, to whole arrays, or composed
    /// multiplicatively with other linear conversions. Check it once, outside a
    /// loop, rather than per value.
    pub fn is_linear(self) -> bool {
        self.offset == 0.0
    }
}

/// One side of a conversion: a delta unit and the origin of its readings.
///
/// A [`DeltaUnit`] is itself with origin `0`; a [`PointUnit`] is its delta
/// with origin `offset`. Borrowed, so building a converter clones nothing.
pub(crate) struct Endpoint<'a> {
    /// The scale and dimension of the readings.
    delta: &'a DeltaUnit,
    /// Where `0` of this unit sits, in `delta`'s units.
    origin: f64,
    /// The registry of the unit itself (a point's own atom, not only its delta's).
    registry_id: Option<UnitRegistryId>,
    /// How this side prints in errors: the point itself, not its delta.
    unit: &'a dyn fmt::Display,
}

impl<'a> Endpoint<'a> {
    /// Builds an [`Endpoint`] from a [`DeltaUnit`].
    pub(crate) fn of_delta(delta: &'a DeltaUnit) -> Self {
        Endpoint {
            delta,
            origin: 0.0,
            registry_id: delta.registry_id(),
            unit: delta,
        }
    }

    /// Builds an [`Endpoint`] from a [`PointUnit`].
    pub(crate) fn of_point(point: &'a PointUnit) -> Self {
        Endpoint {
            delta: point.delta(),
            origin: point.offset(),
            registry_id: Some(point.registry_id()),
            unit: point,
        }
    }
}

/// Builds the converter from `from` to `to`:
/// `factor = s_from / s_to`, `offset = o_from · factor − o_to`.
///
/// # Errors
///
/// Returns [`UnitError::CrossRegistry`] if the two units come from different registries.
/// Returns [`UnitError::IncompatibleDimensions`] if their base signatures differ.
/// Returns [`UnitError::PartiallyCompatibleDimensions`] if they share a base signature
/// but are different kinds.
/// Returns [`UnitError::ScaleOutOfRange`] if either unit's scale relative to the base
/// units overflows or underflows.
/// Returns [`UnitError::ConversionOutOfRange`] if the factor or the offset is not representable.
pub(crate) fn converter_between(
    from: Endpoint<'_>,
    to: Endpoint<'_>,
) -> Result<Converter, UnitError> {
    check_same_registry(from.registry_id, to.registry_id)?;
    let (left, right) = (from.delta.dimension(), to.delta.dimension());
    match left.compatibility(right) {
        Compatibility::Full => {}
        Compatibility::Partial => {
            return Err(UnitError::PartiallyCompatibleDimensions {
                left: left.clone(),
                right: right.clone(),
            });
        }
        Compatibility::Incompatible => {
            return Err(UnitError::IncompatibleDimensions {
                left: left.clone(),
                right: right.clone(),
            });
        }
    }
    let s1 = from.delta.scale()?;
    let s2 = to.delta.scale()?;
    let out_of_range = || UnitError::ConversionOutOfRange {
        from: from.unit.to_string(),
        to: to.unit.to_string(),
    };
    let factor = s1.checked_div(s2).ok_or_else(out_of_range)?;
    let offset = from.origin * factor.value() - to.origin;
    Converter::new(factor, offset).ok_or_else(out_of_range)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn accepts_finite_offset() {
            let converter = Converter::new(Scale::new(2.0).unwrap(), 1.0).unwrap();
            assert_eq!(converter.factor, 2.0);
            assert_eq!(converter.offset, 1.0);
        }

        #[test]
        fn rejects_infinite_offset() {
            assert!(Converter::new(Scale::new(1.0).unwrap(), f64::INFINITY).is_none());
        }

        #[test]
        fn rejects_nan_offset() {
            assert!(Converter::new(Scale::new(1.0).unwrap(), f64::NAN).is_none());
        }
    }

    mod apply {
        use super::*;

        #[test]
        fn scales_without_offset() {
            let converter = Converter {
                factor: 2.0,
                offset: 0.0,
            };
            assert_eq!(converter.apply(3.0), 6.0);
        }

        #[test]
        fn applies_factor_then_offset() {
            let converter = Converter {
                factor: 1.0,
                offset: 273.15,
            };
            assert_eq!(converter.apply(0.0), 273.15);
        }
    }

    mod is_linear {
        use super::*;

        #[test]
        fn true_for_zero_offset() {
            let converter = Converter {
                factor: 1.0,
                offset: 0.0,
            };
            assert!(converter.is_linear())
        }

        #[test]
        fn false_for_nonzero_offset() {
            let converter = Converter {
                factor: 1.0,
                offset: 1.0,
            };
            assert!(!converter.is_linear())
        }

        #[test]
        fn true_for_negative_zero_offset() {
            let converter = Converter {
                factor: 1.0,
                offset: -0.0,
            };
            assert!(converter.is_linear())
        }
    }
}
