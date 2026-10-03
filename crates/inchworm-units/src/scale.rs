//! [`Scale`]: how a derived unit relates to its definition.

use inchworm_dimensions::Exp;

/// How a derived unit relates to its definition.
///
/// `#[non_exhaustive]`: logarithmic scales may be added later, so `match`es
/// outside this crate need a wildcard arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    /// `1 unit = factor × definition`
    Linear(f64),
}

impl Scale {
    /// The identity scale.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "starting value when expanding a unit to base units"
        )
    )]
    pub(crate) const ONE: Self = Self::Linear(1.0);

    /// Composes two scales.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "combines factor scales when expanding a unit to base units"
        )
    )]
    pub(crate) fn mul(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Linear(a), Self::Linear(b)) => Self::Linear(a * b),
        }
    }

    /// Raises the scale to `e`.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "raises a factor's scale to its exponent when expanding a unit to base units"
        )
    )]
    pub(crate) fn pow(self, e: Exp) -> Self {
        match self {
            Self::Linear(a) => Self::Linear(a.powf(e.to_f64())),
        }
    }

    /// Checks the scale is finite and strictly positive.
    pub(crate) fn is_valid(self) -> bool {
        match self {
            Self::Linear(a) => a > 0.0 && a.is_finite(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod mul {
        use super::*;

        #[test]
        fn one_is_identity() {
            let scale = Scale::Linear(42.0);
            assert_eq!(scale, scale.mul(Scale::ONE));
        }

        #[test]
        fn returns_prod_for_linears() {
            assert_eq!(
                Scale::Linear(2.0).mul(Scale::Linear(3.0)),
                Scale::Linear(6.0)
            );
        }
    }

    mod pow {
        use super::*;

        #[test]
        fn returns_pow_of_scale_for_linear() {
            let scale = Scale::Linear(1e6);
            assert_eq!(scale.pow(Exp::new(1, 2).unwrap()), Scale::Linear(1e3));
        }

        #[test]
        fn zero_gives_one() {
            let scale = Scale::Linear(42.0);
            assert_eq!(scale.pow(Exp::ZERO), Scale::ONE);
        }

        #[test]
        fn minus_one_gives_reciprocal() {
            let scale = Scale::Linear(2.0);
            assert_eq!(scale.pow(Exp::int(-1)), Scale::Linear(0.5));
        }
    }

    mod is_valid {
        use super::*;

        #[test]
        fn accepts_positive_finite_value() {
            let scale = Scale::Linear(42.0);
            assert!(scale.is_valid());
        }

        #[test]
        fn rejects_linear_zero() {
            let scale = Scale::Linear(0.0);
            assert!(!scale.is_valid());
        }

        #[test]
        fn rejects_linear_negative() {
            let scale = Scale::Linear(-1.0);
            assert!(!scale.is_valid());
        }

        #[test]
        fn rejects_linear_nan() {
            let scale = Scale::Linear(f64::NAN);
            assert!(!scale.is_valid());
        }

        #[test]
        fn rejects_linear_infinity() {
            let scale = Scale::Linear(f64::INFINITY);
            assert!(!scale.is_valid());
        }
    }
}
