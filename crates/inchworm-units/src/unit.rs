use inchworm_dimensions::{Dimension, Exp};
use std::fmt;

use crate::{
    Converter, DeltaUnit, PointUnit, UnitError, UnitRegistryId,
    converter::{Endpoint, converter_between},
};

/// A unit as it appears in an expression or on a quantity.
///
/// Either a composable delta unit or a point unit, which rejects all algebra.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Unit {
    /// A relative interval, difference, or displacement between two points (e.g., m, km/h).
    Delta(DeltaUnit),
    /// A specific point or location on a scale measured from an arbitrary zero-point (e.g., 25°C).
    Point(PointUnit),
}

impl From<DeltaUnit> for Unit {
    fn from(value: DeltaUnit) -> Self {
        Self::Delta(value)
    }
}

impl From<PointUnit> for Unit {
    fn from(value: PointUnit) -> Self {
        Self::Point(value)
    }
}

// ---- accessors ----
impl Unit {
    /// The unit's dimension.
    pub fn dimension(&self) -> &Dimension {
        match self {
            Self::Delta(delta) => delta.dimension(),
            Self::Point(point) => point.dimension(),
        }
    }

    /// The unit's registry id.
    pub fn registry_id(&self) -> Option<UnitRegistryId> {
        match self {
            Self::Delta(delta) => delta.registry_id(),
            Self::Point(point) => Some(point.registry_id()),
        }
    }
}

fn not_composable(point: &PointUnit) -> UnitError {
    UnitError::NotComposable {
        name: point.name().into(),
    }
}

// ---- algebra ----
impl Unit {
    /// Multiplies `self` by `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining exponents overflows.
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were built by different unit registries.
    /// Returns [`UnitError::NotComposable`] if `self` or `rhs` are [`Self::Point`] units.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, UnitError> {
        match (self, rhs) {
            (Self::Delta(lhs), Self::Delta(rhs)) => lhs.try_mul(rhs).map(Self::Delta),
            (Self::Point(point), _) | (_, Self::Point(point)) => Err(not_composable(point)),
        }
    }

    /// Returns the reciprocal of `self`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if computing the reciprocal of an atom's exponents overflows.
    /// Returns [`UnitError::NotComposable`] if `self` is [`Self::Point`] unit.
    pub fn recip(&self) -> Result<Self, UnitError> {
        match self {
            Self::Delta(delta) => delta.recip().map(Self::Delta),
            Self::Point(point) => Err(not_composable(point)),
        }
    }

    /// Divides `self` by `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining exponents overflows.
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were built by different unit registries.
    /// Returns [`UnitError::NotComposable`] if `self` or `rhs` are [`Self::Point`] unit.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, UnitError> {
        match (self, rhs) {
            (Self::Delta(lhs), Self::Delta(rhs)) => lhs.try_div(rhs).map(Self::Delta),
            (Self::Point(point), _) | (_, Self::Point(point)) => Err(not_composable(point)),
        }
    }

    /// Raises `self` to the power of `e`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if multiplying an atom's exponent by `e` overflows.
    /// Returns [`UnitError::NotComposable`] if `self` is [`Self::Point`] unit.
    pub fn pow(&self, e: Exp) -> Result<Self, UnitError> {
        match self {
            Self::Delta(delta) => delta.pow(e).map(Self::Delta),
            Self::Point(point) => Err(not_composable(point)),
        }
    }
}

