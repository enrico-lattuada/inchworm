use crate::error::DimensionError;

/// A rational exponent, always stored in lowest terms with `den > 0`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Exp {
    num: i64,
    den: i64,
}

impl Exp {
    /// Constructs a new rational exponent with the specified numerator and denominator.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ZeroDenominator`] if the denominator is zero.
    /// Returns [`DimensionError::ExponentOverflow`] if either `num` or `den` is `i64::MIN`.
    ///
    /// # Examples
    /// ```
    /// use inchworm_dimensions::Exp;
    ///
    /// let exp = Exp::new(2, 3).unwrap();
    /// assert_eq!(Exp::new(4, 6).unwrap(), exp);
    /// ```
    pub fn new(num: i64, den: i64) -> Result<Self, DimensionError> {
        if den == 0 {
            return Err(DimensionError::ZeroDenominator);
        }
        if num == i64::MIN || den == i64::MIN {
            return Err(DimensionError::ExponentOverflow);
        }
        let gcd = gcd(num.unsigned_abs(), den.unsigned_abs()) as i64;
        let den_sign = if den < 0 { -1 } else { 1 };
        let exp = Self {
            num: den_sign * num / gcd,
            den: den_sign * den / gcd,
        };
        Ok(exp)
    }

    /// Constructs a new integer exponent.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if `n` is `i64::MIN`.
    ///
    /// # Examples
    /// ```
    /// use inchworm_dimensions::Exp;
    ///
    /// let exp = Exp::int(2).unwrap();
    /// assert_eq!(Exp::new(4, 2).unwrap(), exp);
    /// ```
    pub fn int(n: i64) -> Result<Self, DimensionError> {
        if n == i64::MIN {
            return Err(DimensionError::ExponentOverflow);
        }
        Ok(Self { num: n, den: 1 })
    }

    /// The numerator of the rational exponent.
    pub fn num(&self) -> i64 {
        self.num
    }

    /// The denominator of the rational exponent.
    pub fn den(&self) -> i64 {
        self.den
    }

    fn new_from_i128(num: i128, den: i128) -> Result<Self, DimensionError> {
        if den == 0 {
            return Err(DimensionError::ZeroDenominator);
        }
        if num == i128::MIN || den == i128::MIN {
            return Err(DimensionError::ExponentOverflow);
        }
        let gcd = gcd128(num.unsigned_abs(), den.unsigned_abs()) as i128;
        let den_sign = if den < 0 { -1 } else { 1 };
        let new_num = den_sign * num / gcd;
        let new_den = den_sign * den / gcd;
        if new_num <= i128::from(i64::MIN)
            || new_num > i128::from(i64::MAX)
            || new_den <= i128::from(i64::MIN)
            || new_den > i128::from(i64::MAX)
        {
            return Err(DimensionError::ExponentOverflow);
        }
        debug_assert!(new_den > 0, "denominator must be positive at this point");
        debug_assert_eq!(
            gcd128(new_num.unsigned_abs(), new_den.unsigned_abs()),
            1,
            "numerator/denominator must already be in lowest terms at this point"
        );
        Ok(Self {
            num: new_num as i64,
            den: new_den as i64,
        })
    }

    /// Checked exponent addition.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if overflow occurs in `self + rhs`
    ///
    /// # Examples
    /// ```
    /// use inchworm_dimensions::Exp;
    /// use inchworm_dimensions::DimensionError;
    ///
    /// let exp = Exp::new(1, 4).unwrap();
    /// let rhs = Exp::new(3, 4).unwrap();
    /// assert_eq!(exp.checked_add(rhs).unwrap(), Exp::int(1).unwrap());
    ///
    /// let overflowing_rhs = Exp::new(i64::MAX, 1).unwrap();
    /// assert!(matches!(exp.checked_add(overflowing_rhs), Err(DimensionError::ExponentOverflow)))
    /// ```
    pub fn checked_add(self, rhs: Self) -> Result<Self, DimensionError> {
        // With self=a/b and rhs=c/d, new_den=b*d and new_num=a*d+c*b
        let new_den = i128::from(self.den) * i128::from(rhs.den);
        let ad = i128::from(self.num) * i128::from(rhs.den);
        let cb = i128::from(self.den) * i128::from(rhs.num);
        let new_num = ad.checked_add(cb).ok_or(DimensionError::ExponentOverflow)?;
        Self::new_from_i128(new_num, new_den)
    }

    /// Checked exponent multiplication.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if overflow occurs in `self * rhs`
    ///
    /// # Examples
    /// ```
    /// use inchworm_dimensions::Exp;
    /// use inchworm_dimensions::DimensionError;
    ///
    /// let exp = Exp::new(1, 2).unwrap();
    /// let rhs = Exp::new(2, 3).unwrap();
    /// assert_eq!(exp.checked_mul(rhs).unwrap(), Exp::new(1, 3).unwrap());
    ///
    /// let overflowing_rhs = Exp::new(1, i64::MAX).unwrap();
    /// assert!(matches!(exp.checked_mul(overflowing_rhs), Err(DimensionError::ExponentOverflow)))
    /// ```
    pub fn checked_mul(self, rhs: Self) -> Result<Self, DimensionError> {
        let new_num = i128::from(self.num) * i128::from(rhs.num);
        let new_den = i128::from(self.den) * i128::from(rhs.den);
        Self::new_from_i128(new_num, new_den)
    }
}

// Computes the greatest common divisor of `a` and `b`.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

