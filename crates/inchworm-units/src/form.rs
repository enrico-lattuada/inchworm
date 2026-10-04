//! [`UnitForm`], a reduced product of unit-atom powers.
//!
//! A `UnitForm` backs the stored factors of a [`DeltaUnit`](crate::DeltaUnit):
//! the merge algebra (multiply, power, reciprocal) lives here once, and
//! `DeltaUnit` pairs it with the matching dimension algebra.

use std::{
    cmp::Ordering,
    fmt::{self, Write},
};

use inchworm_dimensions::Exp;
use smallvec::{SmallVec, smallvec};

use crate::{
    UnitError,
    atom::UnitAtom,
    parse::{
        CARET_CHAR, LPAREN_CHAR, MUL_CHAR, PRETTY_MUL_CHAR, RPAREN_CHAR, SLASH_CHAR,
        UNITARY_IDENT_CHAR, digit_to_superscript,
    },
};

const MAX_INLINE_FACTORS: usize = 4;

/// A reduced product of powers over named atoms.
///
/// Invariants:
/// - sorted by [`UnitId`](crate::UnitId) ascending
/// - no zero exponents
/// - no duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitForm {
    entries: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]>,
}

// ---- instantiation ----
impl UnitForm {
    /// Returns an empty form: the dimensionless `1`.
    pub fn empty() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }

    /// Returns a form with a single entry, or an empty form if `exp` is zero.
    pub(crate) fn single(atom: &UnitAtom, exp: Exp) -> Self {
        if exp.is_zero() {
            Self::empty()
        } else {
            Self {
                entries: smallvec![(atom.clone(), exp)],
            }
        }
    }
}

// ---- accessors ----
impl UnitForm {
    /// Returns `true` if `self` has no entries, i.e., it is the dimensionless `1`.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries, sorted by [`UnitId`](crate::UnitId).
    pub(crate) fn entries(&self) -> &[(UnitAtom, Exp)] {
        &self.entries
    }
}

