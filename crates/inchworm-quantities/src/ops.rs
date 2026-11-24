//! Operators for quantities: each delegates to a `try_` method of [`Quantity`] and panics
//! with its error message.

use crate::{Magnitude, Quantity};

/// Implements `$trait` for the four owned/borrowed combinations of quantity operands,
/// delegating to `Quantity::$try_method` and panicking with its error message.
macro_rules! impl_quantity_op {
    ($trait:ident, $method:ident, $try_method:ident, $($bounds:tt)+) => {
        #[doc = concat!("Delegates to [`Quantity::", stringify!($try_method), "`].")]
        ///
        /// # Panics
        ///
        /// Panics with the error message in the cases where that method returns an error.
        impl<M: $($bounds)+> std::ops::$trait for &Quantity<M> {
            type Output = Quantity<M>;
            fn $method(self, rhs: &Quantity<M>) -> Self::Output {
                self.$try_method(rhs).unwrap_or_else(|e| panic!("{e}"))
            }
        }

        /// Delegates to the impl for two borrowed quantities; panics under the same conditions.
        impl<M: $($bounds)+> std::ops::$trait for Quantity<M> {
            type Output = Quantity<M>;
            fn $method(self, rhs: Quantity<M>) -> Self::Output {
                std::ops::$trait::$method(&self, &rhs)
            }
        }

        /// Delegates to the impl for two borrowed quantities; panics under the same conditions.
        impl<M: $($bounds)+> std::ops::$trait<&Quantity<M>> for Quantity<M> {
            type Output = Quantity<M>;
            fn $method(self, rhs: &Quantity<M>) -> Self::Output {
                std::ops::$trait::$method(&self, rhs)
            }
        }

        /// Delegates to the impl for two borrowed quantities; panics under the same conditions.
        impl<M: $($bounds)+> std::ops::$trait<Quantity<M>> for &Quantity<M> {
            type Output = Quantity<M>;
            fn $method(self, rhs: Quantity<M>) -> Self::Output {
                std::ops::$trait::$method(self, &rhs)
            }
        }
    };
}

impl_quantity_op!(
    Add,
    add,
    try_add,
    Magnitude<Output = M> + std::ops::Add<Output = M> + Clone
);
impl_quantity_op!(
    Sub,
    sub,
    try_sub,
    Magnitude<Output = M> + std::ops::Sub<Output = M> + Clone
);
impl_quantity_op!(Mul, mul, try_mul, std::ops::Mul<Output = M> + Clone);
impl_quantity_op!(Div, div, try_div, std::ops::Div<Output = M> + Clone);

