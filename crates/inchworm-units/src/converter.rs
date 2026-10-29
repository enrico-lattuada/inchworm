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

    mod converter_between {
        use super::*;
        use crate::{
            Unit, UnitRegistry,
            test_utils::{assert_close, errors_match, mks_registry},
        };
        use inchworm_dimensions::DimRegistry;

        #[test]
        fn same_dimension_scales_linearly() {
            let ureg = mks_registry();
            let km = ureg.get("km").unwrap();
            let m = ureg.get("m").unwrap();
            let converter = km.converter_to(&m).unwrap();
            assert_eq!(converter.apply(1.0), 1e3);
            assert!(converter.is_linear());
        }

        #[test]
        fn compound_angular_rate() {
            let ureg = mks_registry();
            let deg_per_h = ureg.parse("deg / h").unwrap();
            let rad_per_s = ureg.parse("rad / s").unwrap();
            let converter = deg_per_h.converter_to(&rad_per_s).unwrap();
            let pi = std::f64::consts::PI;
            assert_close(converter.apply(1.0), pi / 180.0 / 3600.0);
        }

        #[test]
        fn celsius_to_kelvin() {
            let ureg = mks_registry();
            let deg_c = ureg.get("degC").unwrap();
            let kelvin = ureg.get("K").unwrap();
            let converter = deg_c.converter_to(&kelvin).unwrap();
            assert_eq!(converter.apply(0.0), 273.15);
        }

        #[test]
        fn kelvin_to_celsius() {
            let ureg = mks_registry();
            let kelvin = ureg.get("K").unwrap();
            let deg_c = ureg.get("degC").unwrap();
            let converter = kelvin.converter_to(&deg_c).unwrap();
            assert_close(converter.apply(273.15), 0.0);
        }

        #[test]
        fn fahrenheit_to_celsius() {
            let ureg = mks_registry();
            let deg_f = ureg.get("degF").unwrap();
            let deg_c = ureg.get("degC").unwrap();
            let converter = deg_f.converter_to(&deg_c).unwrap();
            assert_close(converter.apply(32.0), 0.0);
            assert_close(converter.apply(212.0), 100.0);
        }

        #[test]
        fn round_trip_is_identity() {
            let ureg = mks_registry();
            let cases = [("degF", "degC"), ("deg / h", "rad / s")];
            let values = [-40.0, 0.0, 32.0, 100.0];
            for (left, right) in cases {
                let a = ureg.parse(left).unwrap();
                let b = ureg.parse(right).unwrap();
                let there = a.converter_to(&b).unwrap();
                let back = b.converter_to(&a).unwrap();
                for x in values {
                    assert_close(back.apply(there.apply(x)), x);
                }
            }
        }

        #[test]
        fn same_point_unit_gives_identity() {
            let ureg = mks_registry();
            let deg_f = ureg.get("degF").unwrap();
            let converter = deg_f.converter_to(&deg_f).unwrap();
            assert!(converter.is_linear());
            assert_eq!(converter.apply(32.0), 32.0);
        }

        #[test]
        fn delta_to_delta_is_linear() {
            let ureg = mks_registry();
            let km_per_h = ureg.parse("km / h").unwrap();
            let m_per_s = ureg.parse("m / s").unwrap();
            let converter = km_per_h.converter_to(&m_per_s).unwrap();
            assert!(converter.is_linear());
            assert_close(converter.apply(3.6), 1.0);
        }

        #[test]
        fn delta_unit_to_point_converts_as_reading() {
            let ureg = mks_registry();
            let delta_deg_f = ureg.get("delta_degF").unwrap();
            let deg_f = ureg.get("degF").unwrap();
            let converter = delta_deg_f.converter_to(&deg_f).unwrap();
            assert!(!converter.is_linear());
            // A delta-unit value is read as a reading from the delta's zero
            // (absolute zero here), not as a difference; see Unit::converter_to
            assert_eq!(converter.apply(10.0), 10.0 - 459.67);
        }

        #[test]
        fn rejects_incompatible_dimensions() {
            let ureg = mks_registry();
            let m = ureg.get("m").unwrap();
            let s = ureg.get("s").unwrap();
            let actual = m.converter_to(&s).unwrap_err();
            let expected = UnitError::IncompatibleDimensions {
                left: m.dimension().clone(),
                right: s.dimension().clone(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_partially_compatible_dimensions() {
            let ureg = mks_registry();
            let joule_per_rad = ureg.parse("J / rad").unwrap();
            let joule = ureg.get("J").unwrap();
            let actual = joule_per_rad.converter_to(&joule).unwrap_err();
            let expected = UnitError::PartiallyCompatibleDimensions {
                left: joule_per_rad.dimension().clone(),
                right: joule.dimension().clone(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_units_from_another_registry() {
            let ureg_1 = mks_registry();
            let ureg_2 = mks_registry();
            let m_1 = ureg_1.get("m").unwrap();
            let m_2 = ureg_2.get("m").unwrap();
            let actual = m_1.converter_to(&m_2).unwrap_err();
            let expected = UnitError::CrossRegistry {
                left: m_1.registry_id().unwrap(),
                right: m_2.registry_id().unwrap(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_dimensionless_units_from_another_registry() {
            let mut ureg_1 = UnitRegistry::new("test-ureg-1", DimRegistry::new("test-reg-1"));
            ureg_1
                .add_derived("percent", "perc", &DeltaUnit::dimensionless(), 100.0, false)
                .unwrap();
            let mut ureg_2 = UnitRegistry::new("test-ureg-2", DimRegistry::new("test-reg-2"));
            ureg_2
                .add_derived("percent", "perc", &DeltaUnit::dimensionless(), 100.0, false)
                .unwrap();
            let perc_1 = ureg_1.get("perc").unwrap();
            let perc_2 = ureg_2.get("perc").unwrap();
            let actual = perc_1.converter_to(&perc_2).unwrap_err();
            let expected = UnitError::CrossRegistry {
                left: perc_1.registry_id().unwrap(),
                right: perc_2.registry_id().unwrap(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn accepts_bare_dimensionless_with_any_registry() {
            let mut ureg = UnitRegistry::new("test-ureg", DimRegistry::new("test-reg"));
            ureg.add_derived("percent", "perc", &DeltaUnit::dimensionless(), 100.0, false)
                .unwrap();
            let perc = ureg.get("perc").unwrap();
            let converter = Unit::Delta(DeltaUnit::dimensionless()).converter_to(&perc);
            assert!(converter.is_ok());
        }

        #[test]
        fn propagates_scale_out_of_range() {
            let mut ureg = mks_registry();
            ureg.add_derived_expr("huge", "huge", "m", 1e200, false)
                .unwrap();
            let huge2 = ureg.parse("huge^2").unwrap();
            let m2 = ureg.parse("m^2").unwrap();
            let actual = huge2.converter_to(&m2).unwrap_err();
            let expected = UnitError::ScaleOutOfRange {
                unit: huge2.to_string(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_conversion_out_of_range() {
            let mut ureg = mks_registry();
            let huge = ureg
                .add_derived_expr("huge", "huge", "m", 1e200, false)
                .unwrap();
            let tiny = ureg
                .add_derived_expr("tiny", "tiny", "m", 1e-200, false)
                .unwrap();
            let actual = huge.converter_to(&tiny).unwrap_err();
            let expected = UnitError::ConversionOutOfRange {
                from: huge.to_string(),
                to: tiny.to_string(),
            };
            assert!(errors_match(&actual, &expected));
        }
    }
}
