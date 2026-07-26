use std::cmp::Ordering;

use smallvec::{SmallVec, smallvec};

use crate::atom::Atom;
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
    entries: SmallVec<[(Atom, Exp); MAX_INLINE_FACTORS]>,
}

impl Form {
    /// Returns an empty form.
    pub fn empty() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }

    /// Returns a form with a single entry, or an empty form if `exp` is zero.
    pub(crate) fn single(atom: &Atom, exp: Exp) -> Self {
        if exp.is_zero() {
            return Self::empty();
        }
        Self {
            entries: smallvec![(atom.clone(), exp)],
        }
    }

    /// Returns `true` if `self` has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn entries(&self) -> &[(Atom, Exp)] {
        &self.entries
    }
}

// ---- algebra ----
impl Form {
    /// Merges two forms, combining exponents of shared atoms, pruning any that cancel to zero.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    pub(crate) fn mul(&self, rhs: &Self) -> Result<Self, DimensionError> {
        let mut entries = SmallVec::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.entries.len() && j < rhs.entries.len() {
            let (id_a, exp_a) = &self.entries[i];
            let (id_b, exp_b) = &rhs.entries[j];
            match id_a.cmp(id_b) {
                Ordering::Less => {
                    entries.push((id_a.clone(), exp_a.clone()));
                    i += 1;
                }
                Ordering::Greater => {
                    entries.push((id_b.clone(), exp_b.clone()));
                    j += 1;
                }
                Ordering::Equal => {
                    let exp_sum = exp_a.checked_add(exp_b.clone())?;
                    if !exp_sum.is_zero() {
                        entries.push((id_a.clone(), exp_sum));
                    }
                    (i, j) = (i + 1, j + 1);
                }
            }
        }
        entries.extend(self.entries[i..].iter().cloned());
        entries.extend(rhs.entries[j..].iter().cloned());
        Ok(Self { entries })
    }

    /// Raises `self` to the power of `e`, pruning any that cancels to zero.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if multiplying an atom's exponent by `e` overflows.
    pub(crate) fn pow(&self, e: Exp) -> Result<Self, DimensionError> {
        let mut entries = SmallVec::new();
        if !e.is_zero() {
            for (atom_data, exp) in self.entries.iter() {
                let exp_times_e = exp.checked_mul(e)?;
                entries.push((atom_data.clone(), exp_times_e));
            }
        }
        Ok(Self { entries })
    }

    /// Computes the reciprocal of `self` by raising it to the power of `-1`.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if computing the reciprocal of an atom's exponents overflows.
    pub(crate) fn recip(&self) -> Result<Self, DimensionError> {
        let mut entries = SmallVec::new();
        for (atom_data, entry) in self.entries.iter() {
            entries.push((atom_data.clone(), entry.checked_neg()?));
        }
        Ok(Self { entries })
    }
}

// ---- test utils ----
impl Form {
    #[cfg(test)]
    pub(crate) fn raw(entries: impl IntoIterator<Item = (Atom, Exp)>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

/// Base signature, a [`Form`] containing base atoms only.
///
/// A [`Signature`] answers "are these the same physical quantity, ignoring names?"
/// while a canonical [`Form`] additionally keeps named-dimensionless atoms as irreducible factors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub(crate) Form);

impl Signature {
    #[cfg(test)]
    pub(crate) fn raw(entries: impl IntoIterator<Item = (Atom, Exp)>) -> Self {
        Self(Form::raw(entries))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::make_form_entry;

    use super::*;
    use smallvec::smallvec;

    #[test]
    fn form_empty() {
        let empty_form = Form::empty();
        assert_eq!(
            empty_form,
            Form {
                entries: SmallVec::new()
            }
        );
    }

    #[test]
    fn form_is_empty() {
        let empty_form = Form {
            entries: SmallVec::new(),
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
    fn mul_empty_form() {
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
    fn blocked_forms_mul() {
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
    fn interleaved_forms_mul() {
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
    fn fully_overlapping_forms_mul() {
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
    fn zero_exp_result_forms_mul() {
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
    fn generic_forms_mul() {
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
    fn forms_mul_err_on_exp_overflow() {
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
    fn form_pow() {
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
    fn form_pow_invariance() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        let e = Exp::new(-3, 2).unwrap();
        let e_recipr = Exp::new(2, -3).unwrap();
        assert_eq!(form.pow(e).unwrap().pow(e_recipr).unwrap(), form);
    }

    #[test]
    fn form_pow_zero() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        let e = Exp::ZERO;
        assert!(form.pow(e).unwrap().is_empty());
    }

    #[test]
    fn empty_form_raised_to_zero_stays_empty() {
        let empty_entries = smallvec![];
        let empty_form = Form {
            entries: empty_entries,
        };
        let e = Exp::ZERO;
        assert!(empty_form.pow(e).unwrap().is_empty());
    }

    #[test]
    fn form_pow_err_on_exp_overflow() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        let e = Exp::new(i64::MAX, 1).unwrap();
        assert!(matches!(form.pow(e), Err(DimensionError::ExponentOverflow)));
    }

    #[test]
    fn recip_of_recip_is_identity() {
        let entries = smallvec![make_form_entry(0, (1, 2)), make_form_entry(2, (5, 4)),];
        let form = Form { entries };
        assert_eq!(form, form.recip().unwrap().recip().unwrap());
    }
}