// Computes the greatest common divisor of `a` and `b` for u128 input.
fn gcd128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_exp_normalization() {
        let cases = [
            ((2, 4), (1, 2)),
            ((1, -2), (-1, 2)),
            ((0, 3), (0, 1)),
            ((0, -8), (0, 1)),
            ((14, -42), (-1, 3)),
            ((-18, -24), (3, 4)),
            ((-7, 3), (-7, 3)),
            ((i64::MAX, 1), (i64::MAX, 1)),
            ((i64::MIN + 1, 1), (i64::MIN + 1, 1)),
        ];
        for (input, expected) in cases {
            let exp = Exp::new(input.0, input.1).unwrap();
            let expected_exp = Exp {
                num: expected.0,
                den: expected.1,
            };
            assert_eq!(exp, expected_exp, "Exp::new({input:?}) failed.");
        }
    }

    #[test]
    fn new_exp_returns_error_for_zero_denominator() {
        assert!(matches!(
            Exp::new(1, 0),
            Err(DimensionError::ZeroDenominator)
        ));
    }

    #[test]
    fn new_exp_returns_error_for_exponent_overflow() {
        let cases = [(i64::MIN, 1), (1, i64::MIN)];
        for case in cases {
            assert!(matches!(
                Exp::new(case.0, case.1),
                Err(DimensionError::ExponentOverflow)
            ));
        }
    }

    #[test]
    fn int_exp() {
        assert_eq!(Exp::int(3).unwrap(), Exp { num: 3, den: 1 });
    }

    #[test]
    fn int_exp_returns_error_for_exponent_overflow() {
        assert!(matches!(
            Exp::int(i64::MIN),
            Err(DimensionError::ExponentOverflow)
        ));
    }

    #[test]
    fn exp_checked_mul() {
        let cases = [
            ((0, 1), (5, 3), (0, 1)),
            ((1, 2), (1, 1), (1, 2)),
            ((1, 2), (1, 2), (1, 4)),
            ((1, 1), (1, 2), (1, 2)),
            ((4, 3), (3, 2), (2, 1)),
            ((4, 7), (-1, 3), (-4, 21)),
            ((-1, 4), (3, 4), (-3, 16)),
            ((-7, 3), (-7, 3), (49, 9)),
            ((i64::MAX, 2), (2, i64::MAX), (1, 1)),
            ((i64::MIN + 1, 3), (-3, i64::MAX), (1, 1)),
        ];
        for (lhs, rhs, expected) in cases {
            let lhs_exp = Exp {
                num: lhs.0,
                den: lhs.1,
            };
            let rhs_exp = Exp {
                num: rhs.0,
                den: rhs.1,
            };
            let expected_exp = Exp {
                num: expected.0,
                den: expected.1,
            };
            assert_eq!(lhs_exp.checked_mul(rhs_exp).unwrap(), expected_exp);
        }
    }

    #[test]
    fn checked_mul_returns_error_for_exponent_overflow() {
        let cases = [
            ((i64::MAX, 1), (2, 1)),
            ((i64::MIN + 1, 1), (2, 1)),
            (((i64::MAX - 1) / 2 + 1, 1), (2, 1)),
            ((i64::MIN / 2, 1), (2, 1)),
            ((1, i64::MAX), (1, 2)),
            ((1, i64::MIN + 1), (1, 2)),
            ((1, (i64::MAX - 1) / 2 + 1), (1, 2)),
            ((1, i64::MIN / 2), (1, 2)),
        ];
        for (lhs, rhs) in cases {
            let lhs_exp = Exp {
                num: lhs.0,
                den: lhs.1,
            };
            let rhs_exp = Exp {
                num: rhs.0,
                den: rhs.1,
            };
            assert!(matches!(
                lhs_exp.checked_mul(rhs_exp),
                Err(DimensionError::ExponentOverflow)
            ));
        }
    }

    #[test]
    fn exp_checked_add() {
        let cases = [
            ((0, 1), (5, 3), (5, 3)),
            ((1, 2), (1, 1), (3, 2)),
            ((1, 2), (1, 2), (1, 1)),
            ((1, 1), (1, 2), (3, 2)),
            ((4, 3), (3, 2), (17, 6)),
            ((4, 7), (-1, 3), (5, 21)),
            ((-4, 7), (1, 3), (-5, 21)),
            ((-1, 4), (3, 4), (1, 2)),
            ((-7, 3), (7, 3), (0, 1)),
            ((i64::MAX - 1, i64::MAX), (1, i64::MAX), (1, 1)),
            (
                (1, i64::MAX - 1),
                (1, i64::MAX - 1),
                (1, (i64::MAX - 1) / 2),
            ),
        ];
        for (lhs, rhs, expected) in cases {
            let lhs_exp = Exp {
                num: lhs.0,
                den: lhs.1,
            };
            let rhs_exp = Exp {
                num: rhs.0,
                den: rhs.1,
            };
            let expected_exp = Exp {
                num: expected.0,
                den: expected.1,
            };
            assert_eq!(lhs_exp.checked_add(rhs_exp).unwrap(), expected_exp);
        }
    }

    #[test]
    fn checked_add_returns_error_for_exponent_overflow() {
        let cases = [
            ((i64::MAX, 1), (1, 1)),
            ((i64::MIN + 1, 1), (-1, 1)),
            ((1, i64::MAX), (1, 1)),
            ((1, i64::MIN + 1), (-1, 1)),
        ];
        for (lhs, rhs) in cases {
            let lhs_exp = Exp {
                num: lhs.0,
                den: lhs.1,
            };
            let rhs_exp = Exp {
                num: rhs.0,
                den: rhs.1,
            };
            assert!(matches!(
                lhs_exp.checked_add(rhs_exp),
                Err(DimensionError::ExponentOverflow)
            ));
        }
    }
}