// ---- algebra ----
impl UnitForm {
    /// Merges two forms, combining exponents of shared atoms, pruning any that cancel to zero.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining a shared atom's exponents overflows.
    pub(crate) fn mul(&self, rhs: &Self) -> Result<Self, UnitError> {
        let mut entries = SmallVec::new();
        let (mut i, mut j) = (0, 0);
        while i < self.entries.len() && j < rhs.entries.len() {
            let (id_a, exp_a) = &self.entries[i];
            let (id_b, exp_b) = &rhs.entries[j];
            match id_a.cmp(id_b) {
                Ordering::Less => {
                    entries.push((id_a.clone(), *exp_a));
                    i += 1;
                }
                Ordering::Greater => {
                    entries.push((id_b.clone(), *exp_b));
                    j += 1;
                }
                Ordering::Equal => {
                    let exp_sum = exp_a.checked_add(*exp_b)?;
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
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if multiplying an atom's exponent by `e` overflows.
    pub(crate) fn pow(&self, e: Exp) -> Result<Self, UnitError> {
        if e.is_zero() {
            return Ok(Self::empty());
        }
        let mut entries = SmallVec::new();
        for (atom_data, exp) in self.entries() {
            let exp_times_e = exp.checked_mul(e)?;
            entries.push((atom_data.clone(), exp_times_e));
        }
        Ok(Self { entries })
    }

    /// Returns the reciprocal of `self`: every exponent negated (`m^2·s` → `m^-2·s^-1`).
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if computing the reciprocal of an atom's exponents overflows.
    pub(crate) fn recip(&self) -> Result<Self, UnitError> {
        let mut entries = SmallVec::new();
        for (atom_data, exp) in self.entries() {
            entries.push((atom_data.clone(), exp.checked_neg()?));
        }
        Ok(Self { entries })
    }
}

fn write_factor(f: &mut fmt::Formatter<'_>, symbol: &str, exp: Exp, pretty: bool) -> fmt::Result {
    f.write_str(symbol)?;
    let num = exp.num().unsigned_abs();
    match (exp.is_int(), num, pretty) {
        (true, 1, _) => Ok(()),
        (true, _, true) => num
            .to_string()
            .chars()
            .filter_map(digit_to_superscript)
            .try_for_each(|c| f.write_char(c)),
        (true, _, false) => write!(f, "{CARET_CHAR}{num}"),
        (false, ..) => write!(
            f,
            "{CARET_CHAR}{LPAREN_CHAR}{num}{SLASH_CHAR}{}{RPAREN_CHAR}",
            exp.den()
        ),
    }
}

impl fmt::Display for UnitForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pretty = f.alternate();
        let sep = if pretty { PRETTY_MUL_CHAR } else { MUL_CHAR };
        let mut first = true;
        for (atom, exp) in self.entries().iter().filter(|(_, e)| e.num() > 0) {
            if !first {
                f.write_char(sep)?;
            }
            write_factor(f, &atom.symbol, *exp, pretty)?;
            first = false;
        }
        if first {
            f.write_char(UNITARY_IDENT_CHAR)?;
        }
        for (atom, exp) in self.entries().iter().filter(|(_, e)| e.num() < 0) {
            f.write_char(SLASH_CHAR)?;
            write_factor(f, &atom.symbol, *exp, pretty)?;
        }
        Ok(())
    }
}

impl UnitForm {
    #[cfg(test)]
    pub(crate) fn raw(entries: impl IntoIterator<Item = (UnitAtom, Exp)>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        UnitRegistryId,
        test_utils::{errors_match, make_atom},
    };
    use inchworm_dimensions::{Dimension, DimensionError};

    fn atoms<const N: usize>(symbols: [&str; N]) -> [UnitAtom; N] {
        let registry_id = UnitRegistryId::next();
        symbols.map(|symbol| make_atom(registry_id, symbol, Dimension::dimensionless()))
    }

    mod single {
        use super::*;

        #[test]
        fn nonzero_exp_gives_one_entry() {
            let [atom] = atoms(["x"]);
            let form = UnitForm::single(&atom, Exp::int(2));
            assert_eq!(form.entries(), [(atom, Exp::int(2))]);
        }

        #[test]
        fn zero_exp_gives_empty() {
            let [atom] = atoms(["x"]);
            let form = UnitForm::single(&atom, Exp::ZERO);
            assert!(form.entries.is_empty());
        }
    }

    mod mul {
        use super::*;

        #[test]
        fn empty_is_identity() {
            let [atom] = atoms(["x"]);
            let form = UnitForm {
                entries: smallvec![(atom, Exp::ONE)],
            };
            assert_eq!(form.mul(&UnitForm::empty()).unwrap(), form);
            assert_eq!(UnitForm::empty().mul(&form).unwrap(), form);
        }

        #[test]
        fn disjoint_forms_merge_sorted_by_id() {
            let [a, b, c] = atoms(["a", "b", "c"]);
            let form_1 = UnitForm::raw([(a.clone(), Exp::ONE), (c.clone(), Exp::ONE)]);
            let form_2 = UnitForm::raw([(b.clone(), Exp::ONE)]);
            let expected = UnitForm::raw([
                (a.clone(), Exp::ONE),
                (b.clone(), Exp::ONE),
                (c.clone(), Exp::ONE),
            ]);
            assert_eq!(form_1.mul(&form_2).unwrap(), expected);
        }

        #[test]
        fn shared_atom_exponents_add() {
            let [a] = atoms(["a"]);
            let form_1 = UnitForm::raw([(a.clone(), Exp::ONE)]);
            let form_2 = UnitForm::raw([(a.clone(), Exp::int(2))]);
            let expected = UnitForm::raw([(a.clone(), Exp::int(3))]);
            assert_eq!(form_1.mul(&form_2).unwrap(), expected);
        }

        #[test]
        fn cancelling_atom_is_removed() {
            let [a] = atoms(["a"]);
            let form_1 = UnitForm::raw([(a.clone(), Exp::ONE)]);
            let form_2 = UnitForm::raw([(a.clone(), Exp::int(-1))]);
            let expected = UnitForm::empty();
            assert_eq!(form_1.mul(&form_2).unwrap(), expected);
        }

        #[test]
        fn err_on_exp_overflow() {
            let [a] = atoms(["a"]);
            let form_1 = UnitForm::raw([(a.clone(), Exp::ONE)]);
            let form_2 = UnitForm::raw([(a.clone(), Exp::int(i64::MAX))]);
            let actual = form_1.mul(&form_2).unwrap_err();
            let expected = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&actual, &expected));
        }
    }

