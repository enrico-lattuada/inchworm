use crate::{AtomId, Exp};

#[cfg(test)]
pub(crate) fn make_form_entry(id: u64, num_den: (i64, i64)) -> (AtomId, Exp) {
    let (num, den) = num_den;
    let exp = Exp::raw(num, den);
    (AtomId::new(id), exp)
}
