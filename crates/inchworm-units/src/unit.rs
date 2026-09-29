use std::fmt;

use inchworm_dimensions::{Dimension, Exp};

use crate::{DeltaUnit, PointUnit, UnitError};

/// A unit as it appears in a unit expression or on a quantity: either a
/// composable [`DeltaUnit`] (`m`, `km/h`, `delta_degC`) or a [`PointUnit`]
/// (`degC`, `degF`), which is only ever valid on its own.
#[expect(clippy::large_enum_variant, reason = "delta is the common variant")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Unit {
    /// A linear unit: a reduced product of atom powers, freely composable
    /// (`m`, `kg*m/s^2`, `delta_degC/min`). A value in it is an amount or a
    /// difference, and converts by a scale factor alone.
    Delta(DeltaUnit),
    /// A point on an affine axis (`degC`, `degF`). A value in it is a
    /// reading (a position, not a difference), so it cannot be multiplied,
    /// divided or raised to any power. It converts by a scale and an origin
    /// shift, through its [`delta`](PointUnit::delta) unit.
    Point(PointUnit),
}

impl From<DeltaUnit> for Unit {
    fn from(value: DeltaUnit) -> Self {
        Self::Delta(value)
    }
}

impl Unit {
    /// Returns the [`Dimension`] of the unit or, for a point unit, its delta's
    /// dimension.
    pub fn dimension(&self) -> &Dimension {
        match self {
            Self::Delta(delta) => delta.dimension(),
            Self::Point(point) => point.delta().dimension(),
        }
    }
}