    mod pow {
        use super::*;

        #[test]
        fn multiplies_every_exponent() {
            let [a, b] = atoms(["a", "b"]);
            let form = UnitForm::raw([(a.clone(), Exp::ONE), (b.clone(), Exp::int(-1))]);
            let e = Exp::int(2);
            let expected = UnitForm::raw([(a.clone(), Exp::int(2)), (b.clone(), Exp::int(-2))]);
            assert_eq!(form.pow(e).unwrap(), expected);
        }

        #[test]
        fn zero_gives_empty() {
            let [a] = atoms(["a"]);
            let form = UnitForm::raw([(a, Exp::ONE)]);
            let expected = UnitForm::empty();
            assert_eq!(form.pow(Exp::ZERO).unwrap(), expected);
        }

        #[test]
        fn err_on_exp_overflow() {
            let [a] = atoms(["a"]);
            let form = UnitForm::raw([(a, Exp::int(i64::MAX))]);
            let actual = form.pow(Exp::int(2)).unwrap_err();
            let expected = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&actual, &expected));
        }
    }

    mod recip {
        use super::*;

        #[test]
        fn negates_every_exponent() {
            let [a, b] = atoms(["a", "b"]);
            let form = UnitForm::raw([(a.clone(), Exp::int(2)), (b.clone(), Exp::int(-1))]);
            let expected = UnitForm::raw([(a.clone(), Exp::int(-2)), (b.clone(), Exp::ONE)]);
            assert_eq!(form.recip().unwrap(), expected);
        }

        #[test]
        fn recip_of_recip_is_identity() {
            let [a, b] = atoms(["a", "b"]);
            let form = UnitForm::raw([(a, Exp::int(2)), (b, Exp::int(-1))]);
            assert_eq!(form.recip().unwrap().recip().unwrap(), form);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn empty_is_one() {
            let form = UnitForm::empty();
            assert_eq!(form.to_string(), "1");
        }

        #[test]
        fn omits_unit_exp() {
            let [atom] = atoms(["m"]);
            let form = UnitForm::raw([(atom, Exp::ONE)]);
            assert_eq!(form.to_string(), "m");
        }

        #[test]
        fn integer_exp() {
            let [m_atom, s_atom] = atoms(["m", "s"]);
            let m_form = UnitForm::raw([(m_atom, Exp::int(2))]);
            let s_form = UnitForm::raw([(s_atom, Exp::int(-1))]);
            assert_eq!(m_form.to_string(), "m^2");
            assert_eq!(s_form.to_string(), "1/s");
        }

        #[test]
        fn fractional_exp() {
            let [atom] = atoms(["m"]);
            let form = UnitForm::raw([(atom, Exp::new(1, 2).unwrap())]);
            assert_eq!(form.to_string(), "m^(1/2)");
        }

        #[test]
        fn joins_factors_with_star() {
            let [m_atom, s_atom] = atoms(["m", "s"]);
            let form = UnitForm::raw([(m_atom, Exp::ONE), (s_atom, Exp::ONE)]);
            assert_eq!(form.to_string(), "m*s");
        }

        #[test]
        fn alternate_uses_dot_and_superscripts() {
            let [m_atom, s_atom] = atoms(["m", "s"]);
            let form = UnitForm::raw([(m_atom.clone(), Exp::ONE), (s_atom.clone(), Exp::ONE)]);
            assert_eq!(format!("{form:#}"), "m·s");
            let form = UnitForm::raw([
                (m_atom.clone(), Exp::int(2)),
                (s_atom.clone(), Exp::int(-3)),
            ]);
            assert_eq!(format!("{form:#}"), "m²/s³");
        }

        #[test]
        fn alternate_keeps_fraction_ascii() {
            let [atom] = atoms(["m"]);
            let form = UnitForm::raw([(atom, Exp::new(1, 2).unwrap())]);
            assert_eq!(format!("{form:#}"), "m^(1/2)");
        }
    }
}
