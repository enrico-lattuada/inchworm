use inchworm_units::Unit;

use crate::{error::QuantityError, magnitude::Magnitude};

/// A magnitude in a unit: `5 km`, `20 °C`.
#[derive(Debug, Clone)]
pub struct Quantity<M> {
    magnitude: M,
    unit: Unit,
}

impl<M> Quantity<M> {
    /// The quantity `magnitude` in `unit`.
    pub fn new(magnitude: M, unit: impl Into<Unit>) -> Self {
        Self {
            magnitude,
            unit: unit.into(),
        }
    }

    /// The magnitude, in [`unit`](Self::unit).
    pub fn magnitude(&self) -> &M {
        &self.magnitude
    }

    /// The unit the magnitude is expressed in.
    pub fn unit(&self) -> &Unit {
        &self.unit
    }
}

impl<M: Magnitude> Quantity<M> {
    /// `self`'s magnitude converted to `target`.
    fn magnitude_in(&self, target: &Unit) -> Result<M::Output, QuantityError> {
        let converter = self.unit.converter_to(target)?;
        Ok(self.magnitude.convert(converter))
    }

    /// This quantity expressed in `target`: `5 km` to `m` is `5000 m`.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::Unit`] wrapping whatever [`Unit::converter_to`] returns,
    /// such as incompatible or partially compatible dimensions.
    pub fn to(&self, target: &Unit) -> Result<Quantity<M::Output>, QuantityError> {
        Ok(Quantity::new(self.magnitude_in(target)?, target.clone()))
    }
}

// ---- arithmetics ----
impl<M: Magnitude<Output = M> + std::ops::Add<Output = M> + Clone> Quantity<M> {
    /// Adds `rhs`, converted to `self`'s unit: `1 km + 500 m` is `1.5 km`.
    ///
    /// A reading in a point unit plus a difference is a reading, in either order, in the
    /// reading's unit: `20 °C + 9 delta_degF` and `9 delta_degF + 20 °C` are both `25 °C`.
    /// The difference is converted as a difference, so the offset of the point unit does not
    /// apply to it.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::Unit`] if `rhs` cannot be converted to `self`'s unit (or to the
    /// reading's delta unit), such as for incompatible dimensions (`m + s`).
    /// Returns [`QuantityError::ReadingPlusReading`] if both quantities are readings.
    pub fn try_add(&self, rhs: &Self) -> Result<Self, QuantityError> {
        match (&self.unit, &rhs.unit) {
            (Unit::Delta(_), Unit::Delta(_)) => Ok(Self::new(
                self.magnitude.clone() + rhs.magnitude_in(self.unit())?,
                self.unit.clone(),
            )),
            (Unit::Point(point), Unit::Delta(_)) => Ok(Self::new(
                self.magnitude.clone() + rhs.magnitude_in(&Unit::Delta(point.delta().clone()))?,
                self.unit.clone(),
            )),
            (Unit::Delta(_), Unit::Point(point)) => Ok(Self::new(
                self.magnitude_in(&Unit::Delta(point.delta().clone()))? + rhs.magnitude.clone(),
                rhs.unit.clone(),
            )),
            (Unit::Point(_), Unit::Point(_)) => Err(QuantityError::ReadingPlusReading {
                left: self.unit().to_string(),
                right: rhs.unit().to_string(),
            }),
        }
    }
}

impl<M: Magnitude<Output = M> + std::ops::Sub<Output = M> + Clone> Quantity<M> {
    /// Subtracts `rhs`, converted to `self`'s unit: `1 km - 500 m` is `0.5 km`.
    ///
    /// For readings in point units: a reading minus a reading is the difference between them,
    /// in the left reading's delta unit (`68 °F - 20 °C` is `0 delta_degF`); a reading minus a
    /// difference is a reading, in the reading's unit.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::Unit`] if `rhs` cannot be converted to `self`'s unit (or to the
    /// reading's delta unit), such as for incompatible dimensions (`m - s`).
    /// Returns [`QuantityError::DifferenceMinusReading`] if `self` is a difference and `rhs` a
    /// reading.
    pub fn try_sub(&self, rhs: &Self) -> Result<Self, QuantityError> {
        match (&self.unit, &rhs.unit) {
            (Unit::Delta(_), Unit::Delta(_)) => Ok(Self::new(
                self.magnitude.clone() - rhs.magnitude_in(self.unit())?,
                self.unit.clone(),
            )),
            (Unit::Point(point), Unit::Delta(_)) => Ok(Self::new(
                self.magnitude.clone() - rhs.magnitude_in(&Unit::Delta(point.delta().clone()))?,
                self.unit.clone(),
            )),
            (Unit::Delta(left), Unit::Point(right)) => Err(QuantityError::DifferenceMinusReading {
                left: left.to_string(),
                right: right.to_string(),
            }),
            (Unit::Point(point), Unit::Point(_)) => {
                let unit_out = Unit::Delta(point.delta().clone());
                Ok(Self::new(
                    self.magnitude.clone() - rhs.magnitude_in(self.unit())?,
                    unit_out,
                ))
            }
        }
    }
}