/// Delegates to [`Quantity::try_mul_scalar`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Mul<f64, Output = M> + Clone> std::ops::Mul<f64> for Quantity<M> {
    type Output = Quantity<M>;
    fn mul(self, rhs: f64) -> Self::Output {
        self.try_mul_scalar(rhs).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_mul_scalar`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Mul<f64, Output = M> + Clone> std::ops::Mul<f64> for &Quantity<M> {
    type Output = Quantity<M>;
    fn mul(self, rhs: f64) -> Self::Output {
        self.try_mul_scalar(rhs).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// `self / rhs`: the scalar divided by the quantity, inverting the unit (`3 / 2 s` is
/// `1.5 1/s`). Delegates to [`Quantity::try_scalar_div`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: Clone> std::ops::Div<Quantity<M>> for f64
where
    f64: std::ops::Div<M, Output = M>,
{
    type Output = Quantity<M>;
    fn div(self, rhs: Quantity<M>) -> Self::Output {
        rhs.try_scalar_div(self).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// `self / rhs`: the scalar divided by the quantity, inverting the unit (`3 / 2 s` is
/// `1.5 1/s`). Delegates to [`Quantity::try_scalar_div`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: Clone> std::ops::Div<&Quantity<M>> for f64
where
    f64: std::ops::Div<M, Output = M>,
{
    type Output = Quantity<M>;
    fn div(self, rhs: &Quantity<M>) -> Self::Output {
        rhs.try_scalar_div(self).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_mul_scalar`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Mul<f64, Output = M> + Clone> std::ops::Mul<Quantity<M>> for f64 {
    type Output = Quantity<M>;
    fn mul(self, rhs: Quantity<M>) -> Self::Output {
        rhs.try_mul_scalar(self).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_mul_scalar`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Mul<f64, Output = M> + Clone> std::ops::Mul<&Quantity<M>> for f64 {
    type Output = Quantity<M>;
    fn mul(self, rhs: &Quantity<M>) -> Self::Output {
        rhs.try_mul_scalar(self).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_div_scalar`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Div<f64, Output = M> + Clone> std::ops::Div<f64> for Quantity<M> {
    type Output = Quantity<M>;
    fn div(self, rhs: f64) -> Self::Output {
        self.try_div_scalar(rhs).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_div_scalar`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Div<f64, Output = M> + Clone> std::ops::Div<f64> for &Quantity<M> {
    type Output = Quantity<M>;
    fn div(self, rhs: f64) -> Self::Output {
        self.try_div_scalar(rhs).unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_neg`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Neg<Output = M> + Clone> std::ops::Neg for &Quantity<M> {
    type Output = Quantity<M>;
    fn neg(self) -> Self::Output {
        self.try_neg().unwrap_or_else(|e| panic!("{e}"))
    }
}

/// Delegates to [`Quantity::try_neg`].
///
/// # Panics
///
/// Panics with the error message if the quantity is a reading in a point unit.
impl<M: std::ops::Neg<Output = M> + Clone> std::ops::Neg for Quantity<M> {
    type Output = Quantity<M>;
    fn neg(self) -> Self::Output {
        self.try_neg().unwrap_or_else(|e| panic!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_close, test_registry, unit};

    #[test]
    fn add_matches_try_method() {
        let ureg = test_registry();
        let a = Quantity::new(2.0, unit(&ureg, "m"));
        let b = Quantity::new(3.0, unit(&ureg, "m"));
        let sum = a.try_add(&b).unwrap();
        let sum_op = &a + &b;
        assert_close(*sum.magnitude(), *sum_op.magnitude());
        assert_eq!(sum.unit(), sum_op.unit());
    }

    #[test]
    fn sub_matches_try_method() {
        let ureg = test_registry();
        let a = Quantity::new(2.0, unit(&ureg, "m"));
        let b = Quantity::new(3.0, unit(&ureg, "m"));
        let diff = a.try_sub(&b).unwrap();
        let diff_op = &a - &b;
        assert_close(*diff.magnitude(), *diff_op.magnitude());
        assert_eq!(diff.unit(), diff_op.unit());
    }

    #[test]
    fn mul_matches_try_method() {
        let ureg = test_registry();
        let a = Quantity::new(2.0, unit(&ureg, "m"));
        let b = Quantity::new(3.0, unit(&ureg, "m"));
        let prod = a.try_mul(&b).unwrap();
        let prod_op = &a * &b;
        assert_close(*prod.magnitude(), *prod_op.magnitude());
        assert_eq!(prod.unit(), prod_op.unit());
    }

    #[test]
    fn div_matches_try_method() {
        let ureg = test_registry();
        let a = Quantity::new(2.0, unit(&ureg, "m"));
        let b = Quantity::new(3.0, unit(&ureg, "m"));
        let ratio = a.try_div(&b).unwrap();
        let ratio_op = &a / &b;
        assert_close(*ratio.magnitude(), *ratio_op.magnitude());
        assert_eq!(ratio.unit(), ratio_op.unit());
    }

    #[test]
    fn accepts_owned_and_borrowed_operands() {
        let ureg = test_registry();
        let a = Quantity::new(2.0, unit(&ureg, "m"));
        let b = Quantity::new(3.0, unit(&ureg, "m"));
        let expected = &a + &b;
        for sum in [a.clone() + b.clone(), a.clone() + &b, &a + b.clone()] {
            assert_close(*sum.magnitude(), *expected.magnitude());
            assert_eq!(sum.unit(), expected.unit());
        }
    }

    #[test]
    fn scales_on_either_side() {
        let ureg = test_registry();
        let meter = unit(&ureg, "m");
        let q = Quantity::new(2.0, meter.clone());
        let cases = [
            (&q * 3.0, 6.0),
            (3.0 * &q, 6.0),
            (q.clone() * 3.0, 6.0),
            (3.0 * q.clone(), 6.0),
            (&q / 2.0, 1.0),
            (q.clone() / 2.0, 1.0),
        ];
        for (scaled, expected) in cases {
            assert_close(*scaled.magnitude(), expected);
            assert_eq!(scaled.unit(), &meter);
        }
    }

    #[test]
    #[should_panic(expected = "cannot add two readings")]
    fn panics_with_the_error_message() {
        let ureg = test_registry();
        let reading = Quantity::new(20.0, unit(&ureg, "degC"));
        let _ = &reading + &reading;
    }

    #[test]
    #[should_panic(expected = "cannot scale a reading")]
    fn scaling_a_reading_panics() {
        let ureg = test_registry();
        let reading = Quantity::new(20.0, unit(&ureg, "degC"));
        let _ = 2.0 * &reading;
    }

    #[test]
    #[should_panic(expected = "cannot be multiplied, divided, or raised to a power")]
    fn dividing_by_a_reading_panics() {
        let ureg = test_registry();
        let reading = Quantity::new(20.0, unit(&ureg, "degC"));
        let _ = 1.0 / &reading;
    }

    #[test]
    fn negates_a_difference() {
        let ureg = test_registry();
        let negated = -&Quantity::new(3.0, unit(&ureg, "m"));
        assert_close(*negated.magnitude(), -3.0);
        assert_eq!(negated.unit(), &unit(&ureg, "m"));
    }

    #[test]
    fn divides_a_scalar_by_a_quantity() {
        let ureg = test_registry();
        let quantity = Quantity::new(2.0, unit(&ureg, "s"));
        let ratio = 3.0 / &quantity;
        assert_close(*ratio.magnitude(), 1.5);
        assert_eq!(ratio.unit(), &unit(&ureg, "1/s"));
    }
}
