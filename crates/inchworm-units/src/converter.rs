//! [`Converter`]: a reusable conversion between two compatible units.

use crate::scale::Scale;

/// A reusable conversion between two compatible units: a reading `x` in the
/// source unit is `apply(x) = x · factor + offset` in the target unit.
///
/// The conversion is computed and validated once, so applying it to many values
/// is a multiply-add each. The offset is non-zero only when a point unit
/// (`degC`, `degF`) is involved; see [`is_linear`](Self::is_linear).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Converter {
    factor: f64,
    offset: f64,
}

impl Converter {
    /// Builds a converter from a validated `factor` and an `offset` in target units.
    ///
    /// `None` if `offset` is not finite.
    #[cfg_attr(not(test), expect(dead_code, reason = "called by converter_to"))]
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
    pub fn is_linear(&self) -> bool {
        self.offset == 0.0
    }
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
