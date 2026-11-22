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
    /// This quantity expressed in `target`: `5 km` to `m` is `5000 m`.
    ///
    /// # Errors
    ///
    /// Returns [`QuantityError::Unit`] wrapping whatever [`Unit::converter_to`] returns,
    /// such as incompatible or partially compatible dimensions.
    pub fn to(&self, target: &Unit) -> Result<Quantity<M::Output>, QuantityError> {
        let converter = self.unit.converter_to(target)?;
        Ok(Quantity::new(
            self.magnitude().convert(converter),
            target.clone(),
        ))
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
}
