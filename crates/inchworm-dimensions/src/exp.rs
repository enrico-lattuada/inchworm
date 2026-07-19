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
    pub fn new(num: i64, den: i64) -> Result<Exp, DimensionError> {
        if den == 0 {
            return Err(DimensionError::ZeroDenominator);
        }
        if num == i64::MIN || den == i64::MIN {
            return Err(DimensionError::ExponentOverflow);
        }
        let gcd = self::gcd(num.unsigned_abs(), den.unsigned_abs()) as i64;
        let den_sign = if den < 0 { -1 } else { 1 };
        let exp = Exp {
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
    pub fn int(n: i64) -> Result<Exp, DimensionError> {
        if n == i64::MIN {
            return Err(DimensionError::ExponentOverflow);
        }
        Ok(Exp { num: n, den: 1 })
    }

    /// The numerator of the rational exponent.
    pub fn num(&self) -> i64 {
        self.num
    }

    /// The denominator of the rational exponent.
    pub fn den(&self) -> i64 {
        self.den
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
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
}