impl<M: std::ops::Mul<Output = M> + Clone> Quantity<M> {
    /// Multiplies by `rhs`: magnitudes multiply and units multiply, without converting either
    /// (`1 km * 500 m` is `500 km·m`). Use [`to`](Self::to) to express the result in another unit.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::Unit`] if the units cannot be multiplied: either is a reading in
    /// a point unit (wrapping [`NotComposable`](inchworm_units::UnitError::NotComposable)),
    /// they come from different registries, or an exponent overflows.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, QuantityError> {
        let unit = self.unit.try_mul(&rhs.unit)?;
        Ok(Self::new(
            self.magnitude().clone() * rhs.magnitude().clone(),
            unit,
        ))
    }
}

impl<M: std::ops::Div<Output = M> + Clone> Quantity<M> {
    /// Divides by `rhs`: magnitudes divide and units divide, without converting either
    /// (`1 km / 500 m` is `0.002 km/m`, not the dimensionless `2`). Use [`to`](Self::to) to
    /// express the result in another unit.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::Unit`] if the units cannot be divided, for the same reasons as
    /// [`try_mul`](Self::try_mul).
    pub fn try_div(&self, rhs: &Self) -> Result<Self, QuantityError> {
        let unit = self.unit.try_div(&rhs.unit)?;
        Ok(Self::new(
            self.magnitude().clone() / rhs.magnitude().clone(),
            unit,
        ))
    }
}

impl<M: std::ops::Mul<f64, Output = M> + Clone> Quantity<M> {
    /// Multiplies the magnitude by `factor`, keeping the unit: `2 m * 3` is `6 m`.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::ScaledReading`] if `self` is a reading in a point unit: scaling
    /// `20 °C` would depend on where the scale's zero is.
    pub fn try_mul_scalar(&self, factor: f64) -> Result<Self, QuantityError> {
        match self.unit() {
            Unit::Delta(_) => Ok(Self::new(
                self.magnitude().clone() * factor,
                self.unit.clone(),
            )),
            Unit::Point(point) => Err(QuantityError::ScaledReading {
                unit: point.to_string(),
            }),
        }
    }
}

