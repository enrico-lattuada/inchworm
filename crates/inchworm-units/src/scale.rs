//! [`Scale`]: how a derived unit relates to its definition.

use inchworm_dimensions::Exp;

/// How a derived unit relates to its definition: `1 unit = factor × definition`.
///
/// Always positive, finite, and normal (not subnormal), so it keeps full f64 precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Scale(f64);

impl Scale {
    /// The identity scale.
    pub(crate) const ONE: Self = Self(1.0);

    /// `None` if `value` is zero, negative, subnormal, infinite, or NaN.
    pub(crate) fn new(value: f64) -> Option<Self> {
        (value > 0.0 && value.is_normal()).then_some(Self(value))
    }

    /// The value of the scale.
    pub(crate) fn value(self) -> f64 {
        self.0
    }
}

// ---- algebra ----
impl Scale {
    /// Multiplies `self` by `rhs`; `None` if the product is out of range.
    pub(crate) fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::new(self.value() * rhs.value())
    }

    /// Divides `self` by `rhs`; `None` if the quotient is out of range.
    ///
    /// A true division, not `self × rhs⁻¹`: `x / x` is exactly `1` in IEEE
    /// arithmetic, so equal scales give an exact identity factor.
    pub(crate) fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::new(self.value() / rhs.value())
    }

    /// Raises `self` to `e`; `None` if the power is out of range.
    pub(crate) fn checked_pow(self, e: Exp) -> Option<Self> {
        Self::new(self.value().powf(e.to_f64()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn accepts_positive_finite_value() {
            let scale = Scale::new(42.0);
            assert!(scale.is_some());
        }

        #[test]
        fn accepts_min_positive() {
            let scale = Scale::new(f64::MIN_POSITIVE);
            assert!(scale.is_some());
        }

        #[test]
        fn rejects_subnormal() {
            let scale = Scale::new(f64::MIN_POSITIVE / 2.0);
            assert!(scale.is_none());
        }

        #[test]
        fn rejects_zero() {
            let scale = Scale::new(0.0);
            assert!(scale.is_none());
        }

        #[test]
        fn rejects_negative() {
            let scale = Scale::new(-1.0);
            assert!(scale.is_none());
        }

        #[test]
        fn rejects_nan() {
            let scale = Scale::new(f64::NAN);
            assert!(scale.is_none());
        }

        #[test]
        fn rejects_infinity() {
            let scale = Scale::new(f64::INFINITY);
            assert!(scale.is_none());
        }
    }

    mod checked_mul {
        use super::*;

        #[test]
        fn one_is_identity() {
            let scale = Scale(42.0);
            assert_eq!(scale.checked_mul(Scale::ONE), Some(scale));
        }

        #[test]
        fn returns_product() {
            assert_eq!(Scale(2.0).checked_mul(Scale(3.0)), Some(Scale(6.0)));
        }

        #[test]
        fn rejects_overflow() {
            let scale = Scale(1e200);
            assert!(scale.checked_mul(scale).is_none())
        }

        #[test]
        fn rejects_underflow_to_zero() {
            let scale = Scale(1e-200);
            assert!(scale.checked_mul(scale).is_none())
        }

        #[test]
        fn rejects_underflow_to_subnormal() {
            assert!(Scale(1e-154).checked_mul(Scale(1e-155)).is_none())
        }
    }

    mod checked_div {
        use super::*;

        #[test]
        fn returns_quotient() {
            assert_eq!(Scale(6.0).checked_div(Scale(3.0)), Some(Scale(2.0)));
        }

        #[test]
        fn equal_scales_give_exactly_one() {
            assert_eq!(Scale(3.0).checked_div(Scale(3.0)), Some(Scale::ONE));
        }

        #[test]
        fn rejects_overflow() {
            assert!(Scale(1e200).checked_div(Scale(1e-200)).is_none());
        }

        #[test]
        fn rejects_underflow() {
            assert!(Scale(1e-200).checked_div(Scale(1e200)).is_none());
        }
    }

    mod checked_pow {
        use super::*;

        #[test]
        fn returns_pow_of_scale() {
            let scale = Scale(1e6);
            assert_eq!(scale.checked_pow(Exp::new(1, 2).unwrap()), Some(Scale(1e3)));
        }

        #[test]
        fn zero_gives_one() {
            let scale = Scale(42.0);
            assert_eq!(scale.checked_pow(Exp::ZERO), Some(Scale::ONE));
        }

        #[test]
        fn minus_one_gives_reciprocal() {
            let scale = Scale(2.0);
            assert_eq!(scale.checked_pow(Exp::int(-1)), Some(Scale(0.5)));
        }

        #[test]
        fn rejects_overflow() {
            let scale = Scale(1e200);
            assert!(scale.checked_pow(Exp::int(2)).is_none())
        }

        #[test]
        fn rejects_negative_exponent_overflow() {
            let scale = Scale(1e-200);
            assert!(scale.checked_pow(Exp::int(-2)).is_none())
        }
    }
}
