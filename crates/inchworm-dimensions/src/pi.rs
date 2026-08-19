//! Buckingham 'pi' theorem analysis.
//!
//! Given variables v_1...v_n with dimensions, find a basis of the nullspace of
//! the dimensional matrix over Q. Each nullspace vector is a dimensionless 'pi'
//! group; there are `n − rank` of them.

use crate::{Dimension, DimensionError, Exp, exp::gcd, linalg::RatMatrix};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PiVariable {
    /// e.g., "v", "rho", "mu"
    pub name: String,
    pub dimension: Dimension,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PiGroup {
    /// One exponent per input variable, same order as the input slice.
    /// Scaled to smallest integers (lcm of denominators cleared) and
    /// sign-normalized so the first nonzero exponent is positive. Kept as
    /// [`Exp`] for API uniformity, but always integral after normalization.
    pub exponents: Vec<Exp>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PiAnalysis {
    /// Rank of the dimensional matrix.
    pub rank: usize,
    /// `n - rank` groups (possibly empty).
    pub groups: Vec<PiGroup>,
}

/// Options for Buckingham pi analysis.
#[derive(Clone, Copy, Debug, Default)]
pub struct PiOptions {
    /// If true, named dimensionless kinds (plane_angle, strain, ...) act as
    /// independent rows of the dimensional matrix (canonical form), so e.g.,
    /// angular frequency and frequency are not conflated when forming groups.
    /// If false (the default), classic textbook behavior over base signatures.
    pub distinguish_named_dimensionless: bool,
}

/// Normalizes a raw nullspace basis vector into the canonical form used by
/// [`PiGroup`]: scaled to the smallest integers that preserve the ratios
/// between them (denominators cleared via their LCM), then sign-normalized
/// so the first nonzero exponent is positive.
///
/// # Panics
///
/// Panics if `exponents` has no nonzero entry. Every nullspace basis vector
/// has a nonzero entry at its free-column position, so this never happens
/// when called on genuine nullspace output.
///
/// # Errors
///
/// Returns [`DimensionError::ExponentOverflow`] if computing the LCM of the
/// denominators, scaling by it, or negating for sign normalization overflows.
fn normalize(exponents: Vec<Exp>) -> Result<Vec<Exp>, DimensionError> {
    // Find the lcm of exp denominators
    let mut lcm_den: i64 = 1;
    for exp in &exponents {
        let a = lcm_den as u64;
        let b = exp.den() as u64;
        let g = gcd(a, b);
        let candidate = i128::from(a / g) * i128::from(b);
        if candidate > i128::from(i64::MAX) {
            return Err(DimensionError::ExponentOverflow);
        }
        lcm_den = candidate as i64;
    }
    let lcm = Exp::int(lcm_den)?;
    // Scale the exponents by lcm
    let scaled = exponents
        .into_iter()
        .map(|e| e.checked_mul(lcm))
        .collect::<Result<Vec<_>, _>>()?;
    // If the first non-zero entry is negative, negate all elements
    let negate = scaled
        .iter()
        .find(|&e| !e.is_zero())
        .expect("there must be at least one non-zero entry at this point")
        .num()
        .is_negative();
    if negate {
        scaled
            .into_iter()
            .map(Exp::checked_neg)
            .collect::<Result<Vec<_>, _>>()
    } else {
        Ok(scaled)
    }
}

/// Perform Buckingham pi analysis.
///
/// # Errors
/// Returns [`DimensionError::EmptyPiInput`] if no variables are given.
/// Returns [`DimensionError::CrossRegistry`] if variables contain dimensions
/// defined in different registries.
/// Returns [`DimensionError::ExponentOverflow`] if overflow occurs during operations.
pub fn buckingham_pi(
    variables: &[PiVariable],
    options: PiOptions,
) -> Result<PiAnalysis, DimensionError> {
    if variables.is_empty() {
        return Err(DimensionError::EmptyPiInput);
    }
    let (mut matrix, _) = RatMatrix::from_dims(variables, options.distinguish_named_dimensionless)?;
    let pivots = matrix.rref()?;
    if pivots.len() == variables.len() {
        return Ok(PiAnalysis {
            rank: pivots.len(),
            groups: Vec::new(),
        });
    }
    let groups = matrix
        .nullspace()?
        .into_iter()
        .map(|v| normalize(v).map(|exponents| PiGroup { exponents }))
        .collect::<Result<Vec<_>, _>>()?;
    let rank = variables.len() - groups.len();
    Ok(PiAnalysis { rank, groups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::errors_match;

    mod normalize {
        use super::*;

        #[test]
        fn leaves_already_normalized_vector_unchanged() {
            let exponents = vec![
                Exp::int(2).unwrap(),
                Exp::int(-1).unwrap(),
                Exp::int(3).unwrap(),
            ];
            let normalized = normalize(exponents.clone()).unwrap();
            assert_eq!(normalized, exponents);
        }

        #[test]
        fn clears_fractional_denominators_via_lcm() {
            let exponents = vec![Exp::new(1, 2).unwrap(), Exp::new(1, 3).unwrap()];
            let normalized = normalize(exponents).unwrap();
            let expected = vec![Exp::int(3).unwrap(), Exp::int(2).unwrap()];
            assert_eq!(normalized, expected);
        }

        #[test]
        fn negates_when_leading_nonzero_is_negative() {
            let exponents = vec![
                Exp::int(-1).unwrap(),
                Exp::int(-1).unwrap(),
                Exp::int(-1).unwrap(),
                Exp::ONE,
            ];
            let normalized = normalize(exponents).unwrap();
            let expected = vec![Exp::ONE, Exp::ONE, Exp::ONE, Exp::int(-1).unwrap()];
            assert_eq!(normalized, expected);
        }

        #[test]
        fn skips_leading_zeros_when_finding_sign() {
            let exponents = vec![Exp::ZERO, Exp::int(-2).unwrap(), Exp::ONE];
            let normalized = normalize(exponents).unwrap();
            let expected = vec![Exp::ZERO, Exp::int(2).unwrap(), Exp::int(-1).unwrap()];
            assert_eq!(normalized, expected);
        }

        #[test]
        fn propagates_overflow_from_lcm_computation() {
            let exponents = vec![Exp::new(1, 2).unwrap(), Exp::new(1, i64::MAX).unwrap()];
            // LCM is 2 * i64::MAX
            let err = normalize(exponents).unwrap_err();
            let expected_err = DimensionError::ExponentOverflow;
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn propagates_overflow_from_scaling() {
            let exponents = vec![Exp::int(2).unwrap(), Exp::new(1, i64::MAX).unwrap()];
            // LCM is i64::MAX, multiplication leads to 2 * i64::MAX
            let err = normalize(exponents).unwrap_err();
            let expected_err = DimensionError::ExponentOverflow;
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod buckingham_pi {
        use super::*;
        use crate::DimRegistry;

        #[test]
        fn returns_empty_pi_input_error_for_no_variables() {
            let err = buckingham_pi(&[], PiOptions::default()).unwrap_err();
            let expected_err = DimensionError::EmptyPiInput;
            assert!(errors_match(&err, &expected_err))
        }

        #[test]
        fn returns_no_groups_for_full_rank_variables() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            registry.add_base("time", Some("T")).unwrap();
            let variables = [
                PiVariable {
                    name: "T".into(),
                    dimension: registry.parse("time").unwrap(),
                },
                PiVariable {
                    name: "D".into(),
                    dimension: registry.parse("length").unwrap(),
                },
            ];
            let pi_analysis = buckingham_pi(&variables, PiOptions::default()).unwrap();
            let expected = PiAnalysis {
                rank: 2,
                groups: vec![],
            };
            assert_eq!(pi_analysis, expected);
        }

        #[test]
        fn computes_reynolds_number_group() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            registry.add_base("time", Some("T")).unwrap();
            registry.add_base("mass", Some("M")).unwrap();
            let variables = [
                PiVariable {
                    name: "ρ".into(),
                    dimension: registry.parse("mass / length^3").unwrap(),
                },
                PiVariable {
                    name: "u".into(),
                    dimension: registry.parse("length / time").unwrap(),
                },
                PiVariable {
                    name: "D".into(),
                    dimension: registry.parse("length").unwrap(),
                },
                PiVariable {
                    name: "μ".into(),
                    dimension: registry.parse("mass / (length * time)").unwrap(),
                },
            ];
            let pi_analysis = buckingham_pi(&variables, PiOptions::default()).unwrap();
            let expected = PiAnalysis {
                rank: 3,
                groups: vec![PiGroup {
                    exponents: vec![Exp::ONE, Exp::ONE, Exp::ONE, Exp::int(-1).unwrap()],
                }],
            };
            assert_eq!(pi_analysis, expected);
        }

        #[test]
        fn computes_multiple_pi_groups() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            registry.add_base("time", Some("T")).unwrap();
            let variables = [
                PiVariable {
                    name: "D".into(),
                    dimension: registry.parse("length").unwrap(),
                },
                PiVariable {
                    name: "T".into(),
                    dimension: registry.parse("time").unwrap(),
                },
                PiVariable {
                    name: "A".into(),
                    dimension: registry.parse("length * time").unwrap(),
                },
                PiVariable {
                    name: "u".into(),
                    dimension: registry.parse("length / time").unwrap(),
                },
            ];
            // length: [1, 0, 1, 1]
            // time:   [0, 1, 1, -1]
            let pi_analysis = buckingham_pi(&variables, PiOptions::default()).unwrap();
            // Result is
            // [ -1, -1, 1, 0
            //   -1,  1, 0, 1 ]
            // (negated)
            let group1 = PiGroup {
                exponents: vec![Exp::ONE, Exp::ONE, Exp::int(-1).unwrap(), Exp::ZERO],
            };
            let group2 = PiGroup {
                exponents: vec![
                    Exp::ONE,
                    Exp::int(-1).unwrap(),
                    Exp::ZERO,
                    Exp::int(-1).unwrap(),
                ],
            };
            let expected = PiAnalysis {
                rank: 2,
                groups: vec![group1, group2],
            };
            assert_eq!(pi_analysis, expected);
        }

        #[test]
        fn propagates_cross_registry_error() {
            let mut registry1 = DimRegistry::new("test-reg1");
            registry1.add_base("length", Some("L")).unwrap();
            let mut registry2 = DimRegistry::new("test-reg2");
            registry2.add_base("time", Some("T")).unwrap();
            let variables = [
                PiVariable {
                    name: "time".into(),
                    dimension: registry2.parse("time").unwrap(),
                },
                PiVariable {
                    name: "length".into(),
                    dimension: registry1.parse("length").unwrap(),
                },
            ];
            let err = buckingham_pi(&variables, PiOptions::default()).unwrap_err();
            let expected_err = DimensionError::CrossRegistry {
                left: registry2.id(),
                right: registry1.id(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }
}
