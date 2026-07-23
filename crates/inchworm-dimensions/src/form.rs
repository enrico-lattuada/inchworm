use std::cmp::Ordering;

use smallvec::SmallVec;

use crate::atom::AtomId;
use crate::error::DimensionError;
use crate::exp::Exp;

const MAX_INLINE_FACTORS: usize = 4;

/// A reduced product of powers over named atoms.
///
/// Invariants:
/// - sorted by `AtomId` ascending
/// - no zero exponents
/// - no duplicates.
///
/// Used for both the base signature and the canonical form of a `Dimension`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    entries: SmallVec<[(AtomId, Exp); MAX_INLINE_FACTORS]>,
}

impl Form {
    /// Returns `true` if `self` has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Merges two forms, combining exponents of shared atoms, pruning any that cancel to zero.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    pub(crate) fn mul(&self, rhs: &Self) -> Result<Self, DimensionError> {
        let mut entries = SmallVec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.entries.len() && j < rhs.entries.len() {
            let (id_a, exp_a) = self.entries[i];
            let (id_b, exp_b) = rhs.entries[j];
            match id_a.cmp(&id_b) {
                Ordering::Less => {
                    entries.push((id_a, exp_a));
                    i += 1;
                }
                Ordering::Greater => {
                    entries.push((id_b, exp_b));
                    j += 1;
                }
                Ordering::Equal => {
                    let exp_sum = exp_a.checked_add(exp_b)?;
                    if !exp_sum.is_zero() {
                        entries.push((id_a, exp_sum));
                    }
                    (i, j) = (i + 1, j + 1);
                }
            }
        }
        entries.extend_from_slice(&self.entries[i..]);
        entries.extend_from_slice(&rhs.entries[j..]);
        Ok(Self { entries })
    }

    /// Raises `self` to the power of `e`, pruning any that cancels to zero.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    pub(crate) fn pow(&self, e: Exp) -> Result<Self, DimensionError> {
        let mut entries = SmallVec::new();
        if !e.is_zero() {
            for (atom_id, exp) in self.entries.iter().copied() {
                let exp_times_e = exp.checked_mul(e)?;
                entries.push((atom_id, exp_times_e));
            }
        }
        Ok(Self { entries })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use smallvec::smallvec;

    fn make_form_entry(id: u64, num_den: (i64, i64)) -> (AtomId, Exp) {
        let (num, den) = num_den;
        let exp = Exp::new(num, den).unwrap();
        (AtomId::new(id), exp)
    }

    #[test]
    fn test_form_is_empty() {
        let empty_form = Form {
            entries: smallvec![],
        };
        let entries = smallvec![make_form_entry(0, (1, 1)),];
        let form = Form { entries };
        assert!(
            empty_form.is_empty(),
            "is_empty() should return true for empty form"
        );
        assert!(
            !form.is_empty(),
            "is_empty() should return false for empty form"
        );
    }

    #[test]
    fn test_mul_empty_form() {
        let empty_form = Form {
            entries: smallvec![],
        };
        let entries = smallvec![make_form_entry(0, (1, 1)),];
        let form = Form { entries };
        assert_eq!(
            form.mul(&empty_form).unwrap(),
            form.clone(),
            "form multiplied by empty form should return form"
        );
        assert_eq!(
            empty_form.mul(&form).unwrap(),
            form.clone(),
            "empty form multiplied by form should return form"
        );
        assert_eq!(
            empty_form.mul(&empty_form).unwrap(),
            empty_form.clone(),
            "product of empty forms should return empty form"
        );
    }

    #[test]
    fn test_blocked_forms_mul() {
        let entries1 = smallvec![make_form_entry(0, (1, 2)), make_form_entry(1, (1, 3)),];
        let entries2 = smallvec![make_form_entry(2, (1, 1)), make_form_entry(3, (5, 4)),];
        let mul_entries = smallvec![
            make_form_entry(0, (1, 2)),
            make_form_entry(1, (1, 3)),
            make_form_entry(2, (1, 1)),
            make_form_entry(3, (5, 4))
        ];
        let form1 = Form { entries: entries1 };
        let form2 = Form { entries: entries2 };
        let form1_x_form2 = Form {
            entries: mul_entries,
        };
        assert_eq!(form1.mul(&form2).unwrap(), form1_x_form2);
        assert_eq!(form2.mul(&form1).unwrap(), form1_x_form2);
    }

    #[test]
    fn test_interleaved_forms_mul() {
        let entries1 = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (1, 3)),];
        let entries2 = smallvec![make_form_entry(1, (1, 1)), make_form_entry(3, (5, 4)),];
        let mul_entries = smallvec![
            make_form_entry(0, (1, 2)),
            make_form_entry(1, (1, 1)),
            make_form_entry(2, (1, 3)),
            make_form_entry(3, (5, 4))
        ];
        let form1 = Form { entries: entries1 };
        let form2 = Form { entries: entries2 };
        let form1_x_form2 = Form {
            entries: mul_entries,
        };
        assert_eq!(form1.mul(&form2).unwrap(), form1_x_form2);
    }

    #[test]
    fn test_fully_overlapping_forms_mul() {
        let entries1 = smallvec![make_form_entry(0, (1, 2)), make_form_entry(1, (1, 3)),];
        let entries2 = smallvec![make_form_entry(0, (1, 1)), make_form_entry(1, (5, 4)),];
        let mul_entries = smallvec![make_form_entry(0, (3, 2)), make_form_entry(1, (19, 12))];
        let form1 = Form { entries: entries1 };
        let form2 = Form { entries: entries2 };
        let form1_x_form2 = Form {
            entries: mul_entries,
        };
        assert_eq!(form1.mul(&form2).unwrap(), form1_x_form2);
    }

    #[test]
    fn test_zero_exp_result_forms_mul() {
        let entries1 = smallvec![make_form_entry(0, (1, 2)),];
        let entries2 = smallvec![make_form_entry(0, (-1, 2)),];
        let mul_entries = smallvec![];
        let form1 = Form { entries: entries1 };
        let form2 = Form { entries: entries2 };
        let form1_x_form2 = Form {
            entries: mul_entries,
        };
        assert_eq!(form1.mul(&form2).unwrap(), form1_x_form2);
    }

    #[test]
    fn test_generic_forms_mul() {
        let entries1 = smallvec![
            make_form_entry(0, (1, 2)),
            make_form_entry(2, (1, 3)),
            make_form_entry(4, (1, 3)),
            make_form_entry(5, (1, 3))
        ];
        let entries2 = smallvec![
            make_form_entry(1, (1, 1)),
            make_form_entry(3, (5, 4)),
            make_form_entry(4, (-1, 3)),
            make_form_entry(5, (1, 2))
        ];
        let mul_entries = smallvec![
            make_form_entry(0, (1, 2)),
            make_form_entry(1, (1, 1)),
            make_form_entry(2, (1, 3)),
            make_form_entry(3, (5, 4)),
            make_form_entry(5, (5, 6)),
        ];
        let form1 = Form { entries: entries1 };
        let form2 = Form { entries: entries2 };
        let form1_x_form2 = Form {
            entries: mul_entries,
        };
        assert_eq!(form1.mul(&form2).unwrap(), form1_x_form2);
    }

    #[test]
    fn test_forms_mul_err_on_exp_overflow() {
        let entries1 = smallvec![make_form_entry(0, (1, 1)),];
        let entries2 = smallvec![make_form_entry(0, (i64::MAX, 1)),];
        let form1 = Form { entries: entries1 };
        let form2 = Form { entries: entries2 };
        assert!(matches!(
            form1.mul(&form2),
            Err(DimensionError::ExponentOverflow)
        ));
    }

    #[test]
    fn test_form_pow() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(1, (-1, 1)),];
        let form = Form { entries };
        let e = Exp::new(-3, 2).unwrap();
        let expected_entries = smallvec![make_form_entry(0, (-3, 4)), make_form_entry(1, (3, 2)),];
        assert_eq!(
            form.pow(e).unwrap(),
            Form {
                entries: expected_entries
            }
        );
    }

    #[test]
    fn test_form_pow_invariance() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        let e = Exp::new(-3, 2).unwrap();
        let e_recipr = Exp::new(2, -3).unwrap();
        assert_eq!(form.pow(e).unwrap().pow(e_recipr).unwrap(), form);
    }

    #[test]
    fn test_form_pow_zero() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        let e = Exp::ZERO;
        assert!(form.pow(e).unwrap().is_empty());
    }

    #[test]
    fn test_empty_form_raised_to_zero_stays_empty() {
        let empty_entries = smallvec![];
        let empty_form = Form {
            entries: empty_entries,
        };
        let e = Exp::ZERO;
        assert!(empty_form.pow(e).unwrap().is_empty());
    }

    #[test]
    fn test_form_pow_err_on_exp_overflow() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        let e = Exp::new(i64::MAX, 1).unwrap();
        assert!(matches!(form.pow(e), Err(DimensionError::ExponentOverflow)));
    }
}