// ---- conversion ----
impl Unit {
    fn endpoint(&self) -> Endpoint<'_> {
        match self {
            Unit::Delta(delta) => Endpoint::of_delta(delta),
            Unit::Point(point) => Endpoint::of_point(point),
        }
    }

    /// Builds a reusable [`Converter`] from readings in `self` to readings in `target`.
    ///
    /// Delta units convert by scale only (`km/h → m/s`). Point units also shift the
    /// origin (`degC → degF`), and a delta unit is treated as an absolute reading on
    /// its own scale: `K → degC` maps `0` to `−273.15`. This also means that
    /// `delta_degC → degC` maps `10` to `−263.15`: whether a value is a reading or a
    /// difference is not visible to units, and is tracked by the quantity layer.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::CrossRegistry`] if `self` and `target` come from different registries.
    /// Returns [`UnitError::IncompatibleDimensions`] if their base signatures differ
    /// (`m → s`).
    /// Returns [`UnitError::PartiallyCompatibleDimensions`] if they share a base signature
    /// but are different kinds (`N·m` as torque → `J`).
    /// Returns [`UnitError::ScaleOutOfRange`] if either unit's scale relative to the base
    /// units overflows or underflows.
    /// Returns [`UnitError::ConversionOutOfRange`] if the conversion factor or offset is
    /// not representable.
    pub fn converter_to(&self, target: &Self) -> Result<Converter, UnitError> {
        converter_between(self.endpoint(), target.endpoint())
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Delta(delta) => delta.fmt(f),
            Self::Point(point) => point.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{errors_match, make_atom, make_point_atom};
    use inchworm_dimensions::DimRegistry;

    struct Units {
        kelvin: DeltaUnit,
        celsius: PointUnit,
        second: DeltaUnit,
    }

    fn units() -> Units {
        let mut dims = DimRegistry::new("test-reg");
        let temperature = dims.add_base("temperature", None).unwrap();
        let time = dims.add_base("time", None).unwrap();
        let ureg_id = UnitRegistryId::next();
        let kelvin_atom = make_atom(ureg_id, "K", temperature);
        let kelvin = DeltaUnit::from_atom(&kelvin_atom);
        let celsius_atom = make_point_atom(ureg_id, "degC", &kelvin, 273.15);
        let celsius = PointUnit::from_atom(&celsius_atom);
        let second_atom = make_atom(ureg_id, "s", time);
        let second = DeltaUnit::from_atom(&second_atom);
        Units {
            kelvin,
            celsius,
            second,
        }
    }

    mod dimension {
        use super::*;

        #[test]
        fn point_dimension_is_delta_dimension() {
            let Units {
                kelvin, celsius, ..
            } = units();
            assert_eq!(celsius.dimension(), kelvin.dimension());
        }
    }

    mod try_mul {
        use super::*;

        #[test]
        fn delta_delegates_to_delta_unit() {
            let Units { kelvin, second, .. } = units();
            assert_eq!(
                Unit::Delta(kelvin.try_mul(&second).unwrap()),
                Unit::Delta(kelvin).try_mul(&Unit::Delta(second)).unwrap()
            );
        }

        #[test]
        fn point_on_either_side_is_not_composable() {
            let Units {
                kelvin, celsius, ..
            } = units();
            let kelvin = Unit::Delta(kelvin);
            let celsius = Unit::Point(celsius);
            let expected = UnitError::NotComposable {
                name: "degC".into(),
            };
            for (lhs, rhs) in [(&kelvin, &celsius), (&celsius, &kelvin)] {
                let actual = lhs.try_mul(rhs).unwrap_err();
                assert!(errors_match(&actual, &expected));
            }
        }

        #[test]
        fn two_points_name_the_left_operand() {
            let Units {
                kelvin, celsius, ..
            } = units();
            let celsius_atom = make_point_atom(celsius.registry_id(), "celsius_2", &kelvin, 273.15);
            let celsius_2 = PointUnit::from_atom(&celsius_atom);
            for (lhs, rhs) in [(&celsius, &celsius_2), (&celsius_2, &celsius)] {
                let actual = Unit::Point(lhs.clone())
                    .try_mul(&Unit::Point(rhs.clone()))
                    .unwrap_err();
                let expected = UnitError::NotComposable {
                    name: lhs.name().into(),
                };
                assert!(errors_match(&actual, &expected));
            }
        }
    }

    mod recip {
        use super::*;

        #[test]
        fn delta_delegates_to_delta_unit() {
            let Units { kelvin, .. } = units();
            assert_eq!(
                Unit::Delta(kelvin.recip().unwrap()),
                Unit::Delta(kelvin).recip().unwrap()
            );
        }

        #[test]
        fn point_is_not_composable() {
            let Units { celsius, .. } = units();
            let actual = Unit::Point(celsius.clone()).recip().unwrap_err();
            let expected = UnitError::NotComposable {
                name: celsius.name().into(),
            };
            assert!(errors_match(&actual, &expected));
        }
    }

    mod try_div {
        use super::*;

        #[test]
        fn delta_delegates_to_delta_unit() {
            let Units { kelvin, second, .. } = units();
            assert_eq!(
                Unit::Delta(kelvin.try_div(&second).unwrap()),
                Unit::Delta(kelvin).try_div(&Unit::Delta(second)).unwrap()
            );
        }

        #[test]
        fn point_dividend_or_divisor_is_not_composable() {
            let Units {
                celsius, second, ..
            } = units();
            let celsius_unit = Unit::Point(celsius.clone());
            let second_unit = Unit::Delta(second);
            let expected = UnitError::NotComposable {
                name: celsius.name().into(),
            };
            for (lhs, rhs) in [(&celsius_unit, &second_unit), (&second_unit, &celsius_unit)] {
                let actual = lhs.try_div(rhs).unwrap_err();
                assert!(errors_match(&actual, &expected));
            }
        }
    }

    mod pow {
        use super::*;

        #[test]
        fn delta_delegates_to_delta_unit() {
            let Units { kelvin, .. } = units();
            assert_eq!(
                Unit::Delta(kelvin.pow(Exp::int(2)).unwrap()),
                Unit::Delta(kelvin).pow(Exp::int(2)).unwrap()
            );
        }

        #[test]
        fn point_is_not_composable_even_at_one_or_zero() {
            let Units { celsius, .. } = units();
            let celsius_unit = Unit::Point(celsius.clone());
            let expected = UnitError::NotComposable {
                name: celsius.name().into(),
            };
            for exp in [Exp::ONE, Exp::ZERO] {
                let actual = celsius_unit.pow(exp).unwrap_err();
                assert!(errors_match(&actual, &expected))
            }
        }
    }

    mod display {
        use super::*;

        #[test]
        fn delegates_to_variant() {
            let Units {
                kelvin, celsius, ..
            } = units();
            let kelvin_unit = Unit::Delta(kelvin.clone());
            assert_eq!(format!("{kelvin_unit}"), format!("{kelvin}"));
            let celsius_unit = Unit::Point(celsius.clone());
            assert_eq!(format!("{celsius_unit}"), format!("{celsius}"));
        }
    }
}