impl Unit {
    /// Multiplies `self` by `rhs`.
    ///
    /// # Errors
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were minted by
    /// different unit registries.
    /// Returns [`UnitError::NotComposable`] if either side is a point unit.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining a shared atom's exponents overflows, or
    /// [`DimensionError::CrossRegistry`](inchworm_dimensions::DimensionError::CrossRegistry)
    /// if `self` and `rhs`'s dimensions come from different dimension registries.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, UnitError> {
        match (self, rhs) {
            (Self::Delta(lhs), Self::Delta(rhs)) => lhs.try_mul(rhs).map(Self::Delta),
            (Self::Point(point), _) | (_, Self::Point(point)) => Err(UnitError::NotComposable {
                name: point.name().into(),
            }),
        }
    }

    /// Divides `self` by `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were minted by
    /// different unit registries.
    /// Returns [`UnitError::NotComposable`] if either side is a point unit.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining a shared atom's exponents overflows.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::CrossRegistry`](inchworm_dimensions::DimensionError::CrossRegistry)
    /// if `self` and `rhs`'s dimensions come from different dimension registries.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, UnitError> {
        match (self, rhs) {
            (Self::Delta(lhs), Self::Delta(rhs)) => lhs.try_div(rhs).map(Self::Delta),
            (Self::Point(point), _) | (_, Self::Point(point)) => Err(UnitError::NotComposable {
                name: point.name().into(),
            }),
        }
    }

    /// Raises `self` to the power of `e`, pruning any that cancels to zero.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::NotComposable`] if `self` is a point unit.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if multiplying an atom's exponent by `e` overflows.
    pub fn pow(&self, e: Exp) -> Result<Self, UnitError> {
        match self {
            Self::Delta(delta) => delta.pow(e).map(Self::Delta),
            Self::Point(point) => Err(UnitError::NotComposable {
                name: point.name().into(),
            }),
        }
    }

    /// Computes the reciprocal of `self` by raising it to the power of `-1`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::NotComposable`] if `self` is a point unit.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if computing the reciprocal of an atom's exponents overflows.
    pub fn recip(&self) -> Result<Self, UnitError> {
        match self {
            Self::Delta(delta) => delta.recip().map(Self::Delta),
            Self::Point(point) => Err(UnitError::NotComposable {
                name: point.name().into(),
            }),
        }
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
    use crate::UnitRegistryId;
    use crate::atom::ConversionKind;
    use crate::test_utils::{errors_match, make_point_unit, make_unit_atom};
    use inchworm_dimensions::DimRegistry;

    mod unit_enum {
        use super::*;
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn make_kelvin_delta_unit() -> DeltaUnit {
            let registry_id = UnitRegistryId::next();
            let mut dims = DimRegistry::new("test-dim-reg");
            let temperature = dims.add_base("temperature", None).unwrap();
            let kelvin_atom = make_unit_atom(
                registry_id,
                "kelvin",
                temperature,
                ConversionKind::Linear { scale: 1.0 },
            );
            DeltaUnit::single(&kelvin_atom, Exp::ONE).unwrap()
        }

        #[test]
        fn point_times_delta_is_not_composable() {
            let kelvin = make_kelvin_delta_unit();
            let celsius = Unit::Point(make_point_unit("celsius", kelvin.clone(), 273.15));
            let err = celsius.try_mul(&Unit::Delta(kelvin)).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: "celsius".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn delta_times_point_is_not_composable() {
            let kelvin = make_kelvin_delta_unit();
            let celsius = Unit::Point(make_point_unit("celsius", kelvin.clone(), 273.15));
            let err = Unit::Delta(kelvin).try_mul(&celsius).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: "celsius".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn point_times_point_names_left_operand() {
            let kelvin = make_kelvin_delta_unit();
            let celsius1 = make_point_unit("celsius1", kelvin.clone(), 273.15);
            let celsius2 = make_point_unit("celsius2", kelvin.clone(), 273.15);
            let cases = [(&celsius1, &celsius2), (&celsius2, &celsius1)];
            for (lhs, rhs) in cases {
                let (lhs_unit, rhs_unit) = (Unit::Point(lhs.clone()), Unit::Point(rhs.clone()));
                let err = lhs_unit.try_mul(&rhs_unit).unwrap_err();
                let expected_err = UnitError::NotComposable {
                    name: lhs.name().into(),
                };
                assert!(errors_match(&err, &expected_err));
            }
        }

        #[test]
        fn delta_ops_delegate_to_delta_unit() {
            let registry_id = UnitRegistryId::next();
            let mut dims = DimRegistry::new("test-dim-reg");
            let time = dims.add_base("time", None).unwrap();
            let temperature = dims.add_base("temperature", None).unwrap();
            let second_atom = make_unit_atom(
                registry_id,
                "second",
                time,
                ConversionKind::Linear { scale: 1.0 },
            );
            let second = DeltaUnit::single(&second_atom, Exp::ONE).unwrap();
            let kelvin_atom = make_unit_atom(
                registry_id,
                "kelvin",
                temperature,
                ConversionKind::Linear { scale: 1.0 },
            );
            let kelvin = DeltaUnit::single(&kelvin_atom, Exp::ONE).unwrap();
            let (kelvin_unit, second_unit) =
                (Unit::Delta(kelvin.clone()), Unit::Delta(second.clone()));
            let prod = kelvin_unit.try_mul(&second_unit).unwrap();
            assert_eq!(
                prod,
                Unit::Delta(kelvin.try_mul(&second).unwrap()),
                "try_mul must delegate to DeltaUnit"
            );
            let ratio = kelvin_unit.try_div(&second_unit).unwrap();
            assert_eq!(
                ratio,
                Unit::Delta(kelvin.try_div(&second).unwrap()),
                "try_div must delegate to DeltaUnit"
            );
            let pow = kelvin_unit.pow(Exp::int(2)).unwrap();
            assert_eq!(
                pow,
                Unit::Delta(kelvin.pow(Exp::int(2)).unwrap()),
                "pow must delegate to DeltaUnit"
            );
            let recip = kelvin_unit.recip().unwrap();
            assert_eq!(
                recip,
                Unit::Delta(kelvin.recip().unwrap()),
                "recip must delegate to DeltaUnit"
            );
        }

        #[test]
        fn point_dividend_or_divisor_is_not_composable() {
            let kelvin_unit = make_kelvin_delta_unit();
            let celsius = Unit::Point(make_point_unit("celsius", kelvin_unit.clone(), 273.15));
            let kelvin = Unit::Delta(kelvin_unit);
            let expected_err = UnitError::NotComposable {
                name: "celsius".into(),
            };
            assert!(errors_match(
                &kelvin.try_div(&celsius).unwrap_err(),
                &expected_err
            ));
            assert!(errors_match(
                &celsius.try_div(&kelvin).unwrap_err(),
                &expected_err
            ));
        }

        #[test]
        fn point_pow_and_recip_are_not_composable() {
            let kelvin_unit = make_kelvin_delta_unit();
            let celsius = Unit::Point(make_point_unit("celsius", kelvin_unit.clone(), 273.15));
            let expected_err = UnitError::NotComposable {
                name: "celsius".into(),
            };
            assert!(errors_match(
                &celsius.pow(Exp::ONE).unwrap_err(),
                &expected_err
            ));
            assert!(errors_match(
                &celsius.pow(Exp::ZERO).unwrap_err(),
                &expected_err
            ));
            assert!(errors_match(&celsius.recip().unwrap_err(), &expected_err));
        }

        #[test]
        fn point_dimension_is_delta_dimension() {
            let kelvin_unit = make_kelvin_delta_unit();
            let celsius = Unit::Point(make_point_unit("celsius", kelvin_unit.clone(), 273.15));
            let kelvin = Unit::Delta(kelvin_unit);
            assert_eq!(celsius.dimension(), kelvin.dimension());
        }

        #[test]
        fn point_units_compare_by_identity() {
            let kelvin_unit = make_kelvin_delta_unit();
            let origin = 273.15;
            let celsius1 = Unit::Point(make_point_unit("celsius", kelvin_unit.clone(), origin));
            let celsius2 = Unit::Point(make_point_unit("celsius", kelvin_unit.clone(), origin));
            assert_eq!(
                celsius1,
                celsius1.clone(),
                "a clone shares the registration, so it must be equal"
            );
            assert_ne!(
                celsius1, celsius2,
                "same name, delta and origin but separate registrations must differ"
            );
        }

        fn hash_of<T: Hash>(value: &T) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn point_unit_hash_agrees_with_eq() {
            let kelvin_unit = make_kelvin_delta_unit();
            let origin = 273.15;
            let celsius1 = Unit::Point(make_point_unit("celsius", kelvin_unit.clone(), origin));
            assert_eq!(
                hash_of(&celsius1),
                hash_of(&celsius1.clone()),
                "equal units must hash equally"
            );
        }
    }
}