impl<M: std::ops::Div<f64, Output = M> + Clone> Quantity<M> {
    /// Divides the magnitude by `divisor`, keeping the unit: `6 m / 3` is `2 m`. Dividing by
    /// zero follows the magnitude type (an infinite or NaN `f64`).
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::ScaledReading`] if `self` is a reading in a point unit.
    pub fn try_div_scalar(&self, divisor: f64) -> Result<Self, QuantityError> {
        match self.unit() {
            Unit::Delta(_) => Ok(Self::new(
                self.magnitude().clone() / divisor,
                self.unit.clone(),
            )),
            Unit::Point(point) => Err(QuantityError::ScaledReading {
                unit: point.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_close, test_registry, unit};
    use inchworm_units::UnitError;

    mod new {
        use super::*;

        #[test]
        fn accepts_delta_and_point_units() {
            let ureg = test_registry();
            let Unit::Delta(meter) = unit(&ureg, "m") else {
                panic!("m is not a delta unit")
            };
            let Unit::Point(celsius) = unit(&ureg, "degC") else {
                panic!("degC is not a point unit")
            };
            let length = Quantity::new(5.0, meter.clone());
            let reading = Quantity::new(20.0, celsius.clone());
            assert_eq!(*length.magnitude(), 5.0);
            assert_eq!(length.unit(), &Unit::Delta(meter));
            assert_eq!(*reading.magnitude(), 20.0);
            assert_eq!(reading.unit(), &Unit::Point(celsius));
        }
    }

    mod to {
        use super::*;

        #[test]
        fn converts_magnitude_and_sets_unit() {
            let ureg = test_registry();
            let meter = unit(&ureg, "m");
            let converted = Quantity::new(5.0, unit(&ureg, "km")).to(&meter).unwrap();
            assert_close(*converted.magnitude(), 5000.0);
            assert_eq!(converted.unit(), &meter);
        }

        #[test]
        fn converts_readings() {
            let ureg = test_registry();
            let converted = Quantity::new(20.0, unit(&ureg, "degC"))
                .to(&unit(&ureg, "degF"))
                .unwrap();
            assert_close(*converted.magnitude(), 68.0);
        }

        #[test]
        fn promotes_integer_magnitudes() {
            let ureg = test_registry();
            let converted: Quantity<f64> = Quantity::new(5_i64, unit(&ureg, "km"))
                .to(&unit(&ureg, "m"))
                .unwrap();
            assert_close(*converted.magnitude(), 5000.0);
        }

        #[test]
        fn rejects_incompatible_units() {
            let ureg = test_registry();
            let err = Quantity::new(5.0, unit(&ureg, "m"))
                .to(&unit(&ureg, "s"))
                .unwrap_err();
            assert!(
                matches!(
                    err,
                    QuantityError::Unit(UnitError::IncompatibleDimensions { .. })
                ),
                "{err:?}"
            );
        }
    }

    mod try_add {
        use super::*;

        #[test]
        fn converts_rhs_to_lhs_unit() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "km"));
            let rhs = Quantity::new(500.0, unit(&ureg, "m"));
            let sum = lhs.try_add(&rhs).unwrap();
            assert_close(*sum.magnitude(), 1.5);
            assert_eq!(sum.unit(), &unit(&ureg, "km"));
        }

        #[test]
        fn reading_plus_difference_is_a_reading() {
            let ureg = test_registry();
            let lhs = Quantity::new(20.0, unit(&ureg, "degC"));
            let rhs = Quantity::new(9.0, unit(&ureg, "delta_degF"));
            let sum = lhs.try_add(&rhs).unwrap();
            assert_close(*sum.magnitude(), 25.0);
            assert_eq!(sum.unit(), &unit(&ureg, "degC"));
        }

        #[test]
        fn difference_plus_reading_is_a_reading() {
            let ureg = test_registry();
            let celsius = unit(&ureg, "degC");
            let sum = Quantity::new(9.0, unit(&ureg, "delta_degF"))
                .try_add(&Quantity::new(20.0, celsius.clone()))
                .unwrap();
            assert_close(*sum.magnitude(), 25.0);
            assert_eq!(sum.unit(), &celsius);
        }

        #[test]
        fn rejects_two_readings() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "degC"));
            let rhs = Quantity::new(1.0, unit(&ureg, "degC"));
            let err = lhs.try_add(&rhs).unwrap_err();
            assert!(
                matches!(err, QuantityError::ReadingPlusReading { .. }),
                "{err:?}"
            );
        }

        #[test]
        fn rejects_incompatible_units() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "m"));
            let rhs = Quantity::new(1.0, unit(&ureg, "s"));
            let err = lhs.try_add(&rhs).unwrap_err();
            assert!(
                matches!(
                    err,
                    QuantityError::Unit(UnitError::IncompatibleDimensions { .. })
                ),
                "{err:?}"
            );
        }
    }

    mod try_sub {
        use super::*;

        #[test]
        fn converts_rhs_to_lhs_unit() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "km"));
            let rhs = Quantity::new(500.0, unit(&ureg, "m"));
            let diff = lhs.try_sub(&rhs).unwrap();
            assert_close(*diff.magnitude(), 0.5);
            assert_eq!(diff.unit(), &unit(&ureg, "km"));
        }

        #[test]
        fn reading_minus_reading_is_a_difference() {
            let ureg = test_registry();
            let lhs = Quantity::new(68.0, unit(&ureg, "degF"));
            let rhs = Quantity::new(20.0, unit(&ureg, "degC"));
            let diff = lhs.try_sub(&rhs).unwrap();
            assert_close(*diff.magnitude(), 0.0);
            assert_eq!(diff.unit(), &unit(&ureg, "delta_degF"));
        }

        #[test]
        fn reading_minus_difference_is_a_reading() {
            let ureg = test_registry();
            let lhs = Quantity::new(25.0, unit(&ureg, "degC"));
            let rhs = Quantity::new(9.0, unit(&ureg, "delta_degF"));
            let diff = lhs.try_sub(&rhs).unwrap();
            assert_close(*diff.magnitude(), 20.0);
            assert_eq!(diff.unit(), &unit(&ureg, "degC"));
        }

        #[test]
        fn rejects_difference_minus_reading() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "K"));
            let rhs = Quantity::new(1.0, unit(&ureg, "degC"));
            let err = lhs.try_sub(&rhs).unwrap_err();
            assert!(
                matches!(err, QuantityError::DifferenceMinusReading { .. }),
                "{err:?}"
            );
        }
    }

    mod try_mul {
        use super::*;

        #[test]
        fn multiplies_magnitudes_and_units() {
            let ureg = test_registry();
            let lhs = Quantity::new(2.0, unit(&ureg, "m"));
            let rhs = Quantity::new(3.0, unit(&ureg, "s"));
            let prod = lhs.try_mul(&rhs).unwrap();
            assert_close(*prod.magnitude(), 6.0);
            assert_eq!(prod.unit(), &ureg.parse("m*s").unwrap());
        }

        #[test]
        fn keeps_units_unconverted() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "km"));
            let rhs = Quantity::new(500.0, unit(&ureg, "m"));
            let prod = lhs.try_mul(&rhs).unwrap();
            assert_close(*prod.magnitude(), 500.0);
            assert_eq!(prod.unit(), &ureg.parse("km*m").unwrap());
        }

        #[test]
        fn rejects_readings() {
            let ureg = test_registry();
            let reading = Quantity::new(20.0, unit(&ureg, "degC"));
            let length = Quantity::new(2.0, unit(&ureg, "m"));
            for (lhs, rhs) in [(&reading, &length), (&length, &reading)] {
                let err = lhs.try_mul(rhs).unwrap_err();
                assert!(
                    matches!(err, QuantityError::Unit(UnitError::NotComposable { .. })),
                    "{err:?}"
                );
            }
        }
    }

    mod try_div {
        use super::*;

        #[test]
        fn divides_magnitudes_and_units() {
            let ureg = test_registry();
            let lhs = Quantity::new(6.0, unit(&ureg, "m"));
            let rhs = Quantity::new(3.0, unit(&ureg, "s"));
            let ratio = lhs.try_div(&rhs).unwrap();
            assert_close(*ratio.magnitude(), 2.0);
            assert_eq!(ratio.unit(), &ureg.parse("m/s").unwrap());
        }

        #[test]
        fn keeps_units_unconverted() {
            let ureg = test_registry();
            let lhs = Quantity::new(1.0, unit(&ureg, "km"));
            let rhs = Quantity::new(500.0, unit(&ureg, "m"));
            let ratio = lhs.try_div(&rhs).unwrap();
            assert_close(*ratio.magnitude(), 0.002);
            assert_eq!(ratio.unit(), &ureg.parse("km/m").unwrap());
        }

        #[test]
        fn rejects_readings() {
            let ureg = test_registry();
            let reading = Quantity::new(20.0, unit(&ureg, "degC"));
            let length = Quantity::new(2.0, unit(&ureg, "m"));
            for (lhs, rhs) in [(&reading, &length), (&length, &reading)] {
                let err = lhs.try_div(rhs).unwrap_err();
                assert!(
                    matches!(err, QuantityError::Unit(UnitError::NotComposable { .. })),
                    "{err:?}"
                );
            }
        }
    }

    mod try_mul_scalar {
        use super::*;

        #[test]
        fn scales_magnitude() {
            let ureg = test_registry();
            let quantity = Quantity::new(2.0, unit(&ureg, "m"));
            let scaled = quantity.try_mul_scalar(3.0).unwrap();
            assert_close(*scaled.magnitude(), 6.0);
            assert_eq!(scaled.unit(), &unit(&ureg, "m"));
        }

        #[test]
        fn rejects_readings() {
            let ureg = test_registry();
            let quantity = Quantity::new(2.0, unit(&ureg, "degC"));
            let err = quantity.try_mul_scalar(3.0).unwrap_err();
            assert!(
                matches!(err, QuantityError::ScaledReading { .. }),
                "{err:?}"
            );
        }
    }

    mod try_div_scalar {
        use super::*;

        #[test]
        fn scales_magnitude() {
            let ureg = test_registry();
            let quantity = Quantity::new(6.0, unit(&ureg, "m"));
            let scaled = quantity.try_div_scalar(3.0).unwrap();
            assert_close(*scaled.magnitude(), 2.0);
            assert_eq!(scaled.unit(), &unit(&ureg, "m"));
        }

        #[test]
        fn rejects_readings() {
            let ureg = test_registry();
            let quantity = Quantity::new(2.0, unit(&ureg, "degC"));
            let err = quantity.try_div_scalar(3.0).unwrap_err();
            assert!(
                matches!(err, QuantityError::ScaledReading { .. }),
                "{err:?}"
            );
        }
    }
}
